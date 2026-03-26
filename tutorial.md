# Amni-Code Tutorial

Amni-Code is an open-source AI coding agent built in Rust. It gives an LLM real tools to read, write, and edit files, run shell commands, and search your codebase — all from a browser UI at `localhost:3000`.

## One-Click Install (Windows)

The fastest way to get started:

```bash
# Download and run the quickstart script
curl -L -o quickstart.bat https://raw.githubusercontent.com/anmire/Amni-Code/main/quickstart.bat
quickstart.bat
```

This will:
1. Clone the repository
2. Prompt you for API keys (xAI Grok, OpenAI, Anthropic — press Enter to skip any)
3. Install Rust if needed
4. Build the application
5. Launch it at http://localhost:3000

### Already cloned? Just run:
```bash
cd Amni-Code
quickstart.bat
```

## GUI Installer

For a guided experience with progress bars and hardware detection:

```bash
git clone https://github.com/anmire/Amni-Code.git
cd Amni-Code
install.bat
```

The GUI installer provides:
- Real-time progress tracking
- GPU detection (NVIDIA CUDA / AMD HIP/ROCm)
- API key entry fields for xAI, OpenAI, and Anthropic
- Desktop shortcut creation
- One-click launch after install

## Manual Build

```bash
git clone https://github.com/anmire/Amni-Code.git
cd Amni-Code
cargo build --release
target\release\amni-code.exe
```

## Setting Up API Keys

Amni-Code defaults to **xAI Grok**. You need at least one API key from a supported provider.

### During Installation
The quickstart script and GUI installer both prompt for API keys. They're saved to a `.env` file (gitignored).

### After Installation
1. Open http://localhost:3000
2. Click the **⚙ Settings** button (top right)
3. Select your **Provider** (xAI, OpenAI, Anthropic, Ollama)
4. Enter your **API Key**
5. Close the settings panel — changes save automatically

### Manual `.env` File
Create a `.env` file in the Amni-Code directory:
```
XAI_API_KEY=xai-your-key-here
OPENAI_API_KEY=sk-your-key-here
ANTHROPIC_API_KEY=sk-ant-your-key-here
```

## Using Amni-Code

### Chat Interface
Type anything in the text box and press Enter. The agent will:
- Read files to understand your codebase
- Write or edit files to implement changes
- Run shell commands (build, test, install)
- Search across your project for relevant code
- Iterate up to 15 times per request to complete complex tasks

### Example Prompts

**Build something:**
```
Create a Python Flask API with user authentication and a SQLite database.
```

**Debug code:**
```
Find and fix the bug in main.py — the server crashes on startup.
```

**Explore a codebase:**
```
Explain how this project is structured and what each module does.
```

**Scaffold a project:**
```
Set up a new React project with TypeScript, ESLint, and Tailwind CSS.
```

### Code Diff Panel
Click the **diff icon** (top right) to see every file change the agent makes. Shows added/removed lines for each file.

### Auto-Approve Mode
In Settings, toggle **Auto-approve all agent actions**:
- **Off** (default): You see what the agent wants to do before it executes
- **On**: The agent acts fully autonomously

### Working Directory
The agent operates in the directory shown in Settings. Change it to point at any project on your machine.

## Supported Providers

| Provider | Models | API Key Env Var |
|----------|--------|-----------------|
| **xAI** (default) | grok-3, grok-3-mini, grok-2 | `XAI_API_KEY` |
| **OpenAI** | gpt-4o, gpt-4o-mini, o1, o3-mini | `OPENAI_API_KEY` |
| **Anthropic** | claude-sonnet-4, claude-opus-4, claude-3.5-haiku | `ANTHROPIC_API_KEY` |
| **Ollama** | Any local model | (none needed) |
| **Custom** | Any OpenAI-compatible server | (none needed) |

## Troubleshooting

**Port 3000 busy:** Another app is using port 3000. Kill it or change the port in `src/main.rs` and rebuild.

**Build fails:** Make sure Rust is installed: `rustup --version`. If not, run `quickstart.bat` or install from https://rustup.rs.

**"Request failed" error:** Your API key may be invalid or the provider is unreachable. Check Settings and verify your key.

**Ollama not responding:** Make sure Ollama is running (`ollama serve`) and accessible at `http://localhost:11434`.

## Contributing

See [README.md](README.md#contributing) for contribution guidelines.

## License

MIT — see [LICENSE](LICENSE).
