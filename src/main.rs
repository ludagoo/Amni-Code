use axum::{extract::State, response::{Html, Json}, routing::{get, post}, Router};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
#[derive(Clone)]
struct App {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    config: Arc<Mutex<Config>>,
    cwd: Arc<Mutex<PathBuf>>,
}
#[derive(Clone, Default)]
struct Session { messages: Vec<serde_json::Value> }
#[derive(Clone, Serialize, Deserialize)]
struct Config {
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
    auto_approve: bool,
    working_dir: String,
    model_dir: String,
}
impl Default for Config {
    fn default() -> Self {
        let key = std::env::var("XAI_API_KEY").or_else(|_| std::env::var("OPENAI_API_KEY"))
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY")).unwrap_or_default();
        let provider = if !std::env::var("XAI_API_KEY").unwrap_or_default().is_empty() { "xai" }
            else if !std::env::var("OPENAI_API_KEY").unwrap_or_default().is_empty() { "openai" }
            else if !std::env::var("ANTHROPIC_API_KEY").unwrap_or_default().is_empty() { "anthropic" }
            else { "xai" };
        let (model, base_url) = match provider {
            "openai" => ("gpt-4o".to_string(), "https://api.openai.com".to_string()),
            "anthropic" => ("claude-sonnet-4-20250514".to_string(), "https://api.anthropic.com".to_string()),
            "ollama" => (String::new(), "http://localhost:11434".to_string()),
            "local" => (String::new(), "http://localhost:11434".to_string()),
            _ => ("grok-4-1-fast-reasoning".to_string(), "https://api.x.ai".to_string()),
        };
        Self { provider: provider.into(), model, api_key: key, base_url, auto_approve: false,
            working_dir: std::env::current_dir().unwrap_or_default().to_string_lossy().into(),
            model_dir: String::new() }
    }
}
const TOOLS_JSON: &str = r#"[
  {"type":"function","function":{"name":"read_file","description":"Read a file's contents","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}},
  {"type":"function","function":{"name":"write_file","description":"Write content to a file (creates or overwrites)","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}}},
  {"type":"function","function":{"name":"edit_file","description":"Replace a specific string in a file","parameters":{"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"}},"required":["path","old_string","new_string"]}}},
  {"type":"function","function":{"name":"run_command","description":"Run a shell command","parameters":{"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}}},
  {"type":"function","function":{"name":"list_directory","description":"List files in a directory","parameters":{"type":"object","properties":{"path":{"type":"string"}}}}},
  {"type":"function","function":{"name":"search_files","description":"Search for text across files","parameters":{"type":"object","properties":{"query":{"type":"string"},"path":{"type":"string"}},"required":["query"]}}}
]"#;
const SYSTEM_PROMPT: &str = "You are Amni-Code, an expert AI coding agent. Your working directory is: {CWD}\n\nCRITICAL RULES:\n1. ALWAYS use your tools proactively. NEVER ask the user for information you can discover with tools. If the user asks about code or a project, IMMEDIATELY call list_directory and read_file to explore — do NOT ask the user to provide code.\n2. When asked about a codebase: start by calling list_directory on the working directory, then read key files (README, main entry points, config files) to build understanding before responding.\n3. When asked to create or modify code: read the relevant files first, then make changes with write_file or edit_file.\n4. When debugging: read the code, understand the issue, make targeted fixes, then run tests.\n5. After making changes: run builds or tests to verify.\n6. Execute all steps in sequence — don't describe what you would do, just do it.\n7. Be concise but thorough.\n8. Use the working directory as base for relative paths.\n\nAvailable tools: read_file, write_file, edit_file, run_command, list_directory, search_files";
#[derive(Deserialize)]
struct ChatReq { message: String, session_id: Option<String> }
#[derive(Serialize)]
struct ChatRes { session_id: String, message: String, tool_calls: Vec<ToolCallResult>, done: bool }
#[derive(Serialize, Clone)]
struct ToolCallResult { tool: String, input: serde_json::Value, output: String, status: String }
#[derive(Deserialize)]
struct ConfigReq { provider: Option<String>, model: Option<String>, api_key: Option<String>, base_url: Option<String>, auto_approve: Option<bool>, working_dir: Option<String>, model_dir: Option<String> }
#[derive(Serialize)]
struct ModelsRes { models: Vec<String> }
#[derive(Serialize)]
struct DirsRes { dirs: Vec<String> }
#[derive(Deserialize)]
struct DirQuery { path: Option<String> }
async fn exec_tool(name: &str, args: &serde_json::Value, cwd: &PathBuf) -> (String, String) {
    let resolve = |p: &str| -> PathBuf {
        if PathBuf::from(p).is_absolute() { PathBuf::from(p) } else { cwd.join(p) }
    };
    match name {
        "read_file" => {
            let full = resolve(args["path"].as_str().unwrap_or(""));
            match tokio::fs::read_to_string(&full).await {
                Ok(c) => (c, "success".into()),
                Err(e) => (format!("Error: {}", e), "error".into()),
            }
        }
        "write_file" => {
            let full = resolve(args["path"].as_str().unwrap_or(""));
            let content = args["content"].as_str().unwrap_or("");
            if let Some(p) = full.parent() { let _ = tokio::fs::create_dir_all(p).await; }
            match tokio::fs::write(&full, content).await {
                Ok(_) => (format!("Written {} bytes to {}", content.len(), full.display()), "success".into()),
                Err(e) => (format!("Error: {}", e), "error".into()),
            }
        }
        "edit_file" => {
            let full = resolve(args["path"].as_str().unwrap_or(""));
            let old = args["old_string"].as_str().unwrap_or("");
            let new = args["new_string"].as_str().unwrap_or("");
            match tokio::fs::read_to_string(&full).await {
                Ok(c) if c.contains(old) => {
                    match tokio::fs::write(&full, c.replacen(old, new, 1)).await {
                        Ok(_) => (format!("Edited {}", full.display()), "success".into()),
                        Err(e) => (format!("Error: {}", e), "error".into()),
                    }
                }
                Ok(_) => ("String not found in file".into(), "error".into()),
                Err(e) => (format!("Error: {}", e), "error".into()),
            }
        }
        "run_command" => {
            let cmd = args["command"].as_str().unwrap_or("");
            let shell = if cfg!(windows) { ("cmd", vec!["/C", cmd]) } else { ("sh", vec!["-c", cmd]) };
            match tokio::process::Command::new(shell.0).args(&shell.1).current_dir(cwd)
                .stdout(Stdio::piped()).stderr(Stdio::piped()).output().await {
                Ok(o) => {
                    let out = format!("{}{}", String::from_utf8_lossy(&o.stdout),
                        if o.stderr.is_empty() { "".into() } else { format!("\nstderr: {}", String::from_utf8_lossy(&o.stderr)) });
                    let trimmed = if out.len() > 10000 { format!("{}...(truncated)", &out[..10000]) } else { out };
                    (trimmed, if o.status.success() { "success" } else { "error" }.into())
                }
                Err(e) => (format!("Failed: {}", e), "error".into()),
            }
        }
        "list_directory" => {
            let full = resolve(args.get("path").and_then(|p| p.as_str()).unwrap_or("."));
            match tokio::fs::read_dir(&full).await {
                Ok(mut rd) => {
                    let mut items = Vec::new();
                    while let Ok(Some(e)) = rd.next_entry().await {
                        let n = e.file_name().to_string_lossy().to_string();
                        items.push(if e.file_type().await.map(|t| t.is_dir()).unwrap_or(false) { format!("{}/", n) } else { n });
                    }
                    items.sort();
                    (items.join("\n"), "success".into())
                }
                Err(e) => (format!("Error: {}", e), "error".into()),
            }
        }
        "search_files" => {
            let query = args["query"].as_str().unwrap_or("");
            let full = resolve(args.get("path").and_then(|p| p.as_str()).unwrap_or("."));
            let cmd = if cfg!(windows) {
                format!("findstr /s /n /i \"{}\" \"{}\\*\"", query, full.display())
            } else {
                format!("grep -rn -i \"{}\" \"{}\"", query, full.display())
            };
            let shell = if cfg!(windows) { ("cmd", vec!["/C".to_string(), cmd]) } else { ("sh", vec!["-c".to_string(), cmd]) };
            match tokio::process::Command::new(shell.0).args(&shell.1).stdout(Stdio::piped()).stderr(Stdio::piped()).output().await {
                Ok(o) => {
                    let r = String::from_utf8_lossy(&o.stdout).to_string();
                    (if r.is_empty() { "No matches".into() } else if r.len() > 10000 { format!("{}...(truncated)", &r[..10000]) } else { r }, "success".into())
                }
                Err(e) => (format!("Error: {}", e), "error".into()),
            }
        }
        _ => (format!("Unknown tool: {}", name), "error".into()),
    }
}
struct ToolCall { id: String, name: String, args: serde_json::Value }
async fn llm_request(config: &Config, messages: &[serde_json::Value], use_tools: bool) -> Result<(serde_json::Value, Vec<ToolCall>), (String, bool)> {
    let (url, key_header) = match config.provider.as_str() {
        "ollama" => (format!("{}/v1/chat/completions", config.base_url), None),
        "local" => (format!("{}/v1/chat/completions", config.base_url), None),
        "openai" => (format!("{}/v1/chat/completions", config.base_url.trim_end_matches('/')),
            Some(("Authorization", format!("Bearer {}", config.api_key)))),
        "anthropic" => ("https://api.anthropic.com/v1/messages".into(),
            Some(("x-api-key", config.api_key.clone()))),
        "xai" => ("https://api.x.ai/v1/chat/completions".into(),
            Some(("Authorization", format!("Bearer {}", config.api_key)))),
        other => return Err((format!("Unknown provider: {}", other), false)),
    };
    let mut body = serde_json::json!({"model": config.model, "messages": messages, "max_tokens": 4096});
    if use_tools {
        let tools: serde_json::Value = serde_json::from_str(TOOLS_JSON).unwrap();
        body["tools"] = tools;
        body["tool_choice"] = serde_json::json!("auto");
    }
    let client = reqwest::Client::new();
    let mut req = client.post(&url).header("Content-Type", "application/json");
    if let Some((k, v)) = &key_header { req = req.header(k.to_owned(), v.to_owned()); }
    if config.provider == "anthropic" { req = req.header("anthropic-version", "2023-06-01"); }
    let resp = req.json(&body).send().await.map_err(|e| (format!("Request failed: {}", e), false))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| (format!("Read failed: {}", e), false))?;
    if !status.is_success() {
        let is_tool_err = text.contains("does not support tools") || text.contains("tool_use") || text.contains("tools is not supported");
        return Err((format!("API error {}: {}", status, &text[..text.len().min(500)]), is_tool_err));
    }
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| (format!("Parse error: {}", e), false))?;
    let raw_msg = json["choices"][0]["message"].clone();
    let mut tool_calls = Vec::new();
    if use_tools {
        if let Some(calls) = raw_msg.get("tool_calls").and_then(|t| t.as_array()) {
            for call in calls {
                let id = call["id"].as_str().unwrap_or("").to_string();
                let name = call["function"]["name"].as_str().unwrap_or("").to_string();
                let args: serde_json::Value = serde_json::from_str(call["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or_default();
                tool_calls.push(ToolCall { id, name, args });
            }
        }
    }
    Ok((raw_msg, tool_calls))
}
async fn llm_call(config: &Config, messages: &[serde_json::Value]) -> Result<(serde_json::Value, Vec<ToolCall>), String> {
    match llm_request(config, messages, true).await {
        Ok(r) => Ok(r),
        Err((_msg, true)) => llm_request(config, messages, false).await.map_err(|(e, _)| e),
        Err((msg, false)) => Err(msg),
    }
}
async fn agent_loop(app: &App, sid: &str, user_msg: &str) -> ChatRes {
    let config = app.config.lock().await.clone();
    let cwd_path = app.cwd.lock().await.clone();
    {
        let mut sessions = app.sessions.lock().await;
        let session = sessions.entry(sid.to_string()).or_default();
        if session.messages.is_empty() {
            let sys = SYSTEM_PROMPT.replace("{CWD}", &cwd_path.display().to_string());
            session.messages.push(serde_json::json!({"role": "system", "content": sys}));
        }
        session.messages.push(serde_json::json!({"role": "user", "content": user_msg}));
    }
    let mut all_tools = Vec::new();
    for _ in 0..15 {
        let messages = app.sessions.lock().await.get(sid).map(|s| s.messages.clone()).unwrap_or_default();
        match llm_call(&config, &messages).await {
            Ok((raw_msg, tool_calls)) => {
                if tool_calls.is_empty() {
                    let content = raw_msg["content"].as_str().unwrap_or("").to_string();
                    app.sessions.lock().await.entry(sid.to_string()).or_default().messages.push(raw_msg);
                    return ChatRes { session_id: sid.into(), message: content, tool_calls: all_tools, done: true };
                }
                app.sessions.lock().await.entry(sid.to_string()).or_default().messages.push(raw_msg);
                for tc in &tool_calls {
                    let (output, status) = exec_tool(&tc.name, &tc.args, &cwd_path).await;
                    all_tools.push(ToolCallResult { tool: tc.name.clone(), input: tc.args.clone(), output: output.clone(), status });
                    app.sessions.lock().await.entry(sid.to_string()).or_default().messages.push(
                        serde_json::json!({"role": "tool", "tool_call_id": tc.id, "content": output})
                    );
                }
            }
            Err(e) => return ChatRes { session_id: sid.into(), message: format!("Error: {}", e), tool_calls: all_tools, done: true },
        }
    }
    ChatRes { session_id: sid.into(), message: "Reached max iterations — try continuing.".into(), tool_calls: all_tools, done: true }
}
async fn handle_chat(State(app): State<App>, Json(req): Json<ChatReq>) -> Json<ChatRes> {
    let sid = req.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    Json(agent_loop(&app, &sid, &req.message).await)
}
async fn handle_config_get(State(app): State<App>) -> Json<Config> { Json(app.config.lock().await.clone()) }
async fn handle_config_set(State(app): State<App>, Json(req): Json<ConfigReq>) -> Json<Config> {
    let mut cfg = app.config.lock().await;
    if let Some(v) = req.provider { cfg.provider = v; }
    if let Some(v) = req.model { cfg.model = v; }
    if let Some(v) = req.api_key { cfg.api_key = v; }
    if let Some(v) = req.base_url { cfg.base_url = v; }
    if let Some(v) = req.auto_approve { cfg.auto_approve = v; }
    if let Some(v) = req.working_dir { cfg.working_dir = v.clone(); *app.cwd.lock().await = PathBuf::from(v); }
    if let Some(v) = req.model_dir { cfg.model_dir = v; }
    Json(cfg.clone())
}
async fn handle_models(State(app): State<App>) -> Json<ModelsRes> {
    let cfg = app.config.lock().await.clone();
    let mut models = Vec::new();
    if cfg.provider == "ollama" || cfg.provider == "local" {
        if let Ok(resp) = reqwest::get(format!("{}/api/tags", cfg.base_url)).await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = json["models"].as_array() {
                    for m in arr { if let Some(n) = m["name"].as_str() { models.push(n.to_string()); } }
                }
            }
        }
    } else if cfg.provider == "openai" {
        models.extend(["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-mini", "o3-mini"].iter().map(|s| s.to_string()));
    } else if cfg.provider == "anthropic" {
        models.extend(["claude-sonnet-4-20250514", "claude-opus-4-20250514", "claude-3-5-haiku-20241022"].iter().map(|s| s.to_string()));
    } else if cfg.provider == "xai" {
        models.extend(["grok-4-1-fast-reasoning", "grok-4-1-fast-non-reasoning", "grok-4.20-0309-reasoning", "grok-4.20-0309-non-reasoning", "grok-4.20-multi-agent-0309"].iter().map(|s| s.to_string()));
    }
    Json(ModelsRes { models })
}
async fn handle_dirs(axum::extract::Query(q): axum::extract::Query<DirQuery>, State(app): State<App>) -> Json<DirsRes> {
    let base = match q.path {
        Some(p) => p,
        None => app.config.lock().await.working_dir.clone(),
    };
    let mut dirs = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&base).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let n = e.file_name().to_string_lossy().to_string();
                if !n.starts_with('.') { dirs.push(n); }
            }
        }
    }
    dirs.sort();
    Json(DirsRes { dirs })
}
async fn handle_health() -> &'static str { "ok" }
async fn serve_ui() -> Html<&'static str> { Html(include_str!("../static/index.html")) }
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cwd = std::env::current_dir().unwrap_or_default();
    let home_env = dirs::home_dir().map(|h| h.join(".amni").join(".env")).unwrap_or_default();
    for env_path in [home_env, cwd.join(".env")] {
        if env_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&env_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') { continue; }
                    if let Some((k, v)) = line.split_once('=') {
                        std::env::set_var(k.trim(), v.trim());
                    }
                }
            }
        }
    }
    println!("\n  Amni-Code v1.1.0 — AI Coding Agent");
    println!("  Working dir: {}", cwd.display());
    let app = App {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        config: Arc::new(Mutex::new(Config::default())),
        cwd: Arc::new(Mutex::new(cwd)),
    };
    let router = Router::new()
        .route("/", get(serve_ui))
        .route("/api/chat", post(handle_chat))
        .route("/api/config", get(handle_config_get).post(handle_config_set))
        .route("/api/models", get(handle_models))
        .route("/api/dirs", get(handle_dirs))
        .route("/health", get(handle_health))
        .layer(CorsLayer::permissive())
        .with_state(app);
    let use_browser = std::env::args().any(|a| a == "--browser");
    if use_browser {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        println!("  Server: http://localhost:3000");
        println!("  Opening browser...\n");
        let _ = open::that("http://localhost:3000");
        axum::serve(listener, router).await?;
    } else {
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            println!("  Server: http://localhost:3000\n");
            axum::serve(listener, router).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        use tao::event::{Event, WindowEvent};
        use tao::event_loop::{ControlFlow, EventLoop};
        use tao::window::WindowBuilder;
        use wry::WebViewBuilder;
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Amni-Code — AI Coding Agent")
            .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
            .build(&event_loop)
            .unwrap();
        let _webview = WebViewBuilder::new()
            .with_url("http://localhost:3000")
            .build(&window)
            .unwrap();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
    Ok(())
}
