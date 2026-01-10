# Synapse Development Plan

## Progress Report

| Phase | Status | Progress |
|-------|--------|----------|
| 1. Project Foundation | ⬜ Not Started | 0/4 |
| 2. Echo CLI | ⬜ Not Started | 0/3 |
| 3. Configuration | ⬜ Not Started | 0/4 |
| 4. Provider Abstraction | ⬜ Not Started | 0/3 |
| 5. Anthropic Provider | ⬜ Not Started | 0/5 |
| 6. Streaming Responses | ⬜ Not Started | 0/4 |
| 7. Session Storage | ⬜ Not Started | 0/5 |
| 8. CLI REPL | ⬜ Not Started | 0/4 |
| 9. Multi-Provider | ⬜ Not Started | 0/3 |
| 10. MCP Integration | ⬜ Not Started | 0/5 |
| 11. Telegram Bot | ⬜ Not Started | 0/4 |

**Legend:** ⬜ Not Started | 🔄 In Progress | ✅ Complete | ⏸️ Blocked

**Current Phase:** None
**Last Updated:** —

---

## Phase 1: Project Foundation

**Goal:** Workspace compiles, all crates exist.

- [ ] 1.1 Create workspace `Cargo.toml` with members: `synapse-core`, `synapse-cli`, `synapse-telegram`
- [ ] 1.2 Create `synapse-core` crate with `lib.rs` exporting placeholder module
- [ ] 1.3 Create `synapse-cli` crate with `main.rs` printing "Synapse CLI"
- [ ] 1.4 Verify: `cargo build` succeeds for entire workspace

**Test:** `cargo run -p synapse-cli` prints "Synapse CLI"

---

## Phase 2: Echo CLI

**Goal:** CLI accepts input and echoes it back.

- [ ] 2.1 Add `clap` to `synapse-cli`, define basic args (message as positional arg)
- [ ] 2.2 Implement one-shot mode: `synapse "hello"` → prints "Echo: hello"
- [ ] 2.3 Implement stdin mode: `echo "hello" | synapse` → prints "Echo: hello"

**Test:** Both invocation methods return echoed input.

---

## Phase 3: Configuration

**Goal:** Load settings from TOML file.

- [ ] 3.1 Create `synapse-core/src/config.rs` with `Config` struct (provider, api_key, model)
- [ ] 3.2 Add `toml` + `serde` dependencies, implement TOML parsing
- [ ] 3.3 Create `config.example.toml` in repo root
- [ ] 3.4 Load config in CLI, print loaded provider name

**Test:** `synapse "test"` prints "Provider: anthropic" (from config)

---

## Phase 4: Provider Abstraction

**Goal:** Define LLM provider trait in core.

- [ ] 4.1 Create `synapse-core/src/message.rs` with `Role` enum and `Message` struct
- [ ] 4.2 Create `synapse-core/src/provider.rs` with `LlmProvider` trait (async `complete` method)
- [ ] 4.3 Create `MockProvider` in `provider/mock.rs` returning static response

**Test:** Unit test calls `MockProvider::complete()` and gets response.

---

## Phase 5: Anthropic Provider

**Goal:** Real API calls to Claude.

- [ ] 5.1 Create `synapse-core/src/provider/anthropic.rs` with `AnthropicProvider` struct
- [ ] 5.2 Add `reqwest` (with `json` feature), implement Messages API request
- [ ] 5.3 Create `synapse-core/src/error.rs` with `ProviderError` enum
- [ ] 5.4 Wire provider into CLI: load config → create provider → call API
- [ ] 5.5 Add API key validation (fail fast if missing)

**Test:** `synapse "Say hello"` returns real Claude response.

---

## Phase 6: Streaming Responses

**Goal:** Token-by-token output to terminal.

- [ ] 6.1 Add `eventsource-stream`, `async-stream`, `futures` to core
- [ ] 6.2 Create `synapse-core/src/provider/streaming.rs` with `StreamEvent` enum
- [ ] 6.3 Implement SSE parsing in `AnthropicProvider::stream()` method
- [ ] 6.4 Update CLI to print tokens as they arrive

**Test:** `synapse "Count to 5"` shows numbers appearing progressively.

---

## Phase 7: Session Storage

**Goal:** Persist conversations to SQLite.

- [ ] 7.1 Create `synapse-core/src/session.rs` with `Session` struct
- [ ] 7.2 Create `synapse-core/src/storage.rs` with `SessionStore` trait
- [ ] 7.3 Add `sqlx` (sqlite feature), create `storage/database.rs`
- [ ] 7.4 Implement schema migrations (sessions + messages tables)
- [ ] 7.5 Wire storage into CLI: save messages after each exchange

**Test:** `synapse sessions list` shows previous conversations.

---

## Phase 8: CLI REPL

**Goal:** Interactive chat mode.

- [ ] 8.1 Add `ratatui` + `crossterm` to CLI
- [ ] 8.2 Create `synapse-cli/src/repl.rs` with input loop
- [ ] 8.3 Implement `--repl` flag to enter interactive mode
- [ ] 8.4 Add session resume: `synapse --repl --session <id>`

**Test:** `synapse --repl` allows multi-turn conversation with history.

---

## Phase 9: Multi-Provider

**Goal:** Support OpenAI alongside Anthropic.

- [ ] 9.1 Create `synapse-core/src/provider/openai.rs` implementing `LlmProvider`
- [ ] 9.2 Add provider selection in config and CLI flag (`-p openai`)
- [ ] 9.3 Implement provider factory based on config

**Test:** `synapse -p openai "Hello"` uses GPT, default uses Claude.

---

## Phase 10: MCP Integration

**Goal:** Tool calling via Model Context Protocol.

- [ ] 10.1 Add `rmcp` dependency to core
- [ ] 10.2 Create `synapse-core/src/mcp.rs` with `McpClient` struct
- [ ] 10.3 Load MCP server configs from `mcp_servers.json`
- [ ] 10.4 Implement tool discovery and registration
- [ ] 10.5 Handle tool calls in agent loop: detect → execute → return result

**Test:** Configure a simple MCP server, ask Claude to use it.

---

## Phase 11: Telegram Bot

**Goal:** Second interface using shared core.

- [ ] 11.1 Add `teloxide` to `synapse-telegram`
- [ ] 11.2 Create bot initialization with token from config
- [ ] 11.3 Implement message handler using `synapse-core` agent
- [ ] 11.4 Add session-per-chat persistence

**Test:** Send message to bot, receive Claude response.

---

## Notes

- Each phase builds on previous ones
- Complete all tasks in a phase before moving to next
- Update progress table after completing each phase
- Run `cargo test` after each task to catch regressions
