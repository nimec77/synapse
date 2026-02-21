# Synapse Development Plan

## Progress Report

### Infrastructure

| Ticket | Description | Status |
|--------|-------------|--------|
| SY-1 | Project Foundation | ✅ Complete |
| SY-2 | CI/CD Pipeline | ✅ Complete |
| SY-3 | Echo CLI | ✅ Complete |
| SY-8 | Streaming Responses | ✅ Complete |

### Feature Phases

| Phase | Status | Progress |
|-------|--------|----------|
| 1. Project Foundation (SY-1) | ✅ Complete | 4/4 |
| 2. Echo CLI (SY-3) | ✅ Complete | 3/3 |
| 3. Configuration | ✅ Complete | 4/4 |
| 4. Provider Abstraction (SY-5) | ✅ Complete | 3/3 |
| 5. Anthropic Provider (SY-6) | ✅ Complete | 5/5 |
| 6. DeepSeek Provider (SY-7) | ✅ Complete | 5/5 |
| 7. Streaming Responses (SY-8) | ✅ Complete | 4/4 |
| 8. Session Storage (SY-9) | ✅ Complete | 7/7 |
| 9. CLI REPL (SY-10) | ✅ Complete | 4/4 |
| 10. OpenAI Provider | ✅ Complete | 3/3 |
| 11. MCP Integration | ✅ Complete | 5/5 |
| 12. Telegram Bot (SY-13) | ✅ Complete | 5/5 |
| 13. System Prompt | ✅ Complete | 5/5 |
| 14. File Logging | ✅ Complete | 5/5 |
| 15. Code Refactoring | ⬜ Not Started | 0/7 |

**Legend:** ⬜ Not Started | 🔄 In Progress | ✅ Complete | ⏸️ Blocked

**Current Phase:** 15
**Last Updated:** 2026-02-21

---

## Phase 1: Project Foundation

**Goal:** Workspace compiles, all crates exist.

- [x] 1.1 Create workspace `Cargo.toml` with members: `synapse-core`, `synapse-cli`, `synapse-telegram`
- [x] 1.2 Create `synapse-core` crate with `lib.rs` exporting placeholder module
- [x] 1.3 Create `synapse-cli` crate with `main.rs` printing "Synapse CLI"
- [x] 1.4 Verify: `cargo build` succeeds for entire workspace

**Test:** `cargo run -p synapse-cli` prints "Synapse CLI"

---

## Phase 2: Echo CLI

**Goal:** CLI accepts input and echoes it back.

- [x] 2.1 Add `clap` to `synapse-cli`, define basic args (message as positional arg)
- [x] 2.2 Implement one-shot mode: `synapse "hello"` → prints "Echo: hello"
- [x] 2.3 Implement stdin mode: `echo "hello" | synapse` → prints "Echo: hello"

**Test:** Both invocation methods return echoed input.

---

## Phase 3: Configuration

**Goal:** Load settings from TOML file.

- [x] 3.1 Create `synapse-core/src/config.rs` with `Config` struct (provider, api_key, model)
- [x] 3.2 Add `toml` + `serde` dependencies, implement TOML parsing
- [x] 3.3 Create `config.example.toml` in repo root
- [x] 3.4 Load config in CLI, print loaded provider name

**Test:** `synapse "test"` prints "Provider: anthropic" (from config)

---

## Phase 4: Provider Abstraction

**Goal:** Define LLM provider trait in core.

- [x] 4.1 Create `synapse-core/src/message.rs` with `Role` enum and `Message` struct
- [x] 4.2 Create `synapse-core/src/provider.rs` with `LlmProvider` trait (async `complete` method)
- [x] 4.3 Create `MockProvider` in `provider/mock.rs` returning static response

**Test:** Unit test calls `MockProvider::complete()` and gets response.

---

## Phase 5: Anthropic Provider

**Goal:** Real API calls to Claude.

- [x] 5.1 Create `synapse-core/src/provider/anthropic.rs` with `AnthropicProvider` struct
- [x] 5.2 Add `reqwest` (with `json` feature), implement Messages API request
- [x] 5.3 Extend `ProviderError` enum with `AuthenticationError` variant
- [x] 5.4 Wire provider into CLI: load config → create provider → call API
- [x] 5.5 Add API key validation (fail fast if missing)

**Test:** `synapse "Say hello"` returns real Claude response.

---

## Phase 6: DeepSeek Provider

**Goal:** Default provider works out of the box.

- [x] 6.1 Create `synapse-core/src/provider/deepseek.rs` with `DeepSeekProvider`
- [x] 6.2 Implement OpenAI-compatible chat/completions API request
- [x] 6.3 Create `synapse-core/src/provider/factory.rs` with provider selection
- [x] 6.4 Update CLI to use factory based on `config.provider`
- [x] 6.5 Support `DEEPSEEK_API_KEY` environment variable

**Test:** `synapse "Hello"` with default config uses DeepSeek API.

---

## Phase 7: Streaming Responses

**Goal:** Token-by-token output to terminal.

- [x] 7.1 Add `eventsource-stream`, `async-stream`, `futures` to core
- [x] 7.2 Create `synapse-core/src/provider/streaming.rs` with `StreamEvent` enum
- [x] 7.3 Implement SSE parsing in `DeepSeekProvider::stream()` method
- [x] 7.4 Update CLI to print tokens as they arrive

**Test:** `synapse "Count to 5"` shows numbers appearing progressively.

---

## Phase 8: Session Storage

**Goal:** Persist conversations to SQLite.

- [x] 8.1 Create `synapse-core/src/session.rs` with `Session` struct
- [x] 8.2 Create `synapse-core/src/storage.rs` with `SessionStore` trait
- [x] 8.3 Add `sqlx` (sqlite feature), create `storage/database.rs`
- [x] 8.4 Implement schema migrations (sessions + messages tables)
- [x] 8.5 Wire storage into CLI: save messages after each exchange
- [x] 8.6 Implement session limits (max sessions, retention period)
- [x] 8.7 Add automatic cleanup job for expired sessions

**Test:** `synapse sessions list` shows previous conversations.

**Implementation Notes:**
- **Session IDs**: UUID v7 - time-sortable UUIDs for better database performance
- **Database URL resolution priority**:
  1. `DATABASE_URL` environment variable (highest priority)
  2. `session.database_url` in config.toml
  3. Default: `sqlite:~/.config/synapse/sessions.db`
- **Dependencies**: `sqlx` (sqlite), `uuid` (v7, serde), `chrono` (serde)

---

## Phase 9: CLI REPL

**Goal:** Interactive chat mode.

- [x] 9.1 Add `ratatui` + `crossterm` to CLI
- [x] 9.2 Create `synapse-cli/src/repl.rs` with input loop
- [x] 9.3 Implement `--repl` flag to enter interactive mode
- [x] 9.4 Add session resume: `synapse --repl --session <id>`

**Test:** `synapse --repl` allows multi-turn conversation with history.

---

## Phase 10: OpenAI Provider

**Goal:** Support OpenAI alongside DeepSeek and Anthropic.

- [x] 10.1 Create `synapse-core/src/provider/openai.rs` implementing `LlmProvider`
- [x] 10.2 Add provider selection in config and CLI flag (`-p openai`)
- [x] 10.3 Implement streaming for OpenAI API

**Test:** `synapse -p openai "Hello"` uses GPT, default uses DeepSeek.

---

## Phase 11: MCP Integration

**Goal:** Tool calling via Model Context Protocol.

- [x] 11.1 Add `rmcp` dependency to core
- [x] 11.2 Create `synapse-core/src/mcp.rs` with `McpClient` struct
- [x] 11.3 Load MCP server configs from `mcp_servers.json`
- [x] 11.4 Implement tool discovery and registration
- [x] 11.5 Handle tool calls in agent loop: detect → execute → return result

**Test:** Configure a simple MCP server, ask the LLM to use it.

---

## Phase 12: Telegram Bot

**Goal:** Second interface using shared core.

- [x] 12.1 Add `teloxide` to `synapse-telegram`
- [x] 12.2 Create bot initialization with token from config
- [x] 12.3 Implement message handler using `synapse-core` agent
- [x] 12.4 Add session-per-chat persistence
- [x] 12.5 Add user authorization via `allowed_users` allowlist

**Test:** Send message to bot, receive LLM response. Messages from unlisted user IDs are silently dropped.

**Implementation Notes:**
- **Bot token resolution priority**:
  1. `TELEGRAM_BOT_TOKEN` environment variable (highest priority)
  2. `telegram.token` in config.toml
- **`TelegramConfig`**: `token: Option<String>`, `allowed_users: Vec<u64>`
- **Secure-by-default**: empty `allowed_users` rejects all users
- **Silent drop**: unauthorized messages receive no reply

---

## Phase 13: System Prompt

**Goal:** Wire `config.system_prompt` through the Agent to all provider calls.

- [x] 13.1 Add `system_prompt: Option<String>` to `Config` struct
- [x] 13.2 Add `system_prompt` field and `with_system_prompt()` builder to `Agent`
- [x] 13.3 Implement `build_messages()` helper to prepend `Role::System` on-the-fly
- [x] 13.4 Wire system prompt from config/session into Agent in CLI and Telegram
- [x] 13.5 Update `config.example.toml` with `system_prompt` example

**Test:** Setting `system_prompt` in config causes a `Role::System` message to be prepended to every provider call.

---

## Phase 14: File Logging

**Goal:** Production-ready file-based logging with rotation for the Telegram bot.

- [x] 14.1 Add `LoggingConfig` struct to `synapse-core/src/config.rs` with defaults
- [x] 14.2 Add `tracing-appender` dependency and `registry` feature to `synapse-telegram`
- [x] 14.3 Rewrite tracing init with layered subscriber (stdout + file appender)
- [x] 14.4 Update `config.example.toml` with `[logging]` section documentation
- [x] 14.5 Update `docs/idea.md`, `docs/vision.md`, and `docs/tasklist.md`

**Test:** Add `[logging]` to config, start the bot, verify log files appear in the
configured directory with correct rotation and file count limits.

---

## Phase 15: Code Refactoring

**Goal:** Improve internal code quality without changing external behaviour. Eliminate dead code, reduce duplication, and harden the public API surface.

- [ ] 15.1 Remove dead code and vestigial modules
  - Remove `placeholder` module from `synapse-core/src/lib.rs`
  - Remove unused `StreamEvent` variants (`ToolCall`, `ToolResult`, `Error`)
  - Simplify stream match arms in CLI that reference removed variants
  - Add serde-justification comments on `#[allow(dead_code)]` for deserialization fields

- [ ] 15.2 Extract shared OpenAI-compatible provider base
  - Create `synapse-core/src/provider/openai_compat.rs` with shared types and logic (~400 lines deduplicated from DeepSeek/OpenAI)
  - Shared serde types: `ApiMessage`, `ApiRequest`, `StreamingApiRequest`, `OaiTool`, `OaiFunction`, `OaiToolCall`, `OaiToolCallFunction`, `ApiResponse`, `Choice`, `ChoiceMessage`, `ApiError`, `ErrorDetail`, `StreamChunk`, `StreamDelta`, `StreamChoice`
  - Shared functions: `build_api_messages()`, `complete_request()`, `to_oai_tools()`, `stream_sse()`
  - Reduce `deepseek.rs` and `openai.rs` to thin wrappers (~50-80 lines each)

- [ ] 15.3 Extract magic strings into constants and methods
  - Add `Role::as_str()` and `Role::from_str()` methods
  - Replace manual role-to-string matching in providers and SQLite storage
  - Add constants: `SSE_DONE_MARKER`, provider/env-var lookup in factory, `ERROR_REPLY` in Telegram handlers, `DEFAULT_TRACING_DIRECTIVE` in Telegram main

- [ ] 15.4 Add structured tracing to synapse-core
  - Add `tracing = "0.1"` to `synapse-core/Cargo.toml` and `synapse-cli/Cargo.toml`
  - Instrument: agent (tool loop), providers (HTTP requests), factory (provider creation), storage (session CRUD), config (path resolution), MCP (tool discovery/execution)
  - Replace `eprintln!` in `mcp/tools.rs` and `cli/main.rs` with `tracing::warn!`

- [ ] 15.5 Extract shared utility functions into synapse-core
  - Move `init_mcp_client()` from CLI and Telegram into `synapse-core/src/mcp.rs`
  - Add `Agent::from_config()` factory method to encapsulate system prompt wiring (duplicated 3 times across CLI and Telegram)

- [ ] 15.6 Split large files
  - Split `repl.rs` (678 prod lines) into: `repl/app.rs`, `repl/render.rs`, `repl/input.rs`, `repl.rs` (orchestrator)
  - Extract `synapse-cli/src/commands.rs` from `main.rs` (session subcommand handling)

- [ ] 15.7 Tighten public API surface and fix async convention violation
  - Narrow `lib.rs` re-exports (remove ~16 items never imported by consumer crates)
  - Replace `std::fs::create_dir_all()` with `tokio::fs::create_dir_all().await` in `sqlite.rs`
  - Add `fs` feature to tokio in `synapse-core/Cargo.toml`

**Test:** `cargo clippy -- -D warnings` passes with no new warnings; `cargo test` green; no public API surface regressions.

---

## Notes

- Each phase builds on previous ones
- Complete all tasks in a phase before moving to next
- Update progress table after completing each phase
- Run `cargo test` after each task to catch regressions
