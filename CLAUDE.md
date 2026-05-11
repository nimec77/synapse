# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Synapse is a Rust-based AI agent that serves as a unified interface to interact with multiple LLM providers (Anthropic Claude, DeepSeek, OpenAI). The project targets multiple interfaces: CLI (primary), Telegram bot, and backend service. LLM providers are implemented from scratch (no rig/genai/async-openai) for learning depth and full control.

## Build Commands

Standard `cargo` applies. Per-crate scoping: `cargo test -p synapse-core` / `cargo test -p synapse-cli`
/ `cargo test -p synapse-telegram`.

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

> ⚠️ **Critical invariant** — `synapse-core` never imports from interface crates. Dependencies flow
> inward only. Any code that needs to live in both CLI and Telegram belongs in `synapse-core`.

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

- **`LlmProvider`** (`synapse-core/src/provider.rs`) — `complete`, `stream`, `complete_with_tools`.
  Default `complete_with_tools` delegates to `complete()` and ignores tools; Anthropic, DeepSeek,
  and OpenAI override it to pass tools via the API. Read the source for the full signature.
- **`SessionStore`** (`synapse-core/src/storage.rs`) — async CRUD over `Session`/`StoredMessage`
  plus `cleanup(&SessionConfig)`. Implemented by `SqliteStore`.
- **`Message`** (`synapse-core/src/message.rs`) — `role`, `content`, `tool_calls`,
  `tool_call_id`, `reasoning_content`. Tool-call history rule below applies.

> ⚠️ **`reasoning_content` tool-call history rule** — When an assistant message has *both*
> `tool_calls.is_some()` and `reasoning_content.is_some()`, `reasoning_content` **MUST** be
> included in the outbound `ApiMessage` on every subsequent API call (DeepSeek thinking-mode
> requirement). `build_api_messages` in `openai_compat.rs` handles this automatically. On
> text-only turns (`tool_calls.is_none()`), reasoning is dropped to save tokens. Violating this
> rule causes mid-conversation 400 errors.

### Streaming

Providers return `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>`. The `StreamEvent` enum (`provider/streaming.rs`) is `#[non_exhaustive]` and has three variants:
- `TextDelta(String)` — incremental fragment of the user-visible answer
- `ReasoningDelta(String)` — incremental fragment of chain-of-thought reasoning (DeepSeek V4 Pro and other reasoning models); renderers display this above/distinct from `TextDelta`
- `Done` — stream complete

`#[non_exhaustive]` was added so future variants (Anthropic/OpenAI reasoning) do not break downstream `match` arms. All `match StreamEvent` sites outside `synapse-core` must include a wildcard arm.

`Agent::stream()` and `Agent::stream_owned()` yield `Result<StreamEvent, AgentError>` (not `ProviderError`). When tools are available, they resolve tool call iterations internally via `complete()`, then yield: `ReasoningDelta(s)` (if reasoning present) → `TextDelta(content)` → `Done`. When no tools are configured, they delegate directly to the provider's `stream()`. CLI consumes streams with `tokio::select!` for Ctrl+C handling. Uses `async_stream::stream!` macro and `eventsource-stream` for SSE parsing. OpenAI-compatible providers (Anthropic, DeepSeek, OpenAI) set `tool_choice: "auto"` in the API request when tools are present.

### Telegram Bot

Dispatcher: under `dptree::entry()`, `filter_message` splits into command vs. non-command branches
(→ `commands::handle_command` / `handlers::handle_message`); `filter_callback_query` →
`commands::handle_callback`. `Me` is injected at startup via `bot.get_me().await?` (required by
`filter_command::<Command>()` to strip `/cmd@botname` suffixes).

**Multi-session per chat** — `ChatSessionMap = Arc<RwLock<HashMap<i64, ChatSessions>>>` where
`ChatSessions { sessions: Vec<Uuid>, active_idx: usize }`. Hot path uses `active_session_id()`;
display/index commands always call `list_sessions()` fresh for stable 1-based ordering.
`max_sessions_per_chat` is enforced only in `/new` (oldest evicted). `rebuild_chat_map` at startup
groups `tg:<chat_id>` sessions with the most recently updated at `active_idx = 0`. Slash commands
never invoke the LLM; `handle_message` returns early for any text starting with `/` to prevent
command fall-through. For command/keyboard internals, see `synapse-telegram/src/commands.rs` (use
`ast-index outline` first).

**Markdown → Telegram pipeline** — LLM output is Markdown; Telegram supports HTML or MarkdownV2.
HTML is used because only `&`, `<`, `>` need escaping (MarkdownV2 requires 18+ characters and is
fragile for LLM output). `synapse-telegram/src/format.rs`:

- `md_to_telegram_html(markdown)` — walks `pulldown_cmark::Parser` events and emits Telegram's
  HTML subset (`<b>`, `<i>`, `<s>`, `<code>`, `<pre>`, `<a>`, `<blockquote>`).
- `chunk_html(html)` — splits into ≤4096-char chunks with **balanced tags** (closes open tags at
  each boundary and reopens them in the next chunk).
- `escape_html(text)` — escapes `&` `<` `>` only.

The send loop in `handlers.rs` attempts `ParseMode::Html` first and falls back to plain text on
rejection. `ERROR_REPLY` is always plain text.

**Reasoning rendering** — streaming accumulates `ReasoningDelta` events into a separate buffer
(never sent inline). On stream completion, if `telegram.show_reasoning == true`, the reasoning is
truncated to `TELEGRAM_REASONING_PREVIEW_MAX_CHARS = 1500` via `synapse_core::text::truncate` and
prepended as `<blockquote>{escaped_reasoning}</blockquote>\n\n` before the first answer chunk.
Reasoning is **always** persisted to SQLite regardless of the flag (required for DeepSeek
tool-call history rule).

### Provider Factory

`create_provider(config) -> Box<dyn LlmProvider>` in `provider/factory.rs`. API key resolution: **env var > config file** (e.g., `DEEPSEEK_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`). Provider selection by `config.provider` string: `"deepseek"`, `"anthropic"`, `"openai"`. `MockProvider` is test-only and not available through `create_provider()`.

**Adding a new OpenAI-compatible provider:** use `provider/openai_compat.rs` as the shared base (serde types, `build_api_messages`, `complete_request`, `stream_sse`, `SSE_DONE_MARKER`). See `deepseek.rs` and `openai.rs` for thin-wrapper examples (~70 lines each).

#### Reasoning Models

`REASONING_MODELS` (`deepseek.rs`) lists DeepSeek model identifiers that support thinking mode (currently `["deepseek-v4-pro"]`). When `config.model` is in this list and `config.provider == "deepseek"`, the factory configures `OpenAiCompatProvider` with `ReasoningSettings` — every request body then includes `thinking: {"type": "enabled"}` and `reasoning_effort: <effort>`. Non-reasoning models and non-DeepSeek providers are never sent these fields.

**Effort resolution** (`resolve_reasoning_effort` in `factory.rs`):
1. `config.reasoning_effort = Some(v)` → use `v` (explicit always wins)
2. `config.mcp.is_some()` AND no explicit → `"max"` (auto-escalated for complex tool flows)
3. Otherwise → `"high"` (safe default per DeepSeek docs)

The factory emits `tracing::info!(model, effort, auto_escalated, "factory: enabling DeepSeek thinking mode")` when reasoning is activated, and `tracing::warn!` when `reasoning_effort` is set for a non-DeepSeek provider (forward-compat warning, not an error).

Reference: `docs/research/SY-22-deepseek-v4-pro.md`.

### Config Loading

Priority (highest first):
1. `--config <path>` CLI flag (error if file missing)
2. `./config.toml`
3. `~/.config/synapse/config.toml`
4. Error — no silent defaults

Load-bearing resolution rules:
- API key: env var (`DEEPSEEK_API_KEY` / `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`) > `api_key` in config.
- Bot token: `TELEGRAM_BOT_TOKEN` env > `telegram.token` in config.
- `system_prompt` (inline) wins over `system_prompt_file` when both are set.
- `telegram.allowed_users = []` rejects **all** users (secure by default).
- `TelegramConfig::default()` uses a manual `impl Default` (not derived) so the type-level default
  (`max_sessions_per_chat = 10`) matches the serde default.

Field-by-field reference: `docs/config-reference.md`.

### Storage

SQLite via `sqlx` with WAL mode, connection pooling (max 5), automatic migrations. Database URL priority: `$DATABASE_URL` > `session.database_url` in config > default `sqlite:~/.config/synapse/sessions.db`. Uses UUID v7 (time-sortable) and RFC3339 timestamps. `create_storage(config) -> Box<dyn SessionStore>` factory in `storage/sqlite.rs`.

Migrations live in `synapse-core/migrations/` with timestamp-prefixed filenames. Current set:
`20250125_001_initial.sql`, `20260208_002_add_tool_columns.sql`,
`20260226_003_drop_system_prompt.sql`, `20260510_004_add_reasoning_content.sql`.

**Migration rule:** prefer nullable column adds. Column drops/renames are only acceptable when the
field is fully unused — migration 003 (`drop_system_prompt`) is the documented precedent: the
column was dead after SY-14 moved system prompts to runtime injection via `Agent::build_messages`.
`StoredMessage` (`session.rs`) includes `reasoning_content: Option<String>` (added in migration
004) which round-trips with `Message.reasoning_content`.

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

- `synapse-core` — library (traits + adapters; depends inward only).
- `synapse-cli` — `synapse` binary; `-p`/`--provider` overrides `config.provider` at runtime.
- `synapse-telegram` — `synapse-telegram` binary; teloxide long-polling.

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

Toolchain is Rust nightly, Edition 2024, resolver v3 (pinned via `rust-toolchain.toml`). Most crate
choices are unsurprising (see `Cargo.toml`). Non-obvious ones:

- **MCP**: `rmcp` (the Rust MCP SDK).
- **Telegram Markdown→HTML**: `pulldown-cmark` 0.13 in `synapse-telegram/src/format.rs`.
- **Tracing split**: CLI uses plain `EnvFilter::from_default_env()`. Telegram bot adds
  `tracing-appender` 0.2 (non-blocking rolling-file writer) and always enables
  `synapse_telegram=info` + `synapse_core=info` on top of `RUST_LOG` via `DEFAULT_DIRECTIVES`.

## Documentation

Essential docs to read before working:
1. `docs/.active_ticket` — current ticket ID
2. `docs/prd/<ticket>.prd.md` — requirements for current feature
3. `docs/tasklist/<ticket>.md` — task breakdown with progress tracking
4. `docs/vision.md` — full technical architecture
5. `docs/conventions.md` — code rules (DO and DON'T)
6. `docs/workflow.md` — step-by-step collaboration process with quality gates

Per-ticket artifacts: `docs/prd/`, `docs/research/`, `docs/plan/`, `docs/tasklist/`,
`docs/phase/`, `docs/summary/`, `reports/qa/`. Operational docs: `docs/deploy.md`,
`docs/relocate-synapse.md`, `docs/releases/`. Ticket history index: `docs/tickets.md`.

## Workflow

Follow `docs/workflow.md`. Start a new feature with `/feature-development SY-<N> @docs/<file>.md`
(automated) or `/analysis SY-<N> @docs/<file>.md` (manual). **Three mandatory checkpoints —
never skip:** *Proceed with this approach?*, *Ready to commit?*, *Continue to next task?*

## Ticket History

Short-form lookup table: `docs/tickets.md`. Detailed release notes: `CHANGELOG.md`. Active ticket
ID: `docs/.active_ticket`. The most recent ticket (SY-22, DeepSeek V4 Pro reasoning) is merged on
`master` but unreleased — see `[Unreleased]` in `CHANGELOG.md`; workspace version is still
`0.21.3`.
