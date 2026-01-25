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
| 8. Session Storage | ⬜ Not Started | 0/7 |
| 9. CLI REPL | ⬜ Not Started | 0/4 |
| 10. OpenAI Provider | ⬜ Not Started | 0/3 |
| 11. MCP Integration | ⬜ Not Started | 0/5 |
| 12. Telegram Bot | ⬜ Not Started | 0/4 |

**Legend:** ⬜ Not Started | 🔄 In Progress | ✅ Complete | ⏸️ Blocked

**Current Phase:** 8
**Last Updated:** 2026-01-25

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

- [ ] 8.1 Create `synapse-core/src/session.rs` with `Session` struct
- [ ] 8.2 Create `synapse-core/src/storage.rs` with `SessionStore` trait
- [ ] 8.3 Add `sqlx` (sqlite feature), create `storage/database.rs`
- [ ] 8.4 Implement schema migrations (sessions + messages tables)
- [ ] 8.5 Wire storage into CLI: save messages after each exchange
- [ ] 8.6 Implement session limits (max sessions, retention period)
- [ ] 8.7 Add automatic cleanup job for expired sessions

**Test:** `synapse sessions list` shows previous conversations.

---

## Phase 9: CLI REPL

**Goal:** Interactive chat mode.

- [ ] 9.1 Add `ratatui` + `crossterm` to CLI
- [ ] 9.2 Create `synapse-cli/src/repl.rs` with input loop
- [ ] 9.3 Implement `--repl` flag to enter interactive mode
- [ ] 9.4 Add session resume: `synapse --repl --session <id>`

**Test:** `synapse --repl` allows multi-turn conversation with history.

---

## Phase 10: OpenAI Provider

**Goal:** Support OpenAI alongside DeepSeek and Anthropic.

- [ ] 10.1 Create `synapse-core/src/provider/openai.rs` implementing `LlmProvider`
- [ ] 10.2 Add provider selection in config and CLI flag (`-p openai`)
- [ ] 10.3 Implement streaming for OpenAI API

**Test:** `synapse -p openai "Hello"` uses GPT, default uses DeepSeek.

---

## Phase 11: MCP Integration

**Goal:** Tool calling via Model Context Protocol.

- [ ] 11.1 Add `rmcp` dependency to core
- [ ] 11.2 Create `synapse-core/src/mcp.rs` with `McpClient` struct
- [ ] 11.3 Load MCP server configs from `mcp_servers.json`
- [ ] 11.4 Implement tool discovery and registration
- [ ] 11.5 Handle tool calls in agent loop: detect → execute → return result

**Test:** Configure a simple MCP server, ask the LLM to use it.

---

## Phase 12: Telegram Bot

**Goal:** Second interface using shared core.

- [ ] 12.1 Add `teloxide` to `synapse-telegram`
- [ ] 12.2 Create bot initialization with token from config
- [ ] 12.3 Implement message handler using `synapse-core` agent
- [ ] 12.4 Add session-per-chat persistence

**Test:** Send message to bot, receive LLM response.

---

## Notes

- Each phase builds on previous ones
- Complete all tasks in a phase before moving to next
- Update progress table after completing each phase
- Run `cargo test` after each task to catch regressions
