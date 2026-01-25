# SY-8: Streaming Responses

Status: IMPLEMENT_STEP_OK

Context: Phase 7 - Implement token-by-token streaming output to the CLI for real-time response display. Based on `docs/prd/SY-8.prd.md` and `docs/plan/SY-8.md`.

---

## Tasks

### Task 7.1: Add Streaming Dependencies
- [x] Update `synapse-core/Cargo.toml`: add `eventsource-stream = "0.2"`, `async-stream = "0.3"`, `futures = "0.3"`, and enable `stream` feature for reqwest
- [x] Update `synapse-cli/Cargo.toml`: add `futures = "0.3"`, enable tokio features `io-std` and `signal`

**Acceptance Criteria:**
- `cargo check -p synapse-core` passes with new dependencies
- `cargo check -p synapse-cli` passes with new features enabled

---

### Task 7.2: Create StreamEvent Enum
- [x] Create `synapse-core/src/provider/streaming.rs` with `StreamEvent` enum
- [x] Define variants: `TextDelta(String)`, `ToolCall { id, name, input }`, `ToolResult { id, output }`, `Done`, `Error(ProviderError)`
- [x] Add module declaration in `synapse-core/src/provider.rs`: `mod streaming;`
- [x] Export `StreamEvent` from `synapse-core/src/lib.rs`

**Acceptance Criteria:**
- `StreamEvent` enum compiles with `Debug` and `Clone` derives
- Unit test `test_stream_event_variants` verifies each variant can be constructed
- `synapse-core` exports `StreamEvent` publicly

---

### Task 7.3: Extend LlmProvider Trait with stream() Method
- [x] Add `stream()` method signature to `LlmProvider` trait in `synapse-core/src/provider.rs`
- [x] Return type: `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>`
- [x] Add necessary imports (`std::pin::Pin`, `futures::Stream`)

**Acceptance Criteria:**
- Trait compiles without errors
- Return type preserves object safety (works with `Box<dyn LlmProvider>`)
- `cargo check -p synapse-core` passes

---

### Task 7.4: Implement DeepSeekProvider::stream()
- [x] Add streaming API types: `StreamingApiRequest`, `StreamChunk`, `StreamChoice`, `StreamDelta`
- [x] Implement `stream()` method using `async_stream::stream!` macro
- [x] Send request with `stream: true` parameter
- [x] Parse SSE events using `eventsource-stream`
- [x] Handle `[DONE]` marker, empty content, and errors
- [x] Yield `TextDelta` for non-empty content, `Done` on completion

**Acceptance Criteria:**
- Unit test `test_parse_sse_text_delta` verifies JSON parsing of delta content
- Unit test `test_parse_sse_done` verifies `[DONE]` marker handling
- Manual test: `DEEPSEEK_API_KEY=... cargo run -p synapse-cli -- "Count 1 to 5"` shows progressive output

---

### Task 7.5: Implement MockProvider::stream()
- [x] Add `stream_tokens: Mutex<Vec<String>>` field to `MockProvider`
- [x] Add `with_stream_tokens()` builder method
- [x] Implement `stream()` method: yield each token as `TextDelta`, then `Done`
- [x] If no tokens configured, fall back to calling `complete()` and yielding single delta

**Acceptance Criteria:**
- Unit test `test_mock_stream_tokens` verifies configured tokens are yielded
- Unit test `test_mock_stream_fallback` verifies fallback to `complete()` behavior
- Stream always ends with `Done` event

---

### Task 7.6: Implement AnthropicProvider::stream()
- [x] Add `stream()` method that wraps `complete()` call
- [x] Yield full response as single `TextDelta`, then `Done`
- [x] Handle errors by yielding `Err(...)` from stream

**Acceptance Criteria:**
- Unit test verifies AnthropicProvider implements `stream()` method
- Manual test with `provider = "anthropic"` in config shows response (non-progressive)
- No compile errors for `dyn LlmProvider` usage

---

### Task 7.7: Update CLI for Streaming Output
- [x] Replace `complete()` call with `stream()` in `synapse-cli/src/main.rs`
- [x] Use `tokio::pin!` on stream for iteration
- [x] Print each `TextDelta` immediately with `print!()` and `stdout.flush()`
- [x] Add `tokio::select!` with `tokio::signal::ctrl_c()` for graceful interruption
- [x] Print `[Interrupted]` message on Ctrl+C
- [x] Print final newline on `Done`

**Acceptance Criteria:**
- Manual test: `synapse "Count 1 to 10"` shows tokens appearing progressively
- Manual test: Pressing Ctrl+C during streaming prints `[Interrupted]` and exits cleanly
- Manual test: Network/API error displays error message appropriately
- `cargo clippy -p synapse-cli` passes without warnings

---

## Summary

| Task | Description | Dependencies |
|------|-------------|--------------|
| 7.1 | Add streaming dependencies | None |
| 7.2 | Create StreamEvent enum | 7.1 |
| 7.3 | Extend LlmProvider trait | 7.2 |
| 7.4 | DeepSeekProvider streaming | 7.3 |
| 7.5 | MockProvider streaming | 7.3 |
| 7.6 | AnthropicProvider streaming | 7.3 |
| 7.7 | CLI streaming output | 7.4, 7.5, 7.6 |
