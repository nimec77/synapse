# Synapse

A Rust-based AI agent that provides a unified interface to multiple LLM providers — Anthropic Claude,
DeepSeek, and OpenAI — through a CLI, Telegram bot, and backend service.

## Features

- **Multi-provider support**: Anthropic Claude, DeepSeek, and OpenAI (providers implemented from
  scratch — no rig/genai/async-openai)
- **CLI with interactive REPL**: Terminal UI built with ratatui/crossterm for multi-turn conversations
- **Streaming responses**: Token-by-token output with Ctrl+C interruption
- **MCP tool calling**: Model Context Protocol integration via [rmcp](https://github.com/modelcontextprotocol/rust-sdk)
- **SQLite session persistence**: Conversation history with auto-cleanup and resume
- **Telegram bot**: Session-per-chat persistence with user allowlist authorization
- **System prompt**: Configurable via inline string or external file
- **File logging**: Rolling log files with configurable rotation (Telegram bot)
- **Hexagonal architecture**: Clean port/adapter separation; core never imports from interfaces

## Status

This project is under active development. All features listed above are implemented and tested, but
the API surface and configuration format may change between releases.

## Quick Start

**Prerequisites**: Rust nightly toolchain (managed via `rust-toolchain.toml`)

```bash
# Clone the repository
git clone https://github.com/nimec77/synapse.git
cd synapse

# Build
cargo build --release

# Create a config file (see Configuration section for all options)
cp config.example.toml ~/.config/synapse/config.toml
# Edit the file: set provider, model, and api_key (or set an env var)

# Run
./target/release/synapse "Hello, world!"
```

## CLI Usage

The CLI binary is named `synapse`.

### One-shot mode

```bash
synapse "What is the capital of France?"
```

### Stdin pipe

```bash
echo "Summarize this" | synapse
cat document.txt | synapse "Summarize the above"
```

### Interactive REPL

```bash
synapse --repl          # Start a new session
synapse -r              # Short form
synapse -r -s <uuid>    # Resume an existing session
```

Inside the REPL: type a message and press Enter to send. `/quit` or Ctrl+C to exit. The session ID
is printed to stderr on exit so you can resume later.

### Continue an existing session (one-shot)

```bash
synapse --session <uuid> "Follow-up question"
synapse -s <uuid> "Follow-up question"
```

### Override provider at runtime

```bash
synapse -p openai "Hello"
synapse --provider anthropic "Hello"
synapse -p deepseek -r          # REPL with DeepSeek
```

### Session management

```bash
synapse sessions list             # List all sessions
synapse sessions show <uuid>      # Show messages in a session
synapse sessions delete <uuid>    # Delete a session
```

### Use a custom config file

```bash
synapse --config /path/to/config.toml "Hello"
synapse -c ./my-config.toml --repl
```

### Help

```bash
synapse --help
synapse sessions --help
```

## Telegram Bot

**Prerequisites**: A Telegram bot token from [@BotFather](https://t.me/BotFather) and your numeric
user ID (message [@userinfobot](https://t.me/userinfobot) to find it).

1. Add the Telegram section to your config:

```toml
[telegram]
# token = "123456:ABC-DEF..."   # or set TELEGRAM_BOT_TOKEN env var
allowed_users = [123456789]     # empty list rejects ALL users
```

2. Run the bot binary:

```bash
cargo build --release
TELEGRAM_BOT_TOKEN="123456:ABC-DEF..." ./target/release/synapse-telegram
```

Each chat supports multiple persistent sessions (up to `max_sessions_per_chat`, default 10). The bot
resumes conversations across restarts. Unauthorized users are silently ignored (secure by default —
an empty `allowed_users` list blocks everyone).

### Bot commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/new` | Start a new session |
| `/history` | Show conversation history of the current session |
| `/list` | List all sessions for this chat |
| `/switch N` | Switch to session N (1-based index from `/list`) |
| `/delete N` | Delete session N (1-based index from `/list`) |

When `/new` would exceed `max_sessions_per_chat`, the oldest session is automatically evicted. Set
`max_sessions_per_chat` in the `[telegram]` config section to adjust the cap.

## Configuration

### Config file search order

Synapse searches for a config file in this order (first found wins):

1. `--config <path>` CLI flag — error if the file does not exist
2. `./config.toml` — current working directory
3. `~/.config/synapse/config.toml` — user config directory
4. Error — no silent defaults

### Full annotated example

```toml
# LLM provider: deepseek | anthropic | openai
provider = "deepseek"

# API key for the selected provider.
# Prefer the environment variable equivalents (see below) over storing keys in this file.
# api_key = "your-api-key-here"

# Model name
# DeepSeek: deepseek-chat, deepseek-reasoner
# Anthropic: claude-sonnet-4-6, claude-opus-4-6
# OpenAI: gpt-4o, o3-mini
model = "deepseek-chat"

# System prompt prepended to every conversation (never stored in the database).
# system_prompt = "You are a helpful programming assistant."

# Load system prompt from a file instead. Inline system_prompt takes priority if both are set.
# system_prompt_file = "prompts/system.md"

[session]
# SQLite database path. Also overridable via DATABASE_URL env var.
# database_url = "sqlite:~/.config/synapse/sessions.db"
max_sessions = 100       # oldest sessions deleted when this limit is exceeded; 0 = unlimited
retention_days = 90      # delete sessions older than N days; 0 = keep forever
auto_cleanup = true      # run cleanup on startup

[mcp]
# Path to the MCP servers JSON file. Also overridable via SYNAPSE_MCP_CONFIG env var.
# config_path = "~/.config/synapse/mcp_servers.json"

[telegram]
# token = "123456:ABC-DEF..."   # overridable via TELEGRAM_BOT_TOKEN env var
# allowed_users = [123456789, 987654321]

[logging]
# File logging for synapse-telegram. Omit this section for stdout-only output.
directory = "logs"     # relative or absolute path
max_files = 7          # number of rotated files to keep
rotation = "daily"     # "daily" | "hourly" | "never"
```

Protect your config file:

```bash
chmod 600 ~/.config/synapse/config.toml
```

### Environment variables

| Variable             | Overrides                          | Description                                 |
|----------------------|------------------------------------|---------------------------------------------|
| `ANTHROPIC_API_KEY`  | `api_key` in config (Anthropic)    | API key for Anthropic Claude                |
| `DEEPSEEK_API_KEY`   | `api_key` in config (DeepSeek)     | API key for DeepSeek                        |
| `OPENAI_API_KEY`     | `api_key` in config (OpenAI)       | API key for OpenAI                          |
| `TELEGRAM_BOT_TOKEN` | `telegram.token` in config         | Telegram bot token                          |
| `DATABASE_URL`       | `session.database_url` in config   | SQLite database URL                         |
| `SYNAPSE_MCP_CONFIG` | `mcp.config_path` in config        | Path to MCP servers JSON file               |
| `RUST_LOG`           | —                                  | Log level filter (e.g. `debug`, `info`)     |

Provider-specific API key env vars take priority over `api_key` in config. `TELEGRAM_BOT_TOKEN` is
the recommended way to supply the bot token in production (never commit tokens to config files).

## MCP (Tool Calling)

Synapse implements the [Model Context Protocol](https://modelcontextprotocol.io/) for tool calling.
The agent discovers available tools from running MCP servers and automatically invokes them when
the LLM requests a tool call (up to 10 iterations per request).

### Setup

Copy the example config and edit it:

```bash
cp mcp_servers.example.json ~/.config/synapse/mcp_servers.json
```

Example `mcp_servers.json`:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/documents"]
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "ghp_your_token_here"
      }
    }
  }
}
```

Point Synapse to the file via config or env var:

```toml
[mcp]
config_path = "~/.config/synapse/mcp_servers.json"
```

```bash
SYNAPSE_MCP_CONFIG=~/.config/synapse/mcp_servers.json synapse "List files in my documents folder"
```

Without a config file, Synapse behaves identically to pre-MCP — graceful degradation is built in.

## Architecture

Synapse uses hexagonal architecture (ports and adapters). The core library defines traits (ports);
implementations are adapters. Dependencies flow inward only — `synapse-core` never imports from
interface crates.

```
synapse-cli / synapse-telegram      ← Interface binaries (use anyhow for errors)
        │
        ▼
    synapse-core                    ← Shared library (uses thiserror for errors)
        │
        ▼
      Agent                         ← Orchestrator: tool call loop + system prompt injection
   ┌────┼────────────┐
   ▼    ▼            ▼
LlmProvider  SessionStore   McpClient
 (trait)      (trait)        (rmcp)
   │            │
   ▼            ▼
Anthropic    SqliteStore
DeepSeek
OpenAI
Mock (test-only)
```

### Workspace crates

| Crate              | Purpose                                                                      |
|--------------------|------------------------------------------------------------------------------|
| `synapse-core`     | Core library: agent orchestrator, config, providers, storage, MCP, messages  |
| `synapse-cli`      | CLI binary (`synapse`): one-shot, stdin, REPL, session commands              |
| `synapse-telegram` | Telegram bot binary (`synapse-telegram`): long-polling, session-per-chat     |

### Core traits

**`LlmProvider`** — the central abstraction:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
    fn stream(&self, messages: &[Message])
        -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;

    // Default: delegates to complete(), ignoring tools.
    // Anthropic, DeepSeek, and OpenAI override this to pass tools via the API.
    async fn complete_with_tools(
        &self, messages: &[Message], tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError>;

    // Default: delegates to stream(), ignoring tools.
    fn stream_with_tools(
        &self, messages: &[Message], tools: &[ToolDefinition],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
}
```

**`SessionStore`** — storage port:

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: &Session) -> Result<(), StorageError>;
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError>;
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError>;
    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError>;
    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError>;
    // ...
}
```

The `Agent` struct is the sole entry point for inference in interface crates — they never call
`LlmProvider` directly:

```rust
let agent = Agent::from_config(&config, mcp_client)?;
agent.complete(&mut messages).await?;   // handles tool call loop
agent.stream(&messages)                 // streaming, no tools
```

## Building & Testing

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test -p synapse-core     # Tests for one crate
cargo check                    # Type-check without building
cargo fmt                      # Format code
cargo clippy -- -D warnings    # Lint
```

**Pre-commit check (required before every commit):**

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

**Toolchain**: Rust nightly (pinned via `rust-toolchain.toml`), Edition 2024, workspace resolver v3.

## CI/CD

GitHub Actions runs on every push to `master`/`feature/*` and on PRs targeting `master`:

| Job              | Steps                                                                    |
|------------------|--------------------------------------------------------------------------|
| Build & Test     | `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test`      |
| Security Audit   | `rustsec/audit-check` for known vulnerability scanning                   |

## License

MIT — see [LICENSE](LICENSE).
