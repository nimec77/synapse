# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.20.0] - 2026-02-26

### Changed

- **SY-20: Improve /history Command** — Replaces the unbounded full message dump with a compact
  "last 10 messages" view in the Telegram bot `/history` command:
  - `format_history(messages: &[StoredMessage]) -> String` pure helper added: filters to
    `Role::User` and `Role::Assistant` only (discarding `System` and `Tool` messages), takes the
    chronologically last 10 messages via `saturating_sub(10)` skip, and formats each as
    `[role_label] timestamp\ncontent\n\n`
  - `truncate_content(content: &str, max_chars: usize) -> String` pure helper added: truncates
    to `max_chars` Unicode scalar values (`.chars().take(max_chars)`) and appends `...`; content
    at or below the limit is returned unchanged; boundary case (exactly 150 chars) returns no
    ellipsis
  - `cmd_history` refactored to delegate formatting to `format_history`; sessions containing
    only `System`/`Tool` messages now correctly reply "No messages in current session."
  - `Command::History` description changed from `"Show conversation history"` to
    `"Show recent messages"` so `/help` output accurately reflects the new behavior
  - 11 new unit tests added covering all PRD scenarios: 5 for `truncate_content` (short, exact
    limit, over limit, long, empty) and 6 for `format_history` (role filtering, User/Assistant
    passthrough, last-10 limit selection, fewer-than-10, empty, truncation)
  - All changes confined to `synapse-telegram/src/commands.rs`; `synapse-core` and `synapse-cli`
    untouched; no new crate dependencies; 79 total `synapse-telegram` tests passing

## [0.19.0] - 2026-02-26

### Added

- **SY-19: Telegram Command Fixes & Interactive Keyboards** — Fixes the command fall-through bug
  where `/switch` and `/delete` without arguments were silently forwarded to the LLM; adds a
  `/start` welcome command, a defensive guard in `handle_message`, and interactive inline keyboards
  for argumentless session selection:
  - `Switch(usize)` and `Delete(usize)` in the `Command` enum changed to `Switch(String)` and
    `Delete(String)` so `BotCommands::parse` always succeeds for argumentless invocations,
    eliminating the root cause of the fall-through bug
  - `parse_session_arg(arg: &str) -> Result<Option<usize>, String>` helper added to triage the
    argument string: empty/whitespace → `Ok(None)` (show keyboard), numeric → `Ok(Some(n))`
    (direct execution), non-numeric non-empty → `Err(hint)` (send error reply)
  - `Start` variant added to `Command` enum with description `"Start the bot"`; `cmd_start`
    sends the standard Telegram bot welcome message; total registered commands is now seven
  - Defensive guard added to `handle_message` before any session resolution or LLM call:
    messages with text starting with `/` receive a hint reply and return early, preventing
    any slash-prefixed text from reaching the LLM regardless of `filter_command` parse outcome
  - `fetch_chat_sessions(chat_id, storage, chat_map)` shared helper introduced to avoid
    duplicating the session-fetch-and-filter pattern across `do_switch`, `do_delete`, and the
    two keyboard functions
  - `build_session_keyboard(action, sessions, active_id) -> InlineKeyboardMarkup` builds an
    inline keyboard with one button per session; callback data format `"action:N"` (1-based);
    active session marked with `*`; preview text capped at 20 chars
  - `cmd_switch_keyboard` and `cmd_delete_keyboard` fetch sessions and send the keyboard with
    `"Select a session to switch to:"` / `"Select a session to delete:"` prompts; plain-text
    hint sent if the chat has no sessions
  - `do_switch(n, chat_id, storage, chat_map) -> Result<String, String>` and
    `do_delete(n, chat_id, config, storage, chat_map) -> Result<String, String>` extracted as
    shared logic functions; both re-fetch the session list from the DB on every call to handle
    staleness between keyboard display and button tap
  - `parse_callback_data(data: &str) -> Option<(&str, usize)>` parses `"switch:N"` /
    `"delete:N"` callback data strings
  - `pub handle_callback(bot, q, config, storage, chat_map) -> ResponseResult<()>` endpoint for
    `CallbackQuery` updates: (1) authorization check — silent drop for unauthorized users; (2)
    `bot.answer_callback_query(q.id.clone())` called immediately to dismiss the loading spinner
    before DB operations; (3) parse callback data; (4) execute `do_switch` / `do_delete`;
    (5) `bot.edit_message_text` replaces the keyboard message with plain result text, preventing
    double-tap; edit failures logged at `warn` but not propagated
  - Dispatcher restructured from `Update::filter_message()` only to `dptree::entry()` with two
    branches: the existing message branch and a new `Update::filter_callback_query()` branch
    routing to `handle_callback`; no new `dptree::deps![]` entries required
  - 25 new unit tests (14 required from Task 7 covering `parse_session_arg`, `parse_callback_data`,
    `build_session_keyboard`, and the defensive guard condition; plus 11 covering `do_delete`
    `active_idx` adjustment edge cases); 68 total `synapse-telegram` tests passing
  - All changes confined to `synapse-telegram`; `synapse-core` and `synapse-cli` untouched;
    no new crate dependencies (all new types from existing `teloxide 0.17`)

## [0.18.0] - 2026-02-25

### Added

- **SY-18: Telegram Bot Commands** - Slash command interface with multi-session management per
  Telegram chat and configurable session cap:
  - Six slash commands registered with Telegram at startup via `set_my_commands` (autocomplete
    in Telegram clients): `/help`, `/new`, `/history`, `/list`, `/switch N`, `/delete N`
  - `ChatSessions` struct added to `synapse-telegram/src/handlers.rs` replacing the previous
    single-UUID-per-chat map; tracks `sessions: Vec<Uuid>` and `active_idx: usize`; the
    `active_session_id()` method provides O(1) lookup for the common case (sending messages)
  - `ChatSessionMap` type alias updated to `Arc<RwLock<HashMap<i64, ChatSessions>>>`
  - `resolve_session` fast and slow paths updated to work with the new `ChatSessions` type;
    double-check pattern preserved for race-safe session creation
  - `max_sessions_per_chat: u32` field added to `TelegramConfig` in
    `synapse-core/src/config.rs` with `#[serde(default = "default_max_sessions_per_chat")]`
    (default `10`); `#[derive(Default)]` replaced with a manual `impl Default` that calls the
    serde default function to keep both defaults in sync
  - `synapse-telegram/src/commands.rs` (new file): `Command` enum with
    `#[derive(BotCommands, Clone)]`; `handle_command` entry point with authorization check;
    six private `cmd_*` functions; `/new` enforces the cap and evicts the oldest session when
    exceeded; `/history` formats messages with `chrono` timestamps and role labels; `/list`
    and `/switch`/`/delete` use `list_sessions()` (`ORDER BY updated_at DESC`) as the source
    of truth for display ordering, ensuring indexes are always consistent across commands
  - Branched teloxide dispatcher: `filter_command::<Command>()` routes to `handle_command`;
    non-command messages fall through to `handle_message` (backward compatible)
  - `bot.get_me()` called at startup; resulting `Me` type injected into `dptree::deps![]` for
    `filter_command` to strip the bot's username suffix from commands
  - `rebuild_chat_map` rewritten to return `HashMap<i64, ChatSessions>`, grouping all
    `tg:<chat_id>` sessions per chat; most recently updated session (index 0 in the DESC-ordered
    result) becomes the active session on startup
  - `chrono = "0.4"` promoted from `[dev-dependencies]` to `[dependencies]` in
    `synapse-telegram/Cargo.toml` for runtime timestamp formatting in `/history`
  - `config.example.toml` updated with a commented-out `max_sessions_per_chat = 10` entry and
    two-line explanatory comment in the `[telegram]` section
  - 12 new unit tests: 11 in `commands.rs` (`ChatSessions::active_session_id`, session cap
    logic, index validation, `/delete` `active_idx` adjustment) and 1 in `main.rs` for
    multi-session grouping in `rebuild_chat_map`; 265 total unit + 13 doc tests passing

### Changed

- **SY-17: Configurable max_tokens** - Token limit for LLM responses is now user-configurable
  via `max_tokens` in `config.toml`; default raised from 1024 to 4096 to prevent silent
  response truncation:
  - `max_tokens: u32` field added to `Config` in `synapse-core/src/config.rs` with
    `#[serde(default = "default_max_tokens")]`; default value `4096` applied automatically via
    serde when the field is absent from `config.toml` — fully backward compatible
  - `default_max_tokens() -> u32` function added returning `4096`; `Config::default()` updated
  - All three provider constructors (`AnthropicProvider::new`, `DeepSeekProvider::new`,
    `OpenAiProvider::new`) extended with a third `max_tokens: u32` parameter; each struct gains
    a `max_tokens: u32` field
  - All four `LlmProvider` trait methods that build API request bodies (`complete`,
    `complete_with_tools`, `stream`, `stream_with_tools`) now use `self.max_tokens` instead of
    the removed `DEFAULT_MAX_TOKENS = 1024` constant
  - `pub(super) const DEFAULT_MAX_TOKENS: u32 = 1024` removed from `openai_compat.rs`; the
    constant is no longer needed and its removal was verified with `cargo clippy -- -D warnings`
  - `create_provider()` in `factory.rs` passes `config.max_tokens` to all three provider
    constructors, completing the end-to-end data flow from config file to API request body
  - `config.example.toml` updated with a commented-out `max_tokens = 4096` entry and
    explanatory two-line comment immediately after the `model` field
  - `test_config_default_max_tokens` unit test added: verifies `4096` default on empty TOML
    and correct parsing of an explicit `max_tokens = 8192` value
  - 168 unit tests + 13 doc tests passing; zero regressions

### Added

- **SY-17: Telegram Markdown Formatting** - LLM Markdown responses are now rendered as formatted
  text in Telegram instead of raw symbols:
  - `synapse-telegram/src/format.rs` module (new) with three public functions:
    - `md_to_telegram_html(markdown: &str) -> String` — walks `pulldown_cmark::Parser` events and
      emits Telegram's HTML subset: `<b>`, `<i>`, `<s>`, `<code>`, `<pre><code>` (with optional
      `class="language-{lang}"`), `<a href>`, `<blockquote>`; headings → `<b>`; tables →
      `<pre>` monospace; images → `[image: title](url)` text fallback; task list markers preserved
      as `[x]`/`[ ]`; HTML entities in raw text escaped via `escape_html()`
    - `chunk_html(html: &str) -> Vec<String>` — splits HTML into ≤4096-character chunks with
      **balanced tags**: open tags are closed at each split boundary and reopened at the start of
      the next chunk; uses a simulate-then-adjust approach so that closing tags appended at the
      boundary never push a chunk over the limit; split priority: `\n\n` > `\n` > space > hard
      split; never splits inside an `<...>` tag
    - `escape_html(text: &str) -> String` — escapes `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`
  - `pulldown-cmark = { version = "0.13", default-features = false }` added to
    `synapse-telegram/Cargo.toml`
  - `handlers.rs` send loop updated: attempts `ParseMode::Html` (Telegram HTML mode) for each
    chunk; on any Telegram rejection falls back to plain-text chunks via the existing
    `chunk_message()`; `ERROR_REPLY` continues to be sent as plain text (no parse mode)
  - Design rationale: HTML chosen over MarkdownV2 because HTML only requires escaping 3 characters;
    MarkdownV2 requires escaping 18+ characters outside entities and is fragile for LLM output
  - 25 unit tests across all three functions: `escape_html` (4), `md_to_telegram_html` (16),
    `chunk_html` (5); 249 total tests passing

### Changed

- **SY-16: Phase 15 Code Refactoring** - Internal quality pass across the entire workspace with zero
  external behaviour changes:
  - Dead code removed: `placeholder` module deleted from `synapse-core`; `StreamEvent` reduced to
    `TextDelta(String)` and `Done` (removed unused `ToolCall`, `ToolResult`, `Error` variants)
  - OpenAI-compatible provider logic centralised: `synapse-core/src/provider/openai_compat.rs`
    (754 lines) now holds all shared serde types and functions; `deepseek.rs` reduced from 840 to
    118 lines; `openai.rs` reduced from 678 to 96 lines; both are thin wrappers delegating to
    `openai_compat`
  - Magic strings replaced with typed constants and methods: `Role::as_str()` and
    `impl FromStr for Role` added to `synapse-core`; `SSE_DONE_MARKER`, `DEFAULT_MAX_TOKENS`,
    `ERROR_REPLY`, `DEFAULT_TRACING_DIRECTIVE`, `DEEPSEEK_API_KEY_ENV`, `ANTHROPIC_API_KEY_ENV`,
    `OPENAI_API_KEY_ENV` constants extracted to their respective modules
  - Structured tracing added to `synapse-core` and `synapse-cli`: `tracing = "0.1"` added to
    both crates; agent tool loop, HTTP requests, provider factory, SQLite CRUD, config path
    resolution, and MCP operations are instrumented at `debug!`/`info!` level; all `eprintln!`
    calls in core and CLI function bodies replaced with `tracing::warn!`/`tracing::info!`; tracing
    is silent by default unless `RUST_LOG` is set
  - Shared utilities extracted to `synapse-core`: `init_mcp_client()` has a single canonical
    definition in `synapse-core/src/mcp.rs` (removed from CLI and Telegram main files);
    `Agent::from_config(config, mcp_client)` factory method encapsulates provider creation and
    system prompt wiring; used at all three call sites (CLI one-shot, CLI REPL, Telegram)
  - Large files split: `repl.rs` (1168 lines) split into `repl/app.rs` (state), `repl/render.rs`
    (TUI layout), `repl/input.rs` (key handling), and `repl.rs` orchestrator (274 lines); no
    `mod.rs` files created; `commands.rs` extracted from `main.rs` containing `Commands`,
    `SessionAction`, `handle_command`, `truncate`
  - Public API surface tightened: 17 re-exports removed from `synapse-core/src/lib.rs` that no
    consumer crate imports directly (concrete provider types, internal error types, internal config
    types); 17 essential items kept; removed items remain accessible via full module paths
  - Async I/O fix: `std::fs::create_dir_all()` in `sqlite.rs` replaced with
    `tokio::fs::create_dir_all().await`; `fs` feature added to tokio in `synapse-core/Cargo.toml`

### Added

- **SY-15: File Logging** - File-based logging with automatic rotation for the Telegram bot:
  - `LoggingConfig` struct added to `synapse-core/src/config.rs` with three serde-defaulted fields: `directory: String` (default `"logs"`), `max_files: usize` (default `7`), `rotation: String` (default `"daily"`)
  - `logging: Option<LoggingConfig>` field added to `Config` with `#[serde(default)]`; omitting the `[logging]` section preserves existing stdout-only behavior (fully backward compatible)
  - `LoggingConfig` re-exported from `synapse-core/src/lib.rs` alongside existing config types
  - `tracing-appender = "0.2"` added to `synapse-telegram`; `"registry"` feature enabled on `tracing-subscriber` to support layered subscribers
  - `init_tracing(config: &Config) -> anyhow::Result<Option<WorkerGuard>>` helper in `synapse-telegram/src/main.rs`: stdout-only path when `logging` is `None`; stdout + rolling file path when `Some`, using `RollingFileAppender::builder()` with `.filename_prefix("synapse-telegram")` and `.max_log_files()`
  - Config loading moved before tracing initialization in `main()` so `config.logging` is available during setup
  - Non-blocking writer guard (`WorkerGuard`) stored as `let _guard` in `main()` before the dispatcher loop, guaranteeing all buffered log writes are flushed on shutdown
  - Directory creation failure falls back to stdout-only with a stderr warning; unknown `rotation` values fall back to `"daily"` with a warning
  - File `fmt` layer uses `.with_ansi(false)` to suppress ANSI escape codes in log files
  - `system_prompt_file: Option<String>` field added to `Config`: loads system prompt from an external file during `Config::load_from()`; inline `system_prompt` takes priority; whitespace-only file contents treated as absent
  - `config.example.toml` updated with documented `[logging]` section and `system_prompt_file` field
  - `synapse-cli` completely unaffected — stdout only, no changes
  - 13 new unit tests (5 for `LoggingConfig` parsing/defaults, 8 for `system_prompt_file` resolution); 244 total tests passing, zero regressions

- **SY-14: System Prompt** - Global system prompt configuration wired through Agent to all provider calls:
  - `system_prompt: Option<String>` field added to `Config` struct in `synapse-core/src/config.rs` with `#[serde(default)]`; fully backward-compatible — existing config files without the field default to `None`
  - `system_prompt` private field and `with_system_prompt(prompt: impl Into<String>) -> Self` builder method added to `Agent` in `synapse-core/src/agent.rs`; `Agent::new()` signature unchanged
  - `build_messages()` private helper on `Agent` prepends `Message::new(Role::System, prompt)` to a fresh `Vec<Message>` on every provider call; original messages slice is never mutated, ensuring system messages are never stored in the database
  - Integration into `Agent::complete()`, `stream()`, and `stream_owned()`: system prompt is prepended on every iteration of the tool call loop and on every streaming call
  - CLI one-shot (`synapse-cli/src/main.rs`) and REPL (`synapse-cli/src/repl.rs`) modes apply `with_system_prompt()` conditionally at agent construction when `config.system_prompt` is `Some`
  - Telegram bot (`synapse-telegram/src/main.rs`) applies `with_system_prompt()` before wrapping in `Arc`, consistent with CLI behavior
  - `config.example.toml` updated with commented-out `system_prompt` example and guidance to keep prompts concise to minimize token usage
  - No provider changes needed: Anthropic, DeepSeek, OpenAI, and Mock providers already handled `Role::System` correctly
  - No DB migrations: `system_prompt TEXT` column already existed on `sessions` table; prompt is injected at runtime, not persisted per-call
  - 10 new unit tests (3 config + 7 agent); 231 total tests passing, zero regressions

- **SY-13: Telegram Bot** - Second interface proving hexagonal architecture with session-per-chat persistence and user authorization:
  - `TelegramConfig` struct in `synapse-core/src/config.rs` with `token: Option<String>` and `allowed_users: Vec<u64>` fields; added as `pub telegram: Option<TelegramConfig>` to `Config` with `#[serde(default)]` (backward-compatible)
  - `synapse-telegram` crate brought from empty placeholder to fully functional Telegram bot using `teloxide` 0.13
  - Bot token resolution: `TELEGRAM_BOT_TOKEN` env var > `telegram.token` in config; token never logged at any level
  - User authorization via `is_authorized()` helper: checks message sender against `allowed_users`; empty list rejects all (secure by default); unauthorized messages silently dropped
  - Session-per-chat persistence: each Telegram chat ID mapped to a unique SQLite session named `"tg:<chat_id>"`; in-memory `ChatSessionMap` (`Arc<RwLock<HashMap<i64, Uuid>>>`) rebuilt from `list_sessions()` on startup
  - `resolve_session()` with read-lock fast path and write-lock double-check for race-safe session creation
  - `rebuild_chat_map()` reconstructs routing map from existing sessions on bot restart
  - `chunk_message()` splits responses at paragraph / newline / space boundaries for Telegram's 4096-character message limit
  - `handle_message()` endpoint using `dptree` dependency injection: authorization → session → history → typing indicator → `agent.complete()` → store → send
  - Typing indicator (`ChatAction::Typing`) sent before LLM invocation
  - Agent errors logged via `tracing::error!`; generic user-friendly message sent on failure
  - Graceful shutdown via `Arc::try_unwrap(agent)` after Dispatcher stops; Ctrl+C handled via `enable_ctrlc_handler()`
  - `config.example.toml` updated with commented-out `[telegram]` section and usage instructions
  - 17 new unit tests: TelegramConfig parsing (4), user authorization (3), token resolution (4), chat map reconstruction (3), message chunking (3)
  - Zero core abstractions added for Telegram; entire implementation uses existing `Agent::complete()`, `SessionStore`, `Config`, and provider/MCP APIs — hexagonal architecture validated
  - Dependencies: `teloxide` 0.13 (macros), `tracing-subscriber` (env-filter), `async-trait`

- **SY-12: MCP Integration** - Tool calling via Model Context Protocol with agent orchestration:
  - MCP client infrastructure using `rmcp` crate with `TokioChildProcess` transport for stdio-based MCP servers
  - `McpConfig` / `McpServerConfig` types parsing standard `mcp_servers.json` format (compatible with Claude Desktop / Windsurf)
  - Config path resolution: `SYNAPSE_MCP_CONFIG` env var > `~/.config/synapse/mcp_servers.json`
  - Tool discovery via `list_tools()` and unified tool registry mapping tool names to servers
  - `ToolDefinition` provider-agnostic tool schema with per-provider serialization (Anthropic `input_schema`, OpenAI/DeepSeek `function.parameters`)
  - `LlmProvider` trait extended with `complete_with_tools()` and `stream_with_tools()` (backward-compatible default implementations)
  - Anthropic provider: native `tool_use` content blocks, `Role::Tool` translated to `user` role with `tool_result` blocks
  - OpenAI provider: function calling format with streaming tool call delta accumulation
  - DeepSeek provider: OpenAI-compatible tool calling format
  - `Agent` orchestrator implementing detect-execute-return tool call loop (max 10 iterations safety limit)
  - `AgentError` enum wrapping `ProviderError`, `McpError`, and `MaxIterationsExceeded`
  - `Role::Tool` variant added to `Role` enum; `ToolCallData` struct for tool call metadata
  - `Message` extended with `tool_calls` and `tool_call_id` optional fields (backward compatible)
  - `StoredMessage` extended with `tool_calls` and `tool_results` JSON text columns
  - Database migration adding `tool_calls` and `tool_results` nullable columns to `messages` table
  - `McpError` enum with `ConfigError`, `ConnectionError`, `ToolError`, `IoError` variants
  - `MockProvider::with_tool_call_response()` builder for testing agent loop without real providers
  - CLI one-shot and REPL modes integrated with Agent wrapper; `Role::Tool` displayed as `[TOOL]`
  - Graceful degradation: without `mcp_servers.json`, behavior identical to pre-MCP
  - 48 new tests (data model, config, client, all 3 providers, mock, agent, storage, CLI)
  - Dependencies: `rmcp` 0.14 (client, transport-child-process, transport-io), tokio `process` feature

- **SY-11: OpenAI Provider** - OpenAI Chat Completions API support with runtime provider override:
  - `OpenAiProvider` implementing `LlmProvider` trait for OpenAI's Chat Completions API (`complete()` and `stream()`)
  - Provider factory updated: `"openai"` recognized with `OPENAI_API_KEY` env var resolution
  - `-p` / `--provider` CLI flag to override configured provider at runtime (e.g., `synapse -p openai "Hello"`)
  - CLI flag works across all modes: one-shot, stdin, REPL, and session resume
  - Full SSE streaming support with token-by-token rendering, identical to DeepSeek
  - HTTP 401 mapped to `AuthenticationError`, missing key to `MissingApiKey` with clear guidance
  - `OpenAiProvider` publicly exported from `synapse-core` for external crate use
  - 16 new tests (10 provider unit + 2 factory + 4 CLI flag parsing)
  - No new dependencies: OpenAI wire format is identical to DeepSeek (same crates reused)

- **SY-10: CLI REPL** - Interactive terminal UI for multi-turn conversations:
  - `synapse --repl` / `synapse -r` enters interactive REPL mode with a `ratatui` + `crossterm` TUI
  - Three-area vertical layout: scrollable conversation history, input area with cursor, status bar
  - Streaming responses render token-by-token in the conversation history
  - Full input editing: character insert, backspace, cursor movement (left/right/home/end)
  - History scrolling: up/down (line), page up/page down (page)
  - Session persistence: all messages stored to SQLite during REPL conversation
  - Session resume: `synapse --repl --session <uuid>` loads and continues a previous conversation
  - Session ID printed to stderr on exit for future resumption
  - `/quit` command and Ctrl+C for clean exit
  - `TerminalGuard` with `Drop` implementation ensures terminal state restoration on all exit paths
  - 29 new tests (25 REPL logic + 4 CLI flag parsing)
  - Dependencies: `ratatui` 0.30.0, `crossterm` 0.29.0 (with `event-stream` feature)

- **SY-9: Session Storage** - Persistent conversation storage using SQLite:
  - `Session`, `SessionSummary`, `StoredMessage` types in `synapse-core/src/session.rs`
  - `SessionStore` trait defining storage abstraction with CRUD operations
  - `SqliteStore` implementation with connection pooling and WAL mode
  - `StorageError` enum with `Database`, `NotFound`, `Migration`, `InvalidData` variants
  - `CleanupResult` struct tracking deleted sessions by limit and retention
  - Database migrations in `synapse-core/migrations/20250125_001_initial.sql`
  - `sessions` and `messages` tables with indexes and CASCADE delete
  - UUID v7 for time-sortable session and message identifiers
  - `SessionConfig` with `database_url`, `max_sessions` (default 100), `retention_days` (default 90), `auto_cleanup` (default true)
  - Database URL resolution priority: `DATABASE_URL` env var > config.toml > default path
  - CLI session commands: `synapse sessions list`, `synapse sessions show <uuid>`, `synapse sessions delete <uuid>`
  - Continue session: `synapse --session <uuid> "message"` or `synapse -s <uuid> "message"`
  - Auto-cleanup on startup when `auto_cleanup: true`
  - `create_storage()` factory function for storage initialization
  - 30+ unit tests for session, storage, and SQLite operations
  - Dependencies: sqlx (sqlite, runtime-tokio), uuid (v7, serde), chrono (serde), async-trait, dirs

- **SY-8: Streaming Responses** - Token-by-token output for real-time response display:
  - `StreamEvent` enum with `TextDelta`, `ToolCall`, `ToolResult`, `Done`, `Error` variants
  - `stream()` method added to `LlmProvider` trait with object-safe return type
  - DeepSeek SSE streaming via `eventsource-stream` and `async_stream` crates
  - CLI prints tokens progressively with `print!()` and `stdout.flush()`
  - Graceful Ctrl+C interruption via `tokio::select!` with `tokio::signal::ctrl_c()`
  - `[Interrupted]` message on Ctrl+C, clean exit
  - AnthropicProvider fallback streaming (wraps `complete()` for non-progressive output)
  - MockProvider `with_stream_tokens()` for testing streaming behavior
  - 12 unit tests for StreamEvent, SSE parsing, and provider streaming
  - Dependencies: `eventsource-stream = "0.2"`, `async-stream = "0.3"`, `futures = "0.3"`, reqwest `stream` feature, tokio `signal` and `io-std` features

- **SY-7: DeepSeek Provider** - Default LLM provider with provider factory:
  - `DeepSeekProvider` implementing `LlmProvider` trait for DeepSeek's OpenAI-compatible Chat Completions API
  - Provider factory pattern with `create_provider(config)` for dynamic provider selection
  - API key resolution with environment variable priority (`DEEPSEEK_API_KEY` > config file)
  - `MissingApiKey` and `UnknownProvider` error variants added to `ProviderError`
  - System messages included in messages array (OpenAI format, not separate field)
  - Authorization via Bearer token header
  - Default provider changed from hardcoded Anthropic to configuration-based selection
  - CLI now uses factory to create provider based on `config.provider` setting
  - 13 unit tests for DeepSeekProvider (5) and provider factory (8)
  - Support for provider switching: `provider = "deepseek"` or `provider = "anthropic"` in config

- **SY-6: Anthropic Provider** - Real Claude API integration:
  - `AnthropicProvider` implementing `LlmProvider` trait for Anthropic Messages API
  - HTTP client via `reqwest` with JSON serialization for API requests
  - System message extraction to separate `system` field in API request
  - `AuthenticationError` variant added to `ProviderError` for 401 responses
  - API version pinned to `2023-06-01` for stability
  - CLI now sends messages to Claude and displays real responses
  - API key validation with clear error message if missing
  - Support for both one-shot (`synapse "msg"`) and piped input modes
  - 8 unit tests for request/response serialization and message handling
  - Dependencies: reqwest (json), serde_json in synapse-core; tokio (rt-multi-thread), anyhow in synapse-cli

- **SY-5: Provider Abstraction** - LLM provider abstraction layer:
  - `Role` enum with `System`, `User`, `Assistant` variants for conversation roles
  - `Message` struct with role and content fields for conversation messages
  - `LlmProvider` trait with async `complete()` method as the provider contract
  - `MockProvider` for testing with configurable LIFO responses
  - `ProviderError` enum with `ProviderError` and `RequestFailed` variants
  - Object-safe, thread-safe trait design (`Send + Sync` bounds)
  - Dependencies: tokio (rt, macros), async-trait in synapse-core

- **SY-4: Configuration System** - TOML-based configuration loading:
  - `Config` struct with `provider`, `api_key`, and `model` fields
  - Priority-based config loading: `SYNAPSE_CONFIG` env var > `./config.toml` > `~/.config/synapse/config.toml` > defaults
  - Default values: provider = "deepseek", model = "deepseek-chat"
  - `ConfigError` with `IoError` and `ParseError` variants
  - CLI displays configured provider on startup
  - `config.example.toml` with documented options
  - Dependencies: toml, serde, dirs, thiserror in synapse-core
  - synapse-cli now depends on synapse-core

- **SY-3: Echo CLI** - CLI argument parsing with clap:
  - One-shot mode: `synapse "message"` prints `Echo: message`
  - Stdin mode: `echo "message" | synapse` reads from pipe
  - TTY detection shows help when no input provided
  - `--help` and `--version` flags

- **SY-2: CI/CD Pipeline** - GitHub Actions workflow for automated quality checks:
  - Format check (`cargo fmt --check`)
  - Linting with warnings as errors (`cargo clippy -- -D warnings`)
  - Test execution (`cargo test`)
  - Security audit via `rustsec/audit-check`
  - Dependency caching with `Swatinem/rust-cache`
  - Triggers on push to `master`/`feature/*` and PRs to `master`
  - `rust-toolchain.toml` for consistent nightly toolchain

- **SY-1: Project Foundation** - Established Rust workspace with three crates:
  - `synapse-core`: Core library for agent logic, providers, storage, and MCP
  - `synapse-cli`: CLI binary (executable: `synapse`)
  - `synapse-telegram`: Telegram bot binary
  - Configured for Rust Edition 2024 with resolver version 3
