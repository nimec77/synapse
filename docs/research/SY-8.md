# Research: SY-8 - Phase 7: Streaming Responses

## Resolved Questions

The PRD states "None blocking" for open questions, with recommendations provided. Proceeding with the documented recommendations:

1. **Tool Calls in Streaming**: Define `ToolCall`, `ToolResult` enum variants now. Implement parsing only for `TextDelta` and `Done` in this phase. Tool call handling deferred to MCP integration (Phase 11).

2. **Anthropic Streaming**: Deferred to Phase 10 or a separate ticket. This phase focuses on DeepSeek streaming only.

3. **Fallback to Non-Streaming**: Make streaming the default CLI behavior. The `complete()` method remains for programmatic use cases where buffered responses are preferred.

---

## Related Modules and Services

### synapse-core Structure

| File | Purpose | Relevance to SY-8 |
|------|---------|-------------------|
| `synapse-core/src/lib.rs` | Module exports | Must export `StreamEvent` and potentially a streaming method |
| `synapse-core/src/provider.rs` | `LlmProvider` trait, `ProviderError` | Extend trait with `stream()` method |
| `synapse-core/src/provider/deepseek.rs` | `DeepSeekProvider` implementation | Add `stream()` method with SSE parsing |
| `synapse-core/src/provider/mock.rs` | `MockProvider` | May need `stream()` method for testing |
| `synapse-core/src/provider/anthropic.rs` | `AnthropicProvider` | Out of scope for this phase |
| `synapse-core/src/message.rs` | `Role` and `Message` types | Used for context, unchanged |
| `synapse-core/src/provider/factory.rs` | Provider factory | No changes expected |

### synapse-cli Structure

| File | Purpose | Relevance to SY-8 |
|------|---------|-------------------|
| `synapse-cli/src/main.rs` | CLI entry point | Must consume stream and print tokens progressively |

### New File Required

| File | Purpose |
|------|---------|
| `synapse-core/src/provider/streaming.rs` | `StreamEvent` enum and streaming utilities |

---

## Current Endpoints and Contracts

### LlmProvider Trait (Current)

Located in `synapse-core/src/provider.rs`:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

### LlmProvider Trait (Proposed Extension)

The trait needs a `stream()` method. Key design considerations:

1. **Return Type**: `impl Stream<Item = Result<StreamEvent, ProviderError>>`
2. **Object Safety**: The current trait is object-safe (used with `Box<dyn LlmProvider>` in factory). Adding `impl Stream` return type would break object safety.

**Solution Options**:

| Option | Approach | Object Safety | Complexity |
|--------|----------|---------------|------------|
| A | Return `Pin<Box<dyn Stream<...>>>` | Preserved | Medium |
| B | Use associated type | Partially (requires `Sized` or `where Self: Sized`) | Medium |
| C | Separate streaming trait | N/A (new trait) | Low |
| D | Keep generic provider, not object | No factory trait objects | Breaking change |

**Recommended: Option A** - Return boxed stream for object safety:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
}
```

### ProviderError Enum (Current)

Located in `synapse-core/src/provider.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider error: {message}")]
    ProviderError { message: String },

    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("authentication failed: {0}")]
    AuthenticationError(String),

    #[error("missing API key: {0}")]
    MissingApiKey(String),

    #[error("unknown provider: {0}")]
    UnknownProvider(String),
}
```

**Extension needed**: Consider adding `StreamError` variant or reuse `RequestFailed` for stream-specific errors.

### DeepSeek API Streaming Format

DeepSeek uses OpenAI-compatible SSE streaming:

**Request (with streaming)**:
```json
{
  "model": "deepseek-chat",
  "messages": [...],
  "max_tokens": 1024,
  "stream": true
}
```

**SSE Response Format**:
```
data: {"id":"chatcmpl-...","choices":[{"delta":{"content":"Hello"},"index":0}]}

data: {"id":"chatcmpl-...","choices":[{"delta":{"content":" world"},"index":0}]}

data: [DONE]
```

Key SSE parsing considerations:
- Each line prefixed with `data: `
- JSON payloads in `delta.content` field (not `message.content`)
- Stream ends with `data: [DONE]`
- Empty lines between events
- May include `finish_reason` in final delta

---

## Patterns Used

### StreamEvent Enum Design (from `docs/vision.md`)

```rust
pub enum StreamEvent {
    TextDelta(String),
    ToolCall { id: String, name: String, input: Value },
    ToolResult { id: String, output: Value },
    Done,
    Error(Error),
}
```

**Implementation notes**:
- `TextDelta`: The primary event type for this phase
- `Done`: Signals stream completion
- `Error`: Wraps `ProviderError` for stream errors
- `ToolCall`/`ToolResult`: Defined now, implemented in Phase 11

### Async Stream Pattern (from `docs/vision.md`)

```rust
use async_stream::stream;

pub fn chat(...) -> impl Stream<Item = Result<StreamEvent>> {
    async_stream::stream! {
        while let Some(event) = sse_stream.next().await {
            yield Ok(StreamEvent::TextDelta(event.text));
        }
    }
}
```

### SSE Parsing with eventsource-stream

```rust
use eventsource_stream::Eventsource;
use futures::StreamExt;

let stream = response.bytes_stream().eventsource();
while let Some(event) = stream.next().await {
    match event {
        Ok(event) => {
            if event.data == "[DONE]" {
                yield Ok(StreamEvent::Done);
                break;
            }
            // Parse JSON and yield TextDelta
        }
        Err(e) => yield Err(ProviderError::RequestFailed(e.to_string())),
    }
}
```

### MockProvider Pattern for Streaming

```rust
impl MockProvider {
    pub fn with_stream_response(self, tokens: Vec<&str>) -> Self {
        // Store tokens to yield one by one
    }
}

// In stream() implementation
async_stream::stream! {
    for token in self.tokens.lock().unwrap().drain(..) {
        yield Ok(StreamEvent::TextDelta(token));
    }
    yield Ok(StreamEvent::Done);
}
```

---

## Dependencies

### New Dependencies for synapse-core/Cargo.toml

```toml
[dependencies]
# Existing
async-trait = "0.1"
dirs = "6.0.0"
reqwest = { version = "0.12", features = ["json", "stream"] }  # Add "stream" feature
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt", "macros"] }
toml = "0.9.8"

# New for streaming
eventsource-stream = "0.2"
async-stream = "0.3"
futures = "0.3"
```

**Notes**:
- `reqwest` needs `stream` feature for `bytes_stream()`
- `eventsource-stream` v0.2 is the latest as of May 2025
- `async-stream` provides the `stream!` macro for easy stream creation
- `futures` provides `Stream` trait and `StreamExt` for combinators

### synapse-cli/Cargo.toml

```toml
[dependencies]
# Existing
anyhow = "1"
clap = { version = "4.5.54", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std", "signal"] }  # Add features

# New for streaming output
futures = "0.3"
```

**Notes**:
- `tokio` needs `io-std` for stdout flushing and `signal` for Ctrl+C handling
- `futures` needed for `StreamExt` to consume the stream

---

## CLI Streaming Implementation

### Current CLI Flow

```rust
// Current (synapse-cli/src/main.rs)
let response = provider.complete(&messages).await?;
println!("{}", response.content);
```

### Proposed Streaming Flow

```rust
use std::io::{Write, stdout};
use futures::StreamExt;

let mut stream = provider.stream(&messages);
let mut stdout = stdout();

while let Some(event) = stream.next().await {
    match event? {
        StreamEvent::TextDelta(text) => {
            print!("{}", text);
            stdout.flush()?;
        }
        StreamEvent::Done => break,
        StreamEvent::Error(e) => return Err(e.into()),
        _ => {} // Ignore ToolCall/ToolResult for now
    }
}
println!(); // Final newline
```

### Ctrl+C Handling

```rust
use tokio::signal;
use tokio::select;

let stream = provider.stream(&messages);
tokio::pin!(stream);

loop {
    select! {
        event = stream.next() => {
            match event {
                Some(Ok(StreamEvent::TextDelta(text))) => {
                    print!("{}", text);
                    stdout().flush()?;
                }
                Some(Ok(StreamEvent::Done)) | None => break,
                Some(Err(e)) => return Err(e.into()),
                _ => {}
            }
        }
        _ = signal::ctrl_c() => {
            println!("\n[Interrupted]");
            break;
        }
    }
}
```

---

## Limitations and Risks

### Limitations

1. **DeepSeek Only**: Streaming implemented for DeepSeek provider only. AnthropicProvider and MockProvider will need default implementations (returning error or using complete()).

2. **Text Only**: Only `TextDelta` events processed. Tool calls ignored until Phase 11.

3. **No Backpressure**: Stream is consumed as fast as terminal can print. Not an issue for LLM speeds.

4. **No Partial Response Recovery**: If interrupted, no way to resume from where it left off.

5. **Stdout Only**: Streaming output goes directly to stdout, not suitable for REPL mode (Phase 8).

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| R-1: Object safety with `impl Stream` | High | High | Use `Pin<Box<dyn Stream>>` return type |
| R-2: SSE parsing edge cases | Medium | Medium | Test with real API responses, handle malformed data gracefully |
| R-3: `eventsource-stream` API changes | Low | Medium | Pin to specific version |
| R-4: Stream not Send+Sync | Medium | High | Ensure all captured data is Send |
| R-5: Lifetime issues with borrowed messages | Medium | High | Clone messages or use `'static` bounds |

### R-1 Mitigation Detail

The `LlmProvider` trait is currently used with `Box<dyn LlmProvider>`. Adding:

```rust
fn stream(&self, messages: &[Message]) -> impl Stream<...>;
```

Would make the trait non-object-safe. Solution:

```rust
fn stream(
    &self,
    messages: &[Message],
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
```

This preserves object safety while allowing async streaming.

### R-5 Mitigation Detail

The `stream()` method borrows `&self` and `&[Message]`. The returned stream must not outlive these references. Using `+ '_` in the return type ties the stream's lifetime to `&self`.

For simplicity, the implementation can clone messages:

```rust
fn stream(&self, messages: &[Message]) -> ... {
    let messages = messages.to_vec();  // Clone to own
    Box::pin(async_stream::stream! {
        // Use owned messages
    })
}
```

---

## New Technical Questions

Questions discovered during research that may need follow-up:

1. **Default Implementation for Non-Streaming Providers**: Should `MockProvider` and `AnthropicProvider` have a default `stream()` implementation that wraps `complete()`?
   - **Recommendation**: Yes, implement a wrapper that calls `complete()` and yields a single `TextDelta` followed by `Done`.

2. **Error Recovery**: Should the stream yield `StreamEvent::Error` and continue, or should errors terminate the stream?
   - **Recommendation**: Errors should terminate the stream. The caller can decide to retry.

3. **Rate Limiting on Print**: If tokens arrive faster than terminal can render, should we buffer?
   - **Recommendation**: No, LLM token generation is slow enough that this won't be an issue.

4. **Finish Reason**: Should we expose the finish reason (`stop`, `length`, `content_filter`) in `StreamEvent::Done`?
   - **Recommendation**: Defer to future enhancement. For now, `Done` is a unit variant.

5. **Usage Stats**: SSE responses include usage stats in the final event. Should we capture these?
   - **Recommendation**: Defer to future enhancement. For MVP, ignore usage stats.

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/Cargo.toml` | Modify | Add `eventsource-stream`, `async-stream`, `futures`, enable reqwest `stream` feature |
| `synapse-core/src/provider/streaming.rs` | Create | `StreamEvent` enum definition |
| `synapse-core/src/provider.rs` | Modify | Add `mod streaming;`, export `StreamEvent`, extend `LlmProvider` trait with `stream()` method |
| `synapse-core/src/provider/deepseek.rs` | Modify | Implement `stream()` method with SSE parsing |
| `synapse-core/src/provider/anthropic.rs` | Modify | Add default `stream()` implementation (wraps complete) |
| `synapse-core/src/provider/mock.rs` | Modify | Add `stream()` implementation for testing |
| `synapse-core/src/lib.rs` | Modify | Export `StreamEvent` |
| `synapse-cli/Cargo.toml` | Modify | Add `futures`, enable tokio `signal` and `io-std` features |
| `synapse-cli/src/main.rs` | Modify | Replace `complete()` with `stream()`, add progressive output |

---

## Test Plan

### Unit Tests

1. **StreamEvent Enum**:
   - `test_stream_event_text_delta` - Verify TextDelta holds content
   - `test_stream_event_done` - Verify Done variant

2. **SSE Parsing** (in deepseek.rs):
   - `test_parse_sse_text_delta` - Parse single text delta event
   - `test_parse_sse_done` - Parse `[DONE]` event
   - `test_parse_sse_multiple_events` - Parse sequence of events
   - `test_parse_sse_malformed` - Handle malformed JSON gracefully

3. **MockProvider Streaming**:
   - `test_mock_provider_stream` - Stream yields configured tokens
   - `test_mock_provider_stream_empty` - Empty stream yields only Done

### Manual Integration Tests

```bash
# Test streaming output - should see tokens appear progressively
cargo run -p synapse-cli -- "Count from 1 to 10 slowly"

# Test Ctrl+C interruption
cargo run -p synapse-cli -- "Write a very long essay about the history of computing"
# Press Ctrl+C after a few sentences

# Test error handling (invalid API key)
DEEPSEEK_API_KEY=invalid cargo run -p synapse-cli -- "Hello"
```

---

## Implementation Sequence

Recommended order of implementation:

1. **Task 7.1**: Add dependencies to `synapse-core/Cargo.toml`
2. **Task 7.2**: Create `streaming.rs` with `StreamEvent` enum
3. **Task 7.3**: Extend `LlmProvider` trait with `stream()` method (with default impl)
4. **Task 7.4**: Implement `DeepSeekProvider::stream()` with SSE parsing
5. **Task 7.5**: Update `MockProvider` with streaming support
6. **Task 7.6**: Update CLI to use streaming and handle Ctrl+C

---

## References

- `docs/prd/SY-8.prd.md` - PRD document
- `docs/phase/phase-7.md` - Phase task breakdown
- `docs/vision.md` - Architecture patterns and StreamEvent design
- `synapse-core/src/provider/deepseek.rs` - DeepSeekProvider implementation to extend
- [DeepSeek API Documentation](https://api-docs.deepseek.com/)
- [eventsource-stream crate](https://docs.rs/eventsource-stream/)
- [async-stream crate](https://docs.rs/async-stream/)
