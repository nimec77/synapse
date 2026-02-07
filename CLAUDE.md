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
   ┌────┼────────────┐
   ▼    ▼            ▼
LlmProvider  SessionStore   (future: McpClient)
 (trait)      (trait)
   │            │
   ▼            ▼
Anthropic    SqliteStore
DeepSeek
Mock
```

**Critical rule:** `synapse-core` never imports from interface crates. Dependencies flow inward only.

### Core Traits

**LlmProvider** (`synapse-core/src/provider.rs`) — the central abstraction:
```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
    fn stream(&self, messages: &[Message])
        -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
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

Providers return `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>`. The `StreamEvent` enum (`provider/streaming.rs`):
- `TextDelta(String)` — token fragment
- `ToolCall { id, name, input }` — future MCP support
- `ToolResult { id, output }` — future MCP support
- `Done` — stream complete
- `Error(ProviderError)`

CLI consumes streams with `tokio::select!` for Ctrl+C handling. Uses `async_stream::stream!` macro and `eventsource-stream` for SSE parsing.

### Provider Factory

`create_provider(config) -> Box<dyn LlmProvider>` in `provider/factory.rs`. API key resolution: **env var > config file** (e.g., `DEEPSEEK_API_KEY`, `ANTHROPIC_API_KEY`). Provider selection by `config.provider` string: `"deepseek"`, `"anthropic"`, `"mock"`.

### Config Loading

Priority (highest first):
1. `$SYNAPSE_CONFIG` env var path
2. `./config.toml` (local directory)
3. `~/.config/synapse/config.toml` (user default)

### Storage

SQLite via `sqlx` with WAL mode, connection pooling (max 5), automatic migrations. Database URL priority: `$DATABASE_URL` > `session.database_url` in config > default `sqlite:~/.config/synapse/sessions.db`. Uses UUID v7 (time-sortable) and RFC3339 timestamps. `create_storage(config) -> Box<dyn SessionStore>` factory in `storage/sqlite.rs`.

### Error Types

Three `thiserror` enums in `synapse-core`:
- **`ProviderError`** (`provider.rs`): `ProviderError`, `RequestFailed`, `AuthenticationError`, `MissingApiKey`, `UnknownProvider`
- **`StorageError`** (`storage.rs`): `Database`, `NotFound(Uuid)`, `Migration`, `InvalidData`
- **`ConfigError`** (`config.rs`): `IoError`, `ParseError`

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
| `synapse-core` | Core library: config, providers, storage, message types |
| `synapse-cli` | CLI binary: one-shot, stdin, and session modes via `clap` |
| `synapse-telegram` | Telegram bot interface (placeholder) |

## CI/CD

GitHub Actions on push to `master`/`feature/*` and PRs to `master`:
- **check**: `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test`
- **audit**: `rustsec/audit-check` for vulnerability scanning

## Key Technology Decisions

- **Rust**: Nightly, Edition 2024, resolver v3
- **Async Runtime**: Tokio (multi-thread)
- **HTTP**: `reqwest` with `json` + `stream` features
- **SSE**: `eventsource-stream` + `async-stream`
- **Database**: `sqlx` with `runtime-tokio` + `sqlite` features
- **CLI**: `clap` for args, `ratatui` + `crossterm` for REPL UI
- **MCP**: `rmcp` for Model Context Protocol (future)
- **IDs**: `uuid` v4/v7

## Documentation

Essential docs to read before working:
1. `docs/.active_ticket` — current ticket ID
2. `docs/prd/<ticket>.prd.md` — requirements for current feature
3. `docs/tasklist.md` — development plan with progress tracking
4. `docs/vision.md` — full technical architecture
5. `docs/conventions.md` — code rules (DO and DON'T)
6. `docs/workflow.md` — step-by-step collaboration process with quality gates

Ticket artifacts live in: `docs/prd/`, `docs/research/`, `docs/plan/`, `docs/tasklist/`, `docs/summary/`, `reports/qa/`.

## Workflow

**Starting a new feature (full automated):**
```
/feature-development SY-<N> "Title" @docs/<description>.md
```

**Starting a new feature (manual):**
```
/analysis SY-<N> "Title" @docs/<description>.md
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
