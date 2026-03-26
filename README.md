<p align="center">
  <h1 align="center">Amni-Code</h1>
  <p align="center">
    <b>An open-source AI coding agent that runs in your browser.</b><br>
    Built in Rust. Works with any OpenAI-compatible backend — Ollama, OpenAI, Anthropic, xAI, or your own local server.
  </p>
  <p align="center">
    <a href="https://ko-fi.com/anmire"><img src="https://img.shields.io/badge/Ko--fi-Support%20the%20project-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a>
    <a href="https://github.com/anmire/Amni-Code/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
    <img src="https://img.shields.io/badge/built%20with-Rust-orange?logo=rust" alt="Rust">
  </p>
</p>

---

## What is Amni-Code?

Amni-Code is a **self-hosted AI coding agent** — think Claude Code or Cursor, but fully open-source and running on your own machine. It gives an LLM (local or cloud) real tools to read, write, edit files, run shell commands, search your codebase, and iterate on code autonomously.

**Key features:**
- **Agentic tool loop** — The AI can read files, edit code, run commands, list directories, and search across your project — up to 15 iterations per request, fully autonomous.
- **Any LLM provider** — xAI (Grok), OpenAI, Anthropic, Ollama (local), or any OpenAI-compatible server.
- **Single binary + embedded UI** — One Rust binary serves everything. No Node, no Docker, no electron. Just run it.
- **Code diff panel** — See every file change the agent makes in a side-by-side diff view.
- **Auto-approve toggle** — Watch every action before it executes, or let the agent run freely.
- **Defaults to xAI Grok** — Works out of the box with a Grok API key. Switch providers anytime in Settings.

## Quick Start

### One-Click Quickstart (Windows)

Download and run [`quickstart.bat`](quickstart.bat) — it clones the repo, prompts for your API key(s), builds, and launches:

```bash
curl -L -o quickstart.bat https://raw.githubusercontent.com/anmire/Amni-Code/main/quickstart.bat
quickstart.bat
```

Or clone and run it yourself:
```bash
git clone https://github.com/anmire/Amni-Code.git
cd Amni-Code
quickstart.bat
```

You'll be prompted for API keys during setup (press Enter to skip any):
```
  xAI API Key (xai-...): _______________
  OpenAI API Key (sk-...): _______________
  Anthropic API Key (sk-ant-...): _______________
```

### GUI Installer (Windows)
```bash
git clone https://github.com/anmire/Amni-Code.git
cd Amni-Code
install.bat
```
The GUI installer handles Rust, Python, GPU detection, API key configuration, model downloads, and building.

### Build from Source
```bash
git clone https://github.com/anmire/Amni-Code.git
cd Amni-Code
cargo build --release
target\release\amni-code.exe   # Windows
# or: ./target/release/amni-code  # Linux/macOS
```

### Just Run It
```bash
.\run.bat   # Windows — auto-installs Rust and builds if needed
```

Then open **http://localhost:3000** in your browser.

## How It Works

```
┌──────────────────────────────────────────────────────┐
│  Browser UI (localhost:3000)                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────────┐ │
│  │  Chat     │ │ Settings │ │  Code Diff Panel     │ │
│  │  Window   │ │  Panel   │ │  (live file changes) │ │
│  └──────────┘ └──────────┘ └──────────────────────┘ │
└───────────────────────┬──────────────────────────────┘
                        │ /api/chat
┌───────────────────────▼──────────────────────────────┐
│  Rust Backend (Axum)                                  │
│  ┌─────────────────────────────────────────────────┐ │
│  │  Agent Loop (up to 15 iterations)               │ │
│  │                                                 │ │
│  │  1. Send messages + tools → LLM                 │ │
│  │  2. LLM returns tool calls (or final answer)    │ │
│  │  3. Execute tools locally                       │ │
│  │  4. Feed results back → LLM                     │ │
│  │  5. Repeat until done                           │ │
│  └─────────────────────────────────────────────────┘ │
│                                                       │
│  Tools:                                               │
│  • read_file    — Read any file                       │
│  • write_file   — Create/overwrite files              │
│  • edit_file    — Find-and-replace in files           │
│  • run_command  — Execute shell commands              │
│  • list_directory — Browse the filesystem             │
│  • search_files — Grep across your codebase           │
└───────────────────────┬──────────────────────────────┘
                        │ OpenAI-compatible API
┌───────────────────────▼──────────────────────────────┐
│  LLM Provider                                         │
│  Ollama · OpenAI · Anthropic · xAI · Any local server │
└──────────────────────────────────────────────────────┘
```

The agent loop is the core: your message goes to the LLM with a set of tools. The LLM decides what to do (read a file, run a command, etc.), Amni-Code executes it locally, feeds the result back, and the LLM continues until the task is done.

## Configuration

Click the **⚙ Settings** button in the UI to configure:

| Setting | Description |
|---------|-------------|
| **Provider** | xAI (Grok), OpenAI, Anthropic, Ollama, or any OpenAI-compatible server |
| **Model** | Auto-populated from your provider. Pick any model. |
| **API Key** | Required for cloud providers. Auto-detected from `.env` file or env vars (`XAI_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) |
| **Base URL** | Server URL. Defaults to `https://api.x.ai` for Grok, `http://localhost:11434` for Ollama |
| **Working Directory** | The directory the agent operates in |
| **Auto-approve** | When off, you confirm each action. When on, fully autonomous. |

### API Keys

Amni-Code loads API keys from a `.env` file in the project directory (created by the installer) or from system environment variables:

```bash
# .env file (or set as environment variables)
XAI_API_KEY=xai-...          # xAI Grok (default provider)
OPENAI_API_KEY=sk-...        # OpenAI
ANTHROPIC_API_KEY=sk-ant-... # Anthropic
```

The `.env` file is gitignored and never committed. You can also enter keys directly in the Settings panel.

## Hardware Support

The installer auto-detects your GPU and configures acceleration:

| GPU | Framework | Notes |
|-----|-----------|-------|
| **NVIDIA** | CUDA 12.0+ | Full acceleration. Install CUDA toolkit. |
| **AMD** | HIP/ROCm 7.1+ | Installer guides setup. 7000-series supported. |
| **CPU** | — | Works on any modern CPU. 16GB+ RAM recommended. |

## System Requirements

- **OS**: Windows 10/11 (Linux/macOS: build from source)
- **RAM**: 8GB minimum, 16GB+ recommended for local models
- **Storage**: ~500MB for the app, models vary (2GB–70GB+)
- **Rust**: 1.70+ (auto-installed by `install.bat` or `run.bat`)

## Project Structure

```
Amni-Code/
├── src/
│   └── main.rs          # Rust backend — server, agent loop, tool execution, LLM routing
├── static/
│   └── index.html       # Embedded web UI — chat, settings, diff panel
├── Cargo.toml           # Rust dependencies
├── quickstart.bat       # One-click install + API key setup + launch
├── install.bat          # GUI installer (Windows)
├── install.py           # Python installer script
├── install_gui.py       # GUI wrapper for installer
├── run.bat              # Quick-start launcher
├── .env                 # Your API keys (gitignored, created by installer)
└── models/              # Local model storage (gitignored)
```

## Contributing

Contributions welcome! Here's how:

1. **Fork** the repo
2. **Create a branch**: `git checkout -b my-feature`
3. **Make changes** and test
4. **Submit a PR**

### Ideas for contributions:
- Linux/macOS installer scripts
- Streaming responses (SSE/WebSocket)
- File tree sidebar in the UI
- Conversation history/persistence
- More LLM providers (Google Gemini, Mistral, etc.)
- Syntax highlighting in code blocks
- Image/multimodal support

## Support the Project

If Amni-Code is useful to you, consider supporting development:

<a href="https://ko-fi.com/anmire"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Support on Ko-fi"></a>

## License

MIT — see [LICENSE](LICENSE) for details.
