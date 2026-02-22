# Phase 15: Code Refactoring

**Goal:** Improve internal code quality without changing external behaviour. Eliminate dead code, reduce duplication, and harden the public API surface.

## Tasks

- [x] 15.1 Remove dead code and vestigial modules
  - Remove `placeholder` module from `synapse-core/src/lib.rs`
  - Remove unused `StreamEvent` variants (`ToolCall`, `ToolResult`, `Error`)
  - Simplify stream match arms in CLI that reference removed variants
  - Add serde-justification comments on `#[allow(dead_code)]` for deserialization fields

- [x] 15.2 Extract shared OpenAI-compatible provider base
  - Create `synapse-core/src/provider/openai_compat.rs` with shared types and logic (~400 lines deduplicated from DeepSeek/OpenAI)
  - Shared serde types: `ApiMessage`, `ApiRequest`, `StreamingApiRequest`, `OaiTool`, `OaiFunction`, `OaiToolCall`, `OaiToolCallFunction`, `ApiResponse`, `Choice`, `ChoiceMessage`, `ApiError`, `ErrorDetail`, `StreamChunk`, `StreamDelta`, `StreamChoice`
  - Shared functions: `build_api_messages()`, `complete_request()`, `to_oai_tools()`, `stream_sse()`
  - Reduce `deepseek.rs` and `openai.rs` to thin wrappers (~50-80 lines each)

- [x] 15.3 Extract magic strings into constants and methods
  - Add `Role::as_str()` and `Role::from_str()` methods
  - Replace manual role-to-string matching in providers and SQLite storage
  - Add constants: `SSE_DONE_MARKER`, provider/env-var lookup in factory, `ERROR_REPLY` in Telegram handlers, `DEFAULT_TRACING_DIRECTIVE` in Telegram main

- [x] 15.4 Add structured tracing to synapse-core
  - Add `tracing = "0.1"` to `synapse-core/Cargo.toml` and `synapse-cli/Cargo.toml`
  - Instrument: agent (tool loop), providers (HTTP requests), factory (provider creation), storage (session CRUD), config (path resolution), MCP (tool discovery/execution)
  - Replace `eprintln!` in `mcp/tools.rs` and `cli/main.rs` with `tracing::warn!`

- [x] 15.5 Extract shared utility functions into synapse-core
  - Move `init_mcp_client()` from CLI and Telegram into `synapse-core/src/mcp.rs`
  - Add `Agent::from_config()` factory method to encapsulate system prompt wiring (duplicated 3 times across CLI and Telegram)

- [x] 15.6 Split large files
  - Split `repl.rs` (678 prod lines) into: `repl/app.rs`, `repl/render.rs`, `repl/input.rs`, `repl.rs` (orchestrator)
  - Extract `synapse-cli/src/commands.rs` from `main.rs` (session subcommand handling)

- [x] 15.7 Tighten public API surface and fix async convention violation
  - Narrow `lib.rs` re-exports (remove ~16 items never imported by consumer crates)
  - Replace `std::fs::create_dir_all()` with `tokio::fs::create_dir_all().await` in `sqlite.rs`
  - Add `fs` feature to tokio in `synapse-core/Cargo.toml`

## Acceptance Criteria

**Test:** `cargo clippy -- -D warnings` passes with no new warnings; `cargo test` green; no public API surface regressions.

## Dependencies

- Phase 14 complete

## Implementation Notes

- All changes must be purely internal — no external behaviour changes
- After each task, run `cargo clippy -- -D warnings && cargo test` to catch regressions early
- Task 15.2 (OpenAI-compat extraction) is the highest-value refactor; do it before 15.3/15.4
- Task 15.1 must be done before 15.2 to avoid carrying dead variants into the new shared module
