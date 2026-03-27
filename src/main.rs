use axum::{
    extract::{Query, State},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::{Path, PathBuf}, process::Stdio, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
#[derive(Clone, Default, Serialize)]
struct DownloadProgress {
    repo: String,
    file: String,
    downloaded: u64,
    total: u64,
    done: bool,
    error: String,
}
#[derive(Clone)]
struct App {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    config: Arc<Mutex<Config>>,
    cwd: Arc<Mutex<PathBuf>>,
    dl_progress: Arc<Mutex<DownloadProgress>>,
}
#[derive(Clone, Default)]
struct Session {
    messages: Vec<serde_json::Value>,
}
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
        let xai_key = [
            "XAI_API_KEY",
            "GROK_key",
            "xAI_key",
            "GROK_API_KEY",
            "XAI_KEY",
        ]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default();
        let openai_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let anthropic_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
        let (provider, key) = if !xai_key.is_empty() {
            ("xai", xai_key)
        } else if !openai_key.is_empty() {
            ("openai", openai_key)
        } else if !anthropic_key.is_empty() {
            ("anthropic", anthropic_key)
        } else {
            ("xai", String::new())
        };
        let (model, base_url) = match provider {
            "openai" => ("gpt-4o".to_string(), "https://api.openai.com".to_string()),
            "anthropic" => (
                "claude-sonnet-4-20250514".to_string(),
                "https://api.anthropic.com".to_string(),
            ),
            "ollama" => (String::new(), "http://localhost:11434".to_string()),
            "local" => (String::new(), "http://localhost:11434".to_string()),
            _ => (
                "grok-4-1-fast-reasoning".to_string(),
                "https://api.x.ai".to_string(),
            ),
        };
        let model_dir = std::env::current_dir()
            .unwrap_or_default()
            .parent()
            .unwrap_or(&std::env::current_dir().unwrap_or_default())
            .join("Amni-Ai")
            .join("models")
            .to_string_lossy()
            .to_string();
        let model_dir = if std::fs::metadata(&model_dir).is_ok() {
            model_dir
        } else {
            String::new()
        };
        Self {
            provider: provider.into(),
            model,
            api_key: key,
            base_url,
            auto_approve: false,
            working_dir: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            model_dir,
        }
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
struct ChatReq {
    message: String,
    session_id: Option<String>,
}
#[derive(Serialize)]
struct ChatRes {
    session_id: String,
    message: String,
    tool_calls: Vec<ToolCallResult>,
    done: bool,
}
#[derive(Serialize, Clone)]
struct ToolCallResult {
    tool: String,
    input: serde_json::Value,
    output: String,
    status: String,
}
#[derive(Deserialize)]
struct ConfigReq {
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    auto_approve: Option<bool>,
    working_dir: Option<String>,
    model_dir: Option<String>,
}
#[derive(Serialize)]
struct ModelsRes {
    models: Vec<String>,
}
#[derive(Serialize)]
struct DirsRes {
    dirs: Vec<String>,
}
#[derive(Deserialize)]
struct DirQuery {
    path: Option<String>,
}
async fn exec_tool(name: &str, args: &serde_json::Value, cwd: &PathBuf) -> (String, String) {
    let resolve = |p: &str| -> PathBuf {
        if PathBuf::from(p).is_absolute() {
            PathBuf::from(p)
        } else {
            cwd.join(p)
        }
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
            if let Some(p) = full.parent() {
                let _ = tokio::fs::create_dir_all(p).await;
            }
            match tokio::fs::write(&full, content).await {
                Ok(_) => (
                    format!("Written {} bytes to {}", content.len(), full.display()),
                    "success".into(),
                ),
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
            let shell = if cfg!(windows) {
                ("cmd", vec!["/C", cmd])
            } else {
                ("sh", vec!["-c", cmd])
            };
            match tokio::process::Command::new(shell.0)
                .args(&shell.1)
                .current_dir(cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
            {
                Ok(o) => {
                    let out = format!(
                        "{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        if o.stderr.is_empty() {
                            "".into()
                        } else {
                            format!("\nstderr: {}", String::from_utf8_lossy(&o.stderr))
                        }
                    );
                    let trimmed = if out.len() > 10000 {
                        format!("{}...(truncated)", &out[..10000])
                    } else {
                        out
                    };
                    (
                        trimmed,
                        if o.status.success() {
                            "success"
                        } else {
                            "error"
                        }
                        .into(),
                    )
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
                        items.push(
                            if e.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                                format!("{}/", n)
                            } else {
                                n
                            },
                        );
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
            let shell = if cfg!(windows) {
                ("cmd", vec!["/C".to_string(), cmd])
            } else {
                ("sh", vec!["-c".to_string(), cmd])
            };
            match tokio::process::Command::new(shell.0)
                .args(&shell.1)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
            {
                Ok(o) => {
                    let r = String::from_utf8_lossy(&o.stdout).to_string();
                    (
                        if r.is_empty() {
                            "No matches".into()
                        } else if r.len() > 10000 {
                            format!("{}...(truncated)", &r[..10000])
                        } else {
                            r
                        },
                        "success".into(),
                    )
                }
                Err(e) => (format!("Error: {}", e), "error".into()),
            }
        }
        _ => (format!("Unknown tool: {}", name), "error".into()),
    }
}
struct ToolCall {
    id: String,
    name: String,
    args: serde_json::Value,
}
async fn llm_request(
    config: &Config,
    messages: &[serde_json::Value],
    use_tools: bool,
) -> Result<(serde_json::Value, Vec<ToolCall>), (String, bool)> {
    let (url, key_header) = match config.provider.as_str() {
        "ollama" => (format!("{}/v1/chat/completions", config.base_url), None),
        "local" => (format!("{}/v1/chat/completions", config.base_url), None),
        "openai" => (
            format!(
                "{}/v1/chat/completions",
                config.base_url.trim_end_matches('/')
            ),
            Some(("Authorization", format!("Bearer {}", config.api_key))),
        ),
        "anthropic" => (
            "https://api.anthropic.com/v1/messages".into(),
            Some(("x-api-key", config.api_key.clone())),
        ),
        "xai" => (
            "https://api.x.ai/v1/chat/completions".into(),
            Some(("Authorization", format!("Bearer {}", config.api_key))),
        ),
        other => return Err((format!("Unknown provider: {}", other), false)),
    };
    let mut body =
        serde_json::json!({"model": config.model, "messages": messages, "max_tokens": 4096});
    if use_tools {
        let tools: serde_json::Value = serde_json::from_str(TOOLS_JSON).unwrap();
        body["tools"] = tools;
        body["tool_choice"] = serde_json::json!("auto");
    }
    let client = reqwest::Client::new();
    let mut req = client.post(&url).header("Content-Type", "application/json");
    if let Some((k, v)) = &key_header {
        req = req.header(k.to_owned(), v.to_owned());
    }
    if config.provider == "anthropic" {
        req = req.header("anthropic-version", "2023-06-01");
    }
    let resp = req
        .json(&body)
        .send()
        .await
        .map_err(|e| (format!("Request failed: {}", e), false))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| (format!("Read failed: {}", e), false))?;
    if !status.is_success() {
        let is_tool_err = text.contains("does not support tools")
            || text.contains("tool_use")
            || text.contains("tools is not supported");
        return Err((
            format!("API error {}: {}", status, &text[..text.len().min(500)]),
            is_tool_err,
        ));
    }
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| (format!("Parse error: {}", e), false))?;
    let raw_msg = json["choices"][0]["message"].clone();
    let mut tool_calls = Vec::new();
    if use_tools {
        if let Some(calls) = raw_msg.get("tool_calls").and_then(|t| t.as_array()) {
            for call in calls {
                let id = call["id"].as_str().unwrap_or("").to_string();
                let name = call["function"]["name"].as_str().unwrap_or("").to_string();
                let args: serde_json::Value =
                    serde_json::from_str(call["function"]["arguments"].as_str().unwrap_or("{}"))
                        .unwrap_or_default();
                tool_calls.push(ToolCall { id, name, args });
            }
        }
    }
    Ok((raw_msg, tool_calls))
}
async fn llm_call(
    config: &Config,
    messages: &[serde_json::Value],
) -> Result<(serde_json::Value, Vec<ToolCall>), String> {
    match llm_request(config, messages, true).await {
        Ok(r) => Ok(r),
        Err((_msg, true)) => llm_request(config, messages, false)
            .await
            .map_err(|(e, _)| e),
        Err((msg, false)) => Err(msg),
    }
}
async fn agent_loop(app: &App, sid: &str, user_msg: &str) -> ChatRes {
    let config = app.config.lock().await.clone();
    ensure_model_loaded(&config).await;
    let cwd_path = app.cwd.lock().await.clone();
    {
        let mut sessions = app.sessions.lock().await;
        let session = sessions.entry(sid.to_string()).or_default();
        if session.messages.is_empty() {
            let sys = SYSTEM_PROMPT.replace("{CWD}", &cwd_path.display().to_string());
            session
                .messages
                .push(serde_json::json!({"role": "system", "content": sys}));
        }
        session
            .messages
            .push(serde_json::json!({"role": "user", "content": user_msg}));
    }
    let mut all_tools = Vec::new();
    for _ in 0..15 {
        let messages = app
            .sessions
            .lock()
            .await
            .get(sid)
            .map(|s| s.messages.clone())
            .unwrap_or_default();
        match llm_call(&config, &messages).await {
            Ok((raw_msg, tool_calls)) => {
                if tool_calls.is_empty() {
                    let content = raw_msg["content"].as_str().unwrap_or("").to_string();
                    app.sessions
                        .lock()
                        .await
                        .entry(sid.to_string())
                        .or_default()
                        .messages
                        .push(raw_msg);
                    return ChatRes {
                        session_id: sid.into(),
                        message: content,
                        tool_calls: all_tools,
                        done: true,
                    };
                }
                app.sessions
                    .lock()
                    .await
                    .entry(sid.to_string())
                    .or_default()
                    .messages
                    .push(raw_msg);
                for tc in &tool_calls {
                    let (output, status) = exec_tool(&tc.name, &tc.args, &cwd_path).await;
                    all_tools.push(ToolCallResult {
                        tool: tc.name.clone(),
                        input: tc.args.clone(),
                        output: output.clone(),
                        status,
                    });
                    app.sessions.lock().await.entry(sid.to_string()).or_default().messages.push(
                        serde_json::json!({"role": "tool", "tool_call_id": tc.id, "content": output})
                    );
                }
            }
            Err(e) => {
                let err_msg = format!("Error: {}", e);
                app.sessions
                    .lock()
                    .await
                    .entry(sid.to_string())
                    .or_default()
                    .messages
                    .push(serde_json::json!({"role": "assistant", "content": &err_msg}));
                return ChatRes {
                    session_id: sid.into(),
                    message: err_msg,
                    tool_calls: all_tools,
                    done: true,
                };
            }
        }
    }
    let max_msg = "Reached max iterations — try continuing.".to_string();
    app.sessions
        .lock()
        .await
        .entry(sid.to_string())
        .or_default()
        .messages
        .push(serde_json::json!({"role": "assistant", "content": &max_msg}));
    ChatRes {
        session_id: sid.into(),
        message: max_msg,
        tool_calls: all_tools,
        done: true,
    }
}
async fn handle_chat(State(app): State<App>, Json(req): Json<ChatReq>) -> Json<ChatRes> {
    let sid = req
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    Json(agent_loop(&app, &sid, &req.message).await)
}
async fn handle_config_get(State(app): State<App>) -> Json<Config> {
    Json(app.config.lock().await.clone())
}
async fn handle_config_set(State(app): State<App>, Json(req): Json<ConfigReq>) -> Json<Config> {
    let mut cfg = app.config.lock().await;
    if let Some(v) = req.provider {
        cfg.provider = v;
    }
    if let Some(v) = req.model {
        cfg.model = v;
    }
    if let Some(v) = req.api_key {
        cfg.api_key = v;
    }
    if let Some(v) = req.base_url {
        cfg.base_url = v;
    }
    if let Some(v) = req.auto_approve {
        cfg.auto_approve = v;
    }
    if let Some(v) = req.working_dir {
        cfg.working_dir = v.clone();
        *app.cwd.lock().await = PathBuf::from(v);
    }
    if let Some(v) = req.model_dir {
        cfg.model_dir = v;
    }
    let config_path = dirs::home_dir()
        .map(|h| h.join(".amni").join("config.json"))
        .unwrap_or_default();
    if let Ok(json) = serde_json::to_string(&*cfg) {
        let _ = std::fs::create_dir_all(config_path.parent().unwrap());
        let _ = std::fs::write(&config_path, json);
    }
    Json(cfg.clone())
}
async fn handle_models(State(app): State<App>) -> Json<ModelsRes> {
    let cfg = app.config.lock().await.clone();
    let base = cfg.base_url.trim_end_matches('/').to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    let mut models = Vec::new();
    tracing::info!(
        "Model discovery: provider={}, base_url={}",
        cfg.provider,
        base
    );
    if cfg.provider == "ollama" {
        let ollama_installed = tokio::process::Command::new("ollama")
            .arg("--version")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ollama_installed {
            let mut resp = client.get(format!("{}/api/tags", base)).send().await;
            if resp.is_err() {
                let _ = tokio::process::Command::new("ollama").arg("serve").spawn();
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                resp = client.get(format!("{}/api/tags", base)).send().await;
            }
            if let Ok(r) = resp {
                if let Ok(json) = r.json::<serde_json::Value>().await {
                    if let Some(arr) = json["models"].as_array() {
                        for m in arr {
                            if let Some(n) = m["name"].as_str() {
                                models.push(n.to_string());
                            }
                        }
                    }
                }
            }
        }
        if models.is_empty() {
            let scan_dir = if !cfg.model_dir.is_empty() {
                Some(PathBuf::from(&cfg.model_dir))
            } else {
                auto_detect_model_dir(&cfg.working_dir).await
            };
            if let Some(d) = scan_dir {
                tracing::info!("Ollama file scan: {:?}", d);
                models = collect_models(&d).await;
            }
        }
    } else if cfg.provider == "local" {
        tracing::info!("Local: trying {}/v1/models", base);
        match client.get(format!("{}/v1/models", base)).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        tracing::info!("Local /v1/models status={} body={}", status, json);
                        if let Some(arr) = json["data"].as_array() {
                            for m in arr {
                                if let Some(n) = m["id"].as_str() {
                                    models.push(n.to_string());
                                }
                            }
                        }
                    }
                    Err(e) => tracing::warn!("Local /v1/models parse error: {}", e),
                }
            }
            Err(e) => tracing::warn!("Local /v1/models fetch error: {}", e),
        }
        if models.is_empty() {
            tracing::info!("Local: trying {}/api/tags", base);
            match client.get(format!("{}/api/tags", base)).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            tracing::info!("Local /api/tags status={} body={}", status, json);
                            if let Some(arr) = json["models"].as_array() {
                                for m in arr {
                                    if let Some(n) = m["name"].as_str() {
                                        models.push(n.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => tracing::warn!("Local /api/tags parse error: {}", e),
                    }
                }
                Err(e) => tracing::warn!("Local /api/tags fetch error: {}", e),
            }
        }
        let scan_dir = if !cfg.model_dir.is_empty() {
            Some(PathBuf::from(&cfg.model_dir))
        } else {
            auto_detect_model_dir(&cfg.working_dir).await
        };
        if let Some(d) = scan_dir {
            tracing::info!("Local file scan: {:?}", d);
            let file_models = collect_models(&d).await;
            for m in file_models {
                if !models.contains(&m) {
                    models.push(m);
                }
            }
        }
    } else if cfg.provider == "openai" {
        if !cfg.api_key.is_empty() {
            if let Ok(resp) = client
                .get(format!("{}/v1/models", base))
                .header("Authorization", format!("Bearer {}", cfg.api_key))
                .send()
                .await
            {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(arr) = json["data"].as_array() {
                        for m in arr {
                            if let Some(id) = m["id"].as_str() {
                                if id.starts_with("gpt-") || id.starts_with("o") {
                                    // Skip known deprecated
                                    if !id.contains("preview")
                                        && !id.contains("deprecated")
                                        && !id.contains("dall-e")
                                        && !id.contains("tts")
                                        && !id.contains("whisper")
                                        && !id.contains("embedding")
                                        && !id.contains("moderation")
                                        && !id.contains("babbage")
                                        && !id.contains("davinci")
                                        && !id.contains("ada")
                                        && !id.contains("curie")
                                    {
                                        models.push(id.to_string());
                                    }
                                }
                            }
                        }
                        models.sort();
                        models.dedup();
                    }
                }
            }
        }
        // Fallback if fetch fails
        if models.is_empty() {
            models.extend(
                [
                    "gpt-5.4",
                    "gpt-5.4-pro",
                    "gpt-5.4-mini",
                    "gpt-5.4-nano",
                    "gpt-5-mini",
                    "gpt-5-nano",
                    "gpt-5",
                    "gpt-4.1",
                    "gpt-5.2",
                    "gpt-5.1",
                    "gpt-5.2-pro",
                    "gpt-5-pro",
                    "o3-pro",
                    "o3",
                    "o4-mini",
                    "gpt-4.1-mini",
                    "gpt-4.1-nano",
                    "o1-pro",
                    "o3-mini",
                    "o1",
                    "gpt-4o",
                    "gpt-4o-mini",
                    "gpt-4-turbo",
                    "gpt-3.5-turbo",
                    "gpt-4",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
        }
    } else if cfg.provider == "anthropic" {
        models.extend(
            [
                "claude-sonnet-4-20250514",
                "claude-opus-4-20250514",
                "claude-haiku-4-20241022",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
    } else if cfg.provider == "xai" {
        models.extend(
            [
                "grok-4-1-fast-reasoning",
                "grok-4-1-fast-non-reasoning",
                "grok-4.20-0309-reasoning",
                "grok-4.20-0309-non-reasoning",
                "grok-4.20-multi-agent-0309",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
    }
    tracing::info!("Model discovery found {} models", models.len());
    Json(ModelsRes { models })
}
async fn handle_dirs(
    axum::extract::Query(q): axum::extract::Query<DirQuery>,
    State(app): State<App>,
) -> Json<DirsRes> {
    let base = match q.path {
        Some(p) => p,
        None => app.config.lock().await.working_dir.clone(),
    };
    let mut dirs = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&base).await {
        while let Ok(Some(e)) = rd.next_entry().await {
            if e.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let n = e.file_name().to_string_lossy().to_string();
                if !n.starts_with('.') {
                    dirs.push(n);
                }
            }
        }
    }
    dirs.sort();
    Json(DirsRes { dirs })
}
async fn find_gguf_path(model_dir: &Path, model_name: &str) -> Option<PathBuf> {
    let target_gguf = format!("{}.gguf", model_name);
    let mut stack: Vec<(PathBuf, u32)> = vec![(model_dir.to_path_buf(), 0)];
    while let Some((current_dir, depth)) = stack.pop() {
        if depth > 3 {
            continue;
        }
        if let Ok(mut rd) = tokio::fs::read_dir(&current_dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                let path = e.path();
                if path.is_dir() {
                    let dname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !dname.starts_with('.')
                        && dname != "palace_textures"
                        && dname != "__pycache__"
                        && dname != "node_modules"
                    {
                        stack.push((path, depth + 1));
                    }
                } else if path.file_name().and_then(|n| n.to_str()) == Some(target_gguf.as_str()) {
                    return Some(path);
                }
            }
        }
    }
    None
}
async fn ensure_model_loaded(config: &Config) {
    if config.provider != "ollama" && config.provider != "local" {
        return;
    }
    let model_dir = if !config.model_dir.is_empty() {
        Some(PathBuf::from(&config.model_dir))
    } else {
        auto_detect_model_dir(&config.working_dir).await
    };
    let dir = match model_dir {
        Some(d) => d,
        None => return,
    };
    let gguf_path = match find_gguf_path(&dir, &config.model).await {
        Some(p) => p,
        None => return,
    };
    tracing::info!(
        "Model '{}' has GGUF file: {:?} — checking Ollama",
        config.model,
        gguf_path
    );
    let base = config.base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default();
    let already_exists = client
        .post(format!("{}/api/show", base))
        .json(&serde_json::json!({"model": config.model}))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    if already_exists {
        tracing::info!("Model '{}' already exists in Ollama", config.model);
        return;
    }
    tracing::info!(
        "Importing '{}' into Ollama from {:?}",
        config.model,
        gguf_path
    );
    let modelfile = format!("FROM {}", gguf_path.display());
    match client
        .post(format!("{}/api/create", base))
        .json(&serde_json::json!({"model": config.model, "modelfile": modelfile, "stream": false}))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!("Imported '{}' into Ollama successfully", config.model);
        }
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!("Ollama import failed for '{}': {}", config.model, text);
        }
        Err(e) => tracing::warn!("Ollama import request failed: {}", e),
    }
}
async fn auto_detect_model_dir(working_dir: &str) -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(working_dir).join("models"),
        PathBuf::from(working_dir)
            .parent()
            .map(|p| p.join("models"))
            .unwrap_or_default(),
        dirs::home_dir()
            .map(|h| h.join("models"))
            .unwrap_or_default(),
        dirs::home_dir()
            .map(|h| h.join(".cache").join("huggingface").join("hub"))
            .unwrap_or_default(),
    ];
    for c in &candidates {
        if !c.as_os_str().is_empty() && tokio::fs::metadata(c).await.is_ok() {
            tracing::info!("Auto-detected model dir: {:?}", c);
            return Some(c.clone());
        }
    }
    None
}
async fn collect_models(dir: &PathBuf) -> Vec<String> {
    let mut models = Vec::new();
    let mut stack: Vec<(PathBuf, u32)> = vec![(dir.clone(), 0)];
    while let Some((current_dir, depth)) = stack.pop() {
        if depth > 3 {
            continue;
        }
        if let Ok(mut rd) = tokio::fs::read_dir(&current_dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                let path = e.path();
                if path.is_dir() {
                    let dname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !dname.starts_with('.')
                        && dname != "palace_textures"
                        && dname != "__pycache__"
                        && dname != "node_modules"
                    {
                        stack.push((path, depth + 1));
                    }
                } else if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(name) = fname
                        .strip_suffix(".gguf")
                        .or_else(|| fname.strip_suffix(".safetensors"))
                    {
                        models.push(name.to_string());
                    }
                }
            }
        }
    }
    models.sort();
    models
}
#[derive(Deserialize)]
struct HfSearchQuery {
    q: Option<String>,
}
#[derive(Serialize)]
struct HfModelResult {
    id: String,
    downloads: u64,
    likes: u64,
    tags: Vec<String>,
}
async fn handle_hf_search(Query(q): Query<HfSearchQuery>) -> Json<Vec<HfModelResult>> {
    let query = q.q.unwrap_or_default();
    if query.is_empty() {
        return Json(vec![]);
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let url = format!(
        "https://huggingface.co/api/models?search={}&filter=gguf&sort=downloads&direction=-1&limit=20",
        urlencoding::encode(&query)
    );
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("HF search error: {}", e);
            return Json(vec![]);
        }
    };
    let json: Vec<serde_json::Value> = match resp.json().await {
        Ok(j) => j,
        Err(_) => return Json(vec![]),
    };
    let results: Vec<HfModelResult> = json
        .into_iter()
        .map(|m| HfModelResult {
            id: m["id"].as_str().unwrap_or("").to_string(),
            downloads: m["downloads"].as_u64().unwrap_or(0),
            likes: m["likes"].as_u64().unwrap_or(0),
            tags: m["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .filter(|m| !m.id.is_empty())
        .collect();
    Json(results)
}
#[derive(Deserialize)]
struct HfFilesQuery {
    repo: Option<String>,
}
#[derive(Serialize)]
struct HfFileInfo {
    name: String,
    size: u64,
}
async fn handle_hf_files(Query(q): Query<HfFilesQuery>) -> Json<Vec<HfFileInfo>> {
    let repo = q.repo.unwrap_or_default();
    if repo.is_empty() {
        return Json(vec![]);
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();
    let url = format!(
        "https://huggingface.co/api/models/{}/tree/main",
        repo
    );
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("HF files error: {}", e);
            return Json(vec![]);
        }
    };
    let json: Vec<serde_json::Value> = match resp.json().await {
        Ok(j) => j,
        Err(_) => return Json(vec![]),
    };
    let files: Vec<HfFileInfo> = json
        .iter()
        .filter_map(|f| {
            let name = f["path"].as_str()?;
            if name.ends_with(".gguf") {
                Some(HfFileInfo {
                    name: name.to_string(),
                    size: f["size"].as_u64().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect();
    Json(files)
}
#[derive(Deserialize)]
struct HfDownloadReq {
    repo: String,
    file: String,
}
async fn handle_hf_download(
    State(app): State<App>,
    Json(req): Json<HfDownloadReq>,
) -> Json<serde_json::Value> {
    let cfg = app.config.lock().await.clone();
    let dest_dir = if !cfg.model_dir.is_empty() {
        PathBuf::from(&cfg.model_dir)
    } else {
        match auto_detect_model_dir(&cfg.working_dir).await {
            Some(d) => d,
            None => {
                let fallback = dirs::home_dir()
                    .map(|h| h.join("models"))
                    .unwrap_or_else(|| PathBuf::from("models"));
                let _ = tokio::fs::create_dir_all(&fallback).await;
                fallback
            }
        }
    };
    let _ = tokio::fs::create_dir_all(&dest_dir).await;
    let dest_file = dest_dir.join(&req.file);
    if dest_file.exists() {
        return Json(serde_json::json!({"status": "exists", "path": dest_file.display().to_string()}));
    }
    {
        let mut prog = app.dl_progress.lock().await;
        if !prog.done && prog.total > 0 && prog.downloaded < prog.total {
            return Json(
                serde_json::json!({"status": "busy", "message": "A download is already in progress"}),
            );
        }
        *prog = DownloadProgress {
            repo: req.repo.clone(),
            file: req.file.clone(),
            downloaded: 0,
            total: 0,
            done: false,
            error: String::new(),
        };
    }
    let progress = app.dl_progress.clone();
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        req.repo, req.file
    );
    let dest_path_str = dest_file.display().to_string();
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(7200))
            .build()
            .unwrap_or_default();
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                let mut p = progress.lock().await;
                p.done = true;
                p.error = format!("Request failed: {}", e);
                return;
            }
        };
        if !resp.status().is_success() {
            let mut p = progress.lock().await;
            p.done = true;
            p.error = format!("HTTP {}", resp.status());
            return;
        }
        let total = resp.content_length().unwrap_or(0);
        {
            let mut p = progress.lock().await;
            p.total = total;
        }
        let tmp_path = dest_file.with_extension("gguf.part");
        let mut file = match tokio::fs::File::create(&tmp_path).await {
            Ok(f) => f,
            Err(e) => {
                let mut p = progress.lock().await;
                p.done = true;
                p.error = format!("File create error: {}", e);
                return;
            }
        };
        use tokio::io::AsyncWriteExt;
        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        let mut downloaded: u64 = 0;
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    if file.write_all(&bytes).await.is_err() {
                        let mut p = progress.lock().await;
                        p.done = true;
                        p.error = "Write error".to_string();
                        return;
                    }
                    downloaded += bytes.len() as u64;
                    let mut p = progress.lock().await;
                    p.downloaded = downloaded;
                }
                Err(e) => {
                    let mut p = progress.lock().await;
                    p.done = true;
                    p.error = format!("Stream error: {}", e);
                    return;
                }
            }
        }
        let _ = file.flush().await;
        drop(file);
        if let Err(e) = tokio::fs::rename(&tmp_path, &dest_file).await {
            let mut p = progress.lock().await;
            p.done = true;
            p.error = format!("Rename error: {}", e);
            return;
        }
        let mut p = progress.lock().await;
        p.downloaded = total;
        p.done = true;
        tracing::info!("Download complete: {:?}", dest_file);
    });
    Json(serde_json::json!({"status": "started", "path": dest_path_str}))
}
async fn handle_hf_progress(State(app): State<App>) -> Json<DownloadProgress> {
    Json(app.dl_progress.lock().await.clone())
}
async fn handle_health() -> &'static str {
    "ok"
}
async fn serve_ui() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cwd = std::env::current_dir().unwrap_or_default();
    let home_env = dirs::home_dir()
        .map(|h| h.join(".amni").join(".env"))
        .unwrap_or_default();
    for env_path in [home_env, cwd.join(".env")] {
        if env_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&env_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        std::env::set_var(k.trim(), v.trim());
                    }
                }
            }
        }
    }
    let config_path = dirs::home_dir()
        .map(|h| h.join(".amni").join("config.json"))
        .unwrap_or_default();
    let config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| Config::default())
    } else {
        Config::default()
    };
    println!("\n  Amni-Code v1.1.0 — AI Coding Agent");
    println!("  Working dir: {}", cwd.display());
    let app = App {
        sessions: Arc::new(Mutex::new(HashMap::new())),
        config: Arc::new(Mutex::new(config)),
        cwd: Arc::new(Mutex::new(cwd)),
        dl_progress: Arc::new(Mutex::new(DownloadProgress::default())),
    };
    let router = Router::new()
        .route("/", get(serve_ui))
        .route("/api/chat", post(handle_chat))
        .route(
            "/api/config",
            get(handle_config_get).post(handle_config_set),
        )
        .route("/api/models", get(handle_models))
        .route("/api/dirs", get(handle_dirs))
        .route("/api/hf/search", get(handle_hf_search))
        .route("/api/hf/files", get(handle_hf_files))
        .route("/api/hf/download", post(handle_hf_download))
        .route("/api/hf/progress", get(handle_hf_progress))
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
            if let Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } = event
            {
                *control_flow = ControlFlow::Exit;
            }
        });
    }
    Ok(())
}
