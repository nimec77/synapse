# Implementation Plan: SY-8 - Phase 7: Streaming Responses

Status: PLAN_APPROVED

## Overview

This plan details the implementation of streaming responses for the Synapse CLI. The feature enables token-by-token output display, providing immediate feedback to users and a more responsive feel. This phase focuses on extending the `LlmProvider` trait with a `stream()` method, implementing SSE parsing for DeepSeekProvider, and updating the CLI to display tokens progressively.

---

## Components

### 1. StreamEvent Enum (`synapse-core/src/provider/streaming.rs`)

New file defining the streaming event types as specified in `docs/vision.md`.

```rust
use crate::provider::ProviderError;

/// Events emitted during streaming LLM responses.
///
/// Each variant represents a different type of event that can occur
/// during a streaming response.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A text fragment from the LLM response.
    TextDelta(String),

    /// A tool call request from the LLM (Phase 11).
    ToolCall {
        /// Unique identifier for this tool call.
        id: String,
        /// Name of the tool to invoke.
        name: String,
        /// JSON input for the tool.
        input: serde_json::Value,
    },

    /// Result of a tool invocation (Phase 11).
    ToolResult {
        /// Tool call ID this result corresponds to.
        id: String,
        /// JSON output from the tool.
        output: serde_json::Value,
    },

    /// Stream completed successfully.
    Done,

    /// An error occurred during streaming.
    Error(ProviderError),
}
```

**Design notes:**
- `Clone` derived for potential buffering/replay scenarios
- `ToolCall` and `ToolResult` defined now but unused until Phase 11
- `Error` variant wraps `ProviderError` for consistent error handling

### 2. LlmProvider Trait Extension (`synapse-core/src/provider.rs`)

Extend the trait with a `stream()` method. The key challenge is maintaining object safety since the trait is used with `Box<dyn LlmProvider>`.

```rust
use std::pin::Pin;
use futures::Stream;

pub use streaming::StreamEvent;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages to the LLM and get a response.
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;

    /// Stream response tokens from the LLM.
    ///
    /// Returns a stream of [`StreamEvent`] items. The stream ends with
    /// [`StreamEvent::Done`] on success or [`StreamEvent::Error`] on failure.
    ///
    /// # Default Implementation
    ///
    /// By default, this calls `complete()` and yields a single `TextDelta`
    /// followed by `Done`. Providers should override this for true streaming.
    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
}
```

**Object safety solution:**
- Return `Pin<Box<dyn Stream<...> + Send + '_>>` instead of `impl Stream`
- The `'_` lifetime ties the stream to `&self`
- `Send` bound enables use in async contexts

**Default implementation approach:**
- Provide a default implementation in the trait that wraps `complete()`
- Non-streaming providers (Anthropic, Mock) get streaming "for free"
- DeepSeek overrides with true SSE streaming

### 3. DeepSeekProvider Streaming (`synapse-core/src/provider/deepseek.rs`)

Add SSE streaming implementation to DeepSeekProvider.

**New API types for streaming:**

```rust
/// Streaming request body (adds stream: true).
#[derive(Debug, Serialize)]
struct StreamingApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: u32,
    stream: bool,
}

/// SSE delta response chunk.
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}
```

**Stream implementation:**

```rust
impl DeepSeekProvider {
    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        // Clone data needed for the async stream
        let messages = messages.to_vec();
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            // Build request with stream: true
            let api_messages = /* convert messages */;
            let request = StreamingApiRequest {
                model,
                messages: api_messages,
                max_tokens: DEFAULT_MAX_TOKENS,
                stream: true,
            };

            // Send request
            let response = client
                .post(API_ENDPOINT)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    yield Err(ProviderError::RequestFailed(e.to_string()));
                    return;
                }
            };

            // Check for HTTP errors
            if !response.status().is_success() {
                yield Err(ProviderError::RequestFailed(
                    format!("HTTP {}", response.status())
                ));
                return;
            }

            // Parse SSE stream
            let mut stream = response.bytes_stream().eventsource();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        // Handle [DONE] marker
                        if event.data == "[DONE]" {
                            yield Ok(StreamEvent::Done);
                            return;
                        }

                        // Parse JSON delta
                        match serde_json::from_str::<StreamChunk>(&event.data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        if !content.is_empty() {
                                            yield Ok(StreamEvent::TextDelta(content.clone()));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                yield Err(ProviderError::ProviderError {
                                    message: format!("Failed to parse SSE: {}", e),
                                });
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ProviderError::RequestFailed(e.to_string()));
                        return;
                    }
                }
            }

            // Stream ended without [DONE] - still signal done
            yield Ok(StreamEvent::Done);
        })
    }
}
```

### 4. MockProvider Streaming (`synapse-core/src/provider/mock.rs`)

Add streaming capability for testing.

```rust
pub struct MockProvider {
    responses: Mutex<Vec<Message>>,
    stream_tokens: Mutex<Vec<String>>,  // NEW
}

impl MockProvider {
    /// Configure tokens to yield when streaming.
    ///
    /// If not configured, streaming will use the default complete() behavior.
    pub fn with_stream_tokens(self, tokens: Vec<&str>) -> Self {
        let mut guard = self.stream_tokens.lock().unwrap();
        *guard = tokens.into_iter().map(|s| s.to_string()).collect();
        self
    }
}
```

The `stream()` implementation will:
1. If `stream_tokens` is non-empty, yield each token as `TextDelta` then `Done`
2. Otherwise, call `complete()` and yield the full response as a single `TextDelta`

### 5. AnthropicProvider Fallback (`synapse-core/src/provider/anthropic.rs`)

Use the default trait implementation that wraps `complete()`. No code changes required if default impl is provided in trait.

Alternatively, if no default impl is possible, add a simple wrapper:

```rust
fn stream(
    &self,
    messages: &[Message],
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
    let messages = messages.to_vec();
    Box::pin(async_stream::stream! {
        match self.complete(&messages).await {
            Ok(msg) => {
                yield Ok(StreamEvent::TextDelta(msg.content));
                yield Ok(StreamEvent::Done);
            }
            Err(e) => {
                yield Err(e);
            }
        }
    })
}
```

### 6. CLI Streaming Output (`synapse-cli/src/main.rs`)

Replace the `complete()` call with `stream()` and handle progressive output.

```rust
use std::io::{Write, stdout};
use futures::StreamExt;
use tokio::select;
use tokio::signal;

use synapse_core::{Config, Message, Role, StreamEvent, create_provider};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    let message = match get_message(&args) {
        Ok(msg) => msg,
        Err(_) => {
            Args::parse_from(["synapse", "--help"]);
            return Ok(());
        }
    };

    let provider = create_provider(&config)
        .context("Failed to create LLM provider")?;

    let messages = vec![Message::new(Role::User, message)];

    // Stream response with Ctrl+C handling
    let stream = provider.stream(&messages);
    tokio::pin!(stream);

    let mut stdout = stdout();

    loop {
        select! {
            event = stream.next() => {
                match event {
                    Some(Ok(StreamEvent::TextDelta(text))) => {
                        print!("{}", text);
                        stdout.flush().context("Failed to flush stdout")?;
                    }
                    Some(Ok(StreamEvent::Done)) | None => {
                        println!(); // Final newline
                        break;
                    }
                    Some(Ok(StreamEvent::Error(e))) => {
                        return Err(e).context("Streaming error");
                    }
                    Some(Ok(_)) => {
                        // Ignore ToolCall/ToolResult for now
                    }
                    Some(Err(e)) => {
                        return Err(e).context("Stream error");
                    }
                }
            }
            _ = signal::ctrl_c() => {
                println!("\n[Interrupted]");
                break;
            }
        }
    }

    Ok(())
}
```

---

## API Contract

### SSE Streaming Format (OpenAI-compatible)

**Request:**
```json
{
  "model": "deepseek-chat",
  "messages": [
    {"role": "user", "content": "Count from 1 to 5"}
  ],
  "max_tokens": 1024,
  "stream": true
}
```

**Response (SSE):**
```
data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}

data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":"1"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":", "},"finish_reason":null}]}

data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":"2"},"finish_reason":null}]}

...

data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

**Key parsing rules:**
1. Lines prefixed with `data: `
2. JSON payloads have `delta.content` (not `message.content`)
3. First event may have `delta.role` without content
4. Final delta may have empty content with `finish_reason`
5. Stream ends with literal `data: [DONE]`
6. Empty lines between events (handled by eventsource-stream)

### Stream Method Contract

```rust
/// Streams LLM response tokens.
///
/// # Events
/// - `TextDelta(String)`: A text fragment (may be partial word)
/// - `Done`: Stream completed successfully
/// - `Error(ProviderError)`: An error occurred
///
/// # Guarantees
/// - Stream always ends with `Done` or `Error`
/// - `TextDelta` events contain non-empty strings
/// - Events are yielded as soon as received (no buffering)
fn stream(&self, messages: &[Message])
    -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;
```

---

## Data Flows

### Normal Streaming Flow

```
1. User: synapse "Count to 5"
         |
2. CLI: create_provider(&config) -> Box<dyn LlmProvider>
         |
3. CLI: provider.stream(&messages)
         |
4. DeepSeek: POST /chat/completions { stream: true }
         |
5. DeepSeek API: Returns SSE stream
         |
6. DeepSeek: Parse SSE event "data: {...delta.content: "1"...}"
         |
7. DeepSeek: yield Ok(StreamEvent::TextDelta("1"))
         |
8. CLI: print!("1"); stdout.flush()
         |
9. [Repeat steps 6-8 for each token]
         |
10. DeepSeek: Parse "data: [DONE]"
          |
11. DeepSeek: yield Ok(StreamEvent::Done)
          |
12. CLI: println!() // final newline
          |
13. User sees: "1, 2, 3, 4, 5" (appeared progressively)
```

### Ctrl+C Interruption Flow

```
1. User: synapse "Write a long essay"
         |
2. CLI: provider.stream(&messages)
         |
3. [Tokens arriving, printing to terminal]
         |
4. User: Presses Ctrl+C
         |
5. tokio::signal::ctrl_c() resolves
         |
6. select! branch: println!("\n[Interrupted]")
         |
7. CLI: break from loop
         |
8. Stream dropped (connection closed)
         |
9. CLI: return Ok(()) - clean exit
```

### Error During Streaming Flow

```
1. User: synapse "Hello"
         |
2. CLI: provider.stream(&messages)
         |
3. [Connection established, some tokens received]
         |
4. Network: Connection drops
         |
5. eventsource-stream: Returns Err(...)
         |
6. DeepSeek: yield Err(ProviderError::RequestFailed("..."))
         |
7. CLI: Receives Some(Err(e))
         |
8. CLI: return Err(e).context("Stream error")
         |
9. User sees: "Error: Stream error: request failed: ..."
```

### Fallback Provider Flow (Anthropic)

```
1. User: config.toml has provider = "anthropic"
         |
2. CLI: provider.stream(&messages)
         |
3. Anthropic: Default impl calls complete(&messages).await
         |
4. Anthropic: Waits for full response
         |
5. Anthropic: yield Ok(StreamEvent::TextDelta(full_content))
         |
6. Anthropic: yield Ok(StreamEvent::Done)
         |
7. CLI: Prints full response at once
```

---

## Non-Functional Requirements

### Performance

| Requirement | Target | Implementation |
|-------------|--------|----------------|
| First token latency | < 500ms | Stream immediately, no client buffering |
| Token throughput | Real-time | Print as received, flush after each token |
| Memory usage | O(1) per token | No response buffering; process/print/discard |
| Startup overhead | < 10ms | Lazy stream creation |

### Reliability

| Requirement | Implementation |
|-------------|----------------|
| Graceful interruption | Ctrl+C handler via tokio::signal |
| Connection cleanup | Stream drop closes HTTP connection |
| Error propagation | All errors yield through stream, no panics |
| Partial output | User sees tokens received before error |

### Security

| Requirement | Implementation |
|-------------|----------------|
| HTTPS only | Hardcoded API endpoint |
| No credential logging | API key never logged or included in errors |
| Input validation | Messages validated before streaming |

### Compatibility

| Requirement | Implementation |
|-------------|----------------|
| Backward compatible | `complete()` method unchanged |
| Object safety | `Pin<Box<dyn Stream>>` return type |
| Provider agnostic | Default impl for non-streaming providers |

---

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/Cargo.toml` | Modify | Add `eventsource-stream`, `async-stream`, `futures`; add `stream` feature to reqwest |
| `synapse-core/src/provider/streaming.rs` | Create | `StreamEvent` enum definition |
| `synapse-core/src/provider.rs` | Modify | Add `mod streaming;`, export `StreamEvent`, extend trait with `stream()` |
| `synapse-core/src/provider/deepseek.rs` | Modify | Implement `stream()` with SSE parsing |
| `synapse-core/src/provider/anthropic.rs` | Modify | Add fallback `stream()` implementation |
| `synapse-core/src/provider/mock.rs` | Modify | Add `stream()` implementation for testing |
| `synapse-core/src/lib.rs` | Modify | Export `StreamEvent` |
| `synapse-cli/Cargo.toml` | Modify | Add `futures`, enable tokio `signal` and `io-std` features |
| `synapse-cli/src/main.rs` | Modify | Replace `complete()` with `stream()`, add Ctrl+C handling |

---

## Dependency Changes

### synapse-core/Cargo.toml

```toml
[dependencies]
# Existing
async-trait = "0.1"
dirs = "6.0.0"
reqwest = { version = "0.12", features = ["json", "stream"] }  # Add "stream"
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

### synapse-cli/Cargo.toml

```toml
[dependencies]
anyhow = "1"
clap = { version = "4.5.54", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std", "signal"] }  # Add features

# New for streaming
futures = "0.3"
```

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **R-1: Object safety with `impl Stream`** | High | High | Use `Pin<Box<dyn Stream>>` return type. Validated in research. |
| **R-2: SSE parsing edge cases** | Medium | Medium | Handle malformed JSON gracefully; test with real API. |
| **R-3: Lifetime issues** | Medium | High | Clone messages at stream creation; use `'_` lifetime bound. |
| **R-4: Stream not Send** | Medium | High | Ensure all captured data is `Send`; use cloned values. |
| **R-5: eventsource-stream compatibility** | Low | Medium | Pin to v0.2; well-maintained crate. |
| **R-6: Ctrl+C not working** | Low | Medium | Use `tokio::select!` with `tokio::signal::ctrl_c()`. |
| **R-7: Stdout flush overhead** | Low | Low | LLM token rate is slow; flush is negligible. |

---

## Design Decisions

### 1. Return Type for `stream()`

**Decision:** `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>`

**Rationale:** This preserves object safety (critical for `Box<dyn LlmProvider>` in factory) while allowing true async streaming. The alternative (`impl Stream`) would break object safety.

**Trade-off:** Heap allocation for each stream. Acceptable since streams are created per-request.

### 2. Default Implementation Strategy

**Decision:** No default impl in trait; each provider implements explicitly.

**Rationale:** Async trait methods with complex return types make defaults tricky. Explicit impl in each provider is clearer and avoids `#[async_trait]` interaction issues.

**Alternative considered:** Provide default impl that calls `complete()`. Rejected due to complexity of calling async method in sync context.

### 3. Error Handling in Stream

**Decision:** Errors terminate the stream; no recovery.

**Rationale:** Simpler implementation. Callers can catch errors and retry the entire request. Mid-stream recovery would require complex state management.

### 4. Clone Messages at Stream Creation

**Decision:** Clone `messages` slice when creating stream.

**Rationale:** Avoids lifetime complexity. Messages are typically small (few KB). The alternative (borrowing) creates complex lifetime bounds.

### 5. Token Filtering

**Decision:** Only yield `TextDelta` for non-empty content.

**Rationale:** Empty deltas (e.g., first event with only `role`) are noise. Filtering at provider level simplifies CLI code.

### 6. Ctrl+C via tokio::select!

**Decision:** Use `tokio::select!` with `signal::ctrl_c()` for interruption.

**Rationale:** Clean integration with async stream consumption. Alternatives (ctrlc crate, raw signal handlers) add complexity without benefit.

---

## Testing Strategy

### Unit Tests

**In `synapse-core/src/provider/streaming.rs`:**
1. `test_stream_event_text_delta` - Verify TextDelta construction
2. `test_stream_event_done` - Verify Done variant
3. `test_stream_event_error` - Verify Error wraps ProviderError

**In `synapse-core/src/provider/deepseek.rs`:**
1. `test_parse_sse_text_delta` - Parse single text delta JSON
2. `test_parse_sse_done` - Parse `[DONE]` marker
3. `test_parse_sse_empty_content` - Handle empty delta.content
4. `test_parse_sse_with_role` - Handle first event with role only
5. `test_parse_sse_finish_reason` - Handle final event with finish_reason

**In `synapse-core/src/provider/mock.rs`:**
1. `test_mock_stream_tokens` - Stream yields configured tokens
2. `test_mock_stream_empty` - Empty config uses complete() fallback
3. `test_mock_stream_done` - Stream always ends with Done

### Integration Tests (Manual)

```bash
# Test streaming output
DEEPSEEK_API_KEY=... cargo run -p synapse-cli -- "Count from 1 to 10 slowly"
# Expected: Numbers appear progressively

# Test Ctrl+C interruption
DEEPSEEK_API_KEY=... cargo run -p synapse-cli -- "Write a very long essay"
# Press Ctrl+C after a few words
# Expected: "[Interrupted]" message, clean exit

# Test error handling
DEEPSEEK_API_KEY=invalid cargo run -p synapse-cli -- "Hello"
# Expected: Authentication error message

# Test fallback provider (Anthropic)
# Set provider = "anthropic" in config.toml
ANTHROPIC_API_KEY=... cargo run -p synapse-cli -- "Hello"
# Expected: Full response appears (not streamed progressively)
```

---

## Implementation Sequence

Recommended task order:

1. **Task 7.1: Add dependencies** - Update both Cargo.toml files
2. **Task 7.2: Create StreamEvent** - New `streaming.rs` file
3. **Task 7.3: Extend LlmProvider trait** - Add `stream()` method signature
4. **Task 7.4: Implement DeepSeekProvider::stream()** - SSE parsing
5. **Task 7.5: Implement MockProvider::stream()** - For testing
6. **Task 7.6: Implement AnthropicProvider::stream()** - Fallback wrapper
7. **Task 7.7: Update CLI** - Streaming output with Ctrl+C handling

---

## Open Questions

None blocking. All technical decisions resolved during research phase.

---

## References

- `docs/prd/SY-8.prd.md` - Requirements
- `docs/research/SY-8.md` - Technical research
- `docs/vision.md` - StreamEvent design, async patterns
- `docs/conventions.md` - Code standards
- `synapse-core/src/provider/deepseek.rs` - Current DeepSeekProvider
- [DeepSeek API Documentation](https://api-docs.deepseek.com/)
- [eventsource-stream crate](https://docs.rs/eventsource-stream/)
- [async-stream crate](https://docs.rs/async-stream/)
