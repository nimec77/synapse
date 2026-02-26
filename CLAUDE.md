# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Synapse is a Rust-based AI agent that serves as a unified interface to interact with multiple LLM providers (Anthropic Claude, DeepSeek, OpenAI). The project targets multiple interfaces: CLI (primary), Telegram bot, and backend service. LLM providers are implemented from scratch (no rig/genai/async-openai) for learning depth and full control.

## Build Commands

```bash
cargo build                    # Build workspace
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test test_name           # Run specific test
cargo test -p synapse-core     # Run tests for one crate
cargo check                    # Type-check without building
cargo fmt                      # Format code
cargo clippy -- -D warnings    # Lint (CI runs with -D warnings)
```

**Pre-commit (required before every commit):**
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

## Architecture

Hexagonal architecture (ports and adapters). Core defines traits (ports), implementations are adapters.

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
```

**Critical rule:** `synapse-core` never imports from interface crates. Dependencies flow inward only.

### Agent Orchestrator

`Agent` (`synapse-core/src/agent.rs`) is the entry point for all inference. Interface crates never call `LlmProvider` directly — they always go through `Agent`.

```rust
// Preferred: construct from config (calls create_provider + applies system prompt internally)
let mcp_client = init_mcp_client(config.mcp.as_ref().and_then(|m| m.config_path.as_deref())).await;
let agent = Agent::from_config(&config, mcp_client)?;

// Low-level: manual construction
let agent = Agent::new(provider, mcp_client)
    .with_system_prompt("You are a helpful assistant.");

agent.complete(&mut messages).await?;   // blocking, handles tool call loop
agent.stream(&mut messages)             // streaming, tool-aware
agent.stream_owned(messages)            // streaming, takes ownership
agent.shutdown().await;                 // graceful MCP connection teardown
```

`build_messages(&self, messages, tools)` is a private helper that prepends `Role::System` on-the-fly before every provider call without mutating or storing the system message in the session database. When `tools` is non-empty it appends an `## Available Tools` section to the system prompt so that providers which ignore the API-level `tools` field (e.g. DeepSeek) still see the tool list. The tool call loop runs up to `MAX_ITERATIONS = 10`.

### Core Traits

**LlmProvider** (`synapse-core/src/provider.rs`) — the central abstraction:
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
    fn stream(&self, messages: &[Message])
        -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;

    // Default: delegates to complete(), ignoring tools.
    // Anthropic, DeepSeek, and OpenAI override this to pass tools via the API.
    async fn complete_with_tools(&self, messages: &[Message], tools: &[ToolDefinition])
        -> Result<Message, ProviderError>;

    // Default: delegates to stream(), ignoring tools.
    fn stream_with_tools(&self, messages: &[Message], tools: &[ToolDefinition])
        -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
}
```

**Message** (`synapse-core/src/message.rs`) — conversation message:
```rust
pub struct Message {
    pub role: Role,
    pub content: String,
    /// Tool calls requested by the assistant (Some when role == Assistant and model invoked tools).
    pub tool_calls: Option<Vec<ToolCallData>>,
    /// Tool call ID this message responds to (Some when role == Tool).
    pub tool_call_id: Option<String>,
}
```

**SessionStore** (`synapse-core/src/storage.rs`) — persistence port:
```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: &Session) -> Result<(), StorageError>;
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError>;
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError>;
    async fn touch_session(&self, id: Uuid) -> Result<(), StorageError>;
    async fn delete_session(&self, id: Uuid) -> Result<bool, StorageError>;
    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError>;
    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError>;
    async fn cleanup(&self, config: &SessionConfig) -> Result<CleanupResult, StorageError>;
}
```

### Streaming

Providers return `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>`. The `StreamEvent` enum (`provider/streaming.rs`) has exactly two variants:
- `TextDelta(String)` — token fragment
- `Done` — stream complete

`Agent::stream()` and `Agent::stream_owned()` yield `Result<StreamEvent, AgentError>` (not `ProviderError`). When tools are available, they resolve tool call iterations internally via `complete()` and stream only the final text response. When no tools are configured, they delegate directly to the provider's `stream()`. CLI consumes streams with `tokio::select!` for Ctrl+C handling. Uses `async_stream::stream!` macro and `eventsource-stream` for SSE parsing. OpenAI-compatible providers (Anthropic, DeepSeek, OpenAI) set `tool_choice: "auto"` in the API request when tools are present.

### Telegram Bot Architecture

The dispatcher has two top-level branches: message updates and callback query updates. `Me` is injected at startup via `bot.get_me().await?` (required by `filter_command::<Command>()` to strip bot username suffixes like `/new@botname`).

```rust
let handler = dptree::entry()
    .branch(
        Update::filter_message()
            .branch(dptree::entry().filter_command::<Command>().endpoint(commands::handle_command))
            .branch(dptree::entry().endpoint(handlers::handle_message)),
    )
    .branch(Update::filter_callback_query().endpoint(commands::handle_callback));
```

**Multi-session per chat**: `ChatSessionMap` is `Arc<RwLock<HashMap<i64, ChatSessions>>>` where `ChatSessions { sessions: Vec<Uuid>, active_idx: usize }`. Regular messages use `active_session_id()` (hot path, no DB). Commands that display or index sessions (`/list`, `/switch N`, `/delete N`) always call `list_sessions()` fresh for consistent 1-based ordering. Session cap (`max_sessions_per_chat`) is enforced in `/new` only; the oldest session (last in `sessions` vec) is evicted. `rebuild_chat_map` at startup groups `tg:<chat_id>` sessions from the DB with the most recently updated session at `active_idx = 0`.

**Slash commands** (`commands.rs`) do **not** invoke the `Agent`/LLM — they are pure session management. Replies are plain text (no `ParseMode::Html`). Authorization reuses `handlers::is_authorized()`.

**Interactive keyboards**: `/switch` and `/delete` without an argument send an `InlineKeyboardMarkup` (one button per session, callback data `"switch:N"` / `"delete:N"`, 1-based). `handle_callback` calls `answer_callback_query` immediately (before any DB calls) to dismiss the spinner, then executes `do_switch`/`do_delete`, then calls `edit_message_text` to replace the keyboard with a plain-text result. `do_switch` and `do_delete` re-fetch the session list from DB on every call (stale-index safety). `parse_session_arg(arg)` triages the `String` command argument: empty → show keyboard, numeric → execute directly, non-numeric → return error hint.

**Defensive guard**: `handle_message` returns early with a hint for any text starting with `/` before reaching the LLM, preventing future command fall-through regressions.

### Telegram Message Pipeline

LLM responses are Markdown; Telegram requires HTML or MarkdownV2. HTML is used because it only needs `&`, `<`, `>` escaped — MarkdownV2 requires escaping 18+ characters and is fragile for LLM output.

`synapse-telegram/src/format.rs` provides:
- `md_to_telegram_html(markdown)` — walks `pulldown_cmark::Parser` events and emits Telegram's HTML subset (`<b>`, `<i>`, `<s>`, `<code>`, `<pre>`, `<a>`, `<blockquote>`). Tables → `<pre>` monospace; headings → `<b>`; images → text fallback.
- `chunk_html(html)` — splits into ≤4096-char chunks with **balanced tags**: closes open tags at each split boundary and reopens them in the next chunk.
- `escape_html(text)` — escapes `&` `<` `>` only.

In `handlers.rs`, the send loop attempts `ParseMode::Html` first; if Telegram rejects the HTML (e.g. malformed), it falls back to plain-text chunks via `chunk_message()`. `ERROR_REPLY` is always sent as plain text (no parse mode).

### Provider Factory

`create_provider(config) -> Box<dyn LlmProvider>` in `provider/factory.rs`. API key resolution: **env var > config file** (e.g., `DEEPSEEK_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`). Provider selection by `config.provider` string: `"deepseek"`, `"anthropic"`, `"openai"`. `MockProvider` is test-only and not available through `create_provider()`.

**Adding a new OpenAI-compatible provider:** use `provider/openai_compat.rs` as the shared base (serde types, `build_api_messages`, `complete_request`, `stream_sse`, `SSE_DONE_MARKER`). See `deepseek.rs` and `openai.rs` for thin-wrapper examples (~70 lines each).

### Config Loading

Priority (highest first):
1. `--config <path>` CLI flag (error if file missing)
2. `./config.toml` (local directory)
3. `~/.config/synapse/config.toml` (user default)
4. Error — no silent defaults; exits with a clear message

`Config` top-level fields: `provider`, `api_key`, `model`, `max_tokens: u32` (serde default `4096` via `default_max_tokens()`; passed to every provider call), `system_prompt: Option<String>` (injected on-the-fly, never stored in DB), `system_prompt_file: Option<String>` (path to external prompt file; inline `system_prompt` wins if both set), `session`, `mcp`, `telegram`, `logging: Option<LoggingConfig>`. `TelegramConfig` lives in `synapse-core/src/config.rs` and includes `max_sessions_per_chat: u32` (serde default `10`; manual `impl Default` — not derived — so `TelegramConfig::default()` returns `10` not `0`). Bot token resolution: `TELEGRAM_BOT_TOKEN` env var > `telegram.token` in config. Empty `allowed_users` rejects all users (secure by default). `LoggingConfig` fields: `directory` (default `"logs"`), `max_files` (default `7`), `rotation` (`"daily"` / `"hourly"` / `"never"`, default `"daily"`); omitting `[logging]` keeps stdout-only behavior.

### Storage

SQLite via `sqlx` with WAL mode, connection pooling (max 5), automatic migrations. Database URL priority: `$DATABASE_URL` > `session.database_url` in config > default `sqlite:~/.config/synapse/sessions.db`. Uses UUID v7 (time-sortable) and RFC3339 timestamps. `create_storage(config) -> Box<dyn SessionStore>` factory in `storage/sqlite.rs`.

### Error Types

Four `thiserror` enums in `synapse-core`:
- **`AgentError`** (`agent.rs`): `Provider(ProviderError)`, `Mcp(McpError)`, `MaxIterationsExceeded`
- **`ProviderError`** (`provider.rs`): `ProviderError { message }`, `RequestFailed`, `AuthenticationError`, `MissingApiKey`, `UnknownProvider`
- **`StorageError`** (`storage.rs`): `Database`, `NotFound(Uuid)`, `Migration`, `InvalidData`
- **`ConfigError`** (`config.rs`): `IoError`, `ParseError`, `NotFound`

Interface crates use `anyhow` and `.context()` for error wrapping.

## Code Conventions

### Module System
Use the **new Rust module system** (Rust 2018+). **Never use `mod.rs` files.**

```
# Correct: parent file declares submodules
src/
├── provider.rs        # declares: mod anthropic; mod deepseek;
└── provider/
    ├── anthropic.rs
    └── deepseek.rs
```

### Key Rules (from `docs/conventions.md`)
- Group imports: `std` → external → internal (blank lines between)
- No `unwrap()`/`expect()` in `synapse-core` — propagate with `?`
- No blocking I/O in async functions (use `tokio::time::sleep`, not `std::thread::sleep`)
- `thiserror` in core, `anyhow` in CLI/Telegram
- Test naming: `test_<function>_<scenario>`
- `#[tokio::test]` for async tests
- 100 character line limit

## Workspace Crates

| Crate | Purpose |
|-------|---------|
| `synapse-core` | Core library: agent orchestrator, config, providers, storage, MCP, message types |
| `synapse-cli` | CLI binary: one-shot, stdin, and session modes via `clap`; `-p`/`--provider` flag overrides config provider; REPL split into `repl/app.rs`, `repl/render.rs`, `repl/input.rs`; session commands in `commands.rs` |
| `synapse-telegram` | Telegram bot interface: teloxide long-polling, two-branch dispatcher (messages + callback queries), user allowlist auth, `TelegramConfig`; `format.rs` converts LLM Markdown → Telegram HTML before sending; `commands.rs` implements 7 slash commands (`/start`, `/help`, `/new`, `/history` (last 10 messages, truncated to 150 chars), `/list`, `/switch [N]`, `/delete [N]`) plus `handle_callback` for inline keyboard button taps |

## CI/CD

GitHub Actions on push to `master`/`feature/*` and PRs to `master`:
- **check**: `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test`
- **audit**: `rustsec/audit-check` for vulnerability scanning

## Versioning

Lockstep versioning: all three crates share a single version via `version.workspace = true` in each `[package]` section. The canonical version lives in `[workspace.package]` in the root `Cargo.toml`.

**Scheme (pre-1.0 semver `0.MINOR.PATCH`):**
- `minor` — one bump per completed ticket (the normal case)
- `patch` — bugfixes or docs-only changes
- `major` — reserved for 1.0

**Cutting a release:**
```bash
/release minor   # most common: new ticket shipped
/release patch   # bugfix or docs only
/release major   # 1.0 milestone
```

The `/release` skill (`.claude/skills/release/SKILL.md`) runs pre-release checks, bumps the version in `Cargo.toml`, rotates `CHANGELOG.md` (`[Unreleased]` → `[X.Y.Z] - YYYY-MM-DD`), commits, and tags `vX.Y.Z`. It does **not** push — run `git push && git push --tags` manually after review.

## Key Technology Decisions

- **Rust**: Nightly, Edition 2024, resolver v3
- **Async Runtime**: Tokio (multi-thread)
- **HTTP**: `reqwest` with `json` + `stream` features
- **SSE**: `eventsource-stream` + `async-stream`
- **Database**: `sqlx` with `runtime-tokio` + `sqlite` features
- **CLI**: `clap` for args, `ratatui` + `crossterm` for REPL UI
- **MCP**: `rmcp` for Model Context Protocol
- **Telegram**: `teloxide` 0.17 with `macros` feature, dptree dependency injection; `pulldown-cmark` 0.13 for Markdown→HTML conversion in `format.rs`
- **Tracing**: `tracing` 0.1 in `synapse-core` and `synapse-cli` (structured spans/events); `tracing-appender` 0.2 with non-blocking rolling-file writer in Telegram only. CLI uses plain `EnvFilter::from_default_env()`. Telegram bot always enables `synapse_telegram=info` and `synapse_core=info` on top of `RUST_LOG` via `DEFAULT_DIRECTIVES`.
- **IDs**: `uuid` v4/v7

## Documentation

Essential docs to read before working:
1. `docs/.active_ticket` — current ticket ID
2. `docs/prd/<ticket>.prd.md` — requirements for current feature
3. `docs/tasklist/<ticket>.md` — task breakdown with progress tracking
4. `docs/vision.md` — full technical architecture
5. `docs/conventions.md` — code rules (DO and DON'T)
6. `docs/workflow.md` — step-by-step collaboration process with quality gates

Ticket artifacts live in: `docs/prd/`, `docs/research/`, `docs/plan/`, `docs/tasklist/`, `docs/summary/`, `reports/qa/`.

## Workflow

**Starting a new feature (full automated):**
```
/feature-development SY-<N> @docs/<description>.md
```

**Starting a new feature (manual):**
```
/analysis SY-<N> @docs/<description>.md
```

**Follow `docs/workflow.md` strictly. Three mandatory checkpoints — never skip:**
- "Proceed with this approach?"
- "Ready to commit?"
- "Continue to next task?"

## Completed Tickets

| Ticket | Description | Summary |
|--------|-------------|---------|
| SY-1 | Project Foundation | Workspace structure with 3 crates |
| SY-2 | CI/CD Pipeline | GitHub Actions with check + audit jobs |
| SY-3 | Echo CLI | CLI with clap, one-shot and stdin input modes |
| SY-4 | Configuration | TOML config loading with multi-location priority |
| SY-5 | Provider Abstraction | LlmProvider trait, Message/Role types, MockProvider |
| SY-6 | Anthropic Provider | AnthropicProvider with Claude API, async CLI with tokio |
| SY-7 | DeepSeek Provider | DeepSeekProvider with OpenAI-compatible API, provider factory pattern |
| SY-8 | Streaming Responses | Token-by-token streaming via SSE, DeepSeekProvider streaming, Ctrl+C handling |
| SY-9 | Session Storage | SQLite persistence, SessionStore trait, session commands, auto-cleanup |
| SY-10 | CLI REPL | Interactive TUI with ratatui/crossterm, multi-turn conversations, streaming, session resume |
| SY-11 | MCP Integration | Agent struct with tool call loop, MCP client via rmcp, tool discovery |
| SY-13 | Telegram Bot | teloxide bot, session-per-chat persistence, user allowlist auth, TelegramConfig |
| SY-14 | System Prompt | `system_prompt` in Config and Agent, `build_messages()` on-the-fly injection |
| SY-15 | File Logging | `LoggingConfig` in core, `tracing-appender` layered subscriber in Telegram bot |
| SY-16 | Code Refactoring | Dead code removal, `openai_compat.rs` shared base, magic-string constants, structured tracing, `Agent::from_config()`, `init_mcp_client()` in core, REPL file split, API surface tightened |
| SY-17 | Telegram Markdown Formatting | `format.rs` with `md_to_telegram_html` + `chunk_html`; HTML parse mode with plain-text fallback in handlers |
| SY-18 | Telegram Bot Commands | `max_tokens: u32` in `Config` (serde default 4096); `/help`, `/new`, `/history`, `/list`, `/switch N`, `/delete N` commands; `ChatSessions` multi-session struct; branched dispatcher; `max_sessions_per_chat` config cap |
| SY-19 | Telegram Command Fixes & Interactive Keyboards | Fix `/switch`/`/delete` fall-through (`String` instead of `usize`); `parse_session_arg`; `/start` command; defensive guard in `handle_message`; inline keyboards with `InlineKeyboardMarkup`; `handle_callback` with `do_switch`/`do_delete` shared logic; dispatcher extended to handle `CallbackQuery` |
| SY-20 | Improve /history Command | `/history` now shows last 10 user/assistant messages only (filtered from full history), truncated to 150 chars with `...`; `truncate_content` and `format_history` extracted as pure helpers for testability |
