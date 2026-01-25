# SY-8 Summary: Phase 7 - Streaming Responses

**Status:** COMPLETE
**Date:** 2026-01-25

---

## Overview

SY-8 implements streaming responses for the Synapse CLI, enabling token-by-token output display for real-time feedback. Instead of waiting for the complete LLM response before displaying it, tokens now appear progressively as they arrive from the provider. This significantly improves user experience for longer responses.

The key change is that running `synapse "Count from 1 to 10"` now shows each number appearing progressively rather than all at once after the full response is ready.

---

## What Was Built

### New Components

1. **StreamEvent Enum** (`synapse-core/src/provider/streaming.rs`)
   - `TextDelta(String)` - Text fragment from the response
   - `ToolCall { id, name, input }` - Reserved for MCP integration (Phase 11)
   - `ToolResult { id, output }` - Reserved for MCP integration (Phase 11)
   - `Done` - Stream completed successfully
   - `Error(ProviderError)` - Error during streaming
   - Derives `Debug` and `Clone` for flexibility

2. **DeepSeek Streaming Implementation** (`synapse-core/src/provider/deepseek.rs`)
   - `StreamingApiRequest` struct with `stream: true` parameter
   - `StreamChunk`, `StreamChoice`, `StreamDelta` types for SSE parsing
   - Uses `eventsource-stream` for Server-Sent Events parsing
   - Uses `async_stream::stream!` macro for clean async stream generation
   - Handles `[DONE]` marker, empty content filtering, and error propagation

3. **CLI Streaming Output** (`synapse-cli/src/main.rs`)
   - Replaced `complete()` call with `stream()`
   - Uses `tokio::select!` with `tokio::signal::ctrl_c()` for interruption handling
   - Prints `[Interrupted]` on Ctrl+C and exits cleanly
   - Uses `print!()` with `stdout.flush()` for immediate token display

### Modified Components

1. **LlmProvider Trait** (`synapse-core/src/provider.rs`)
   - Added `stream()` method to trait
   - Return type: `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>`
   - Object-safe design enables use with `Box<dyn LlmProvider>`

2. **MockProvider** (`synapse-core/src/provider/mock.rs`)
   - Added `stream_tokens: Mutex<Vec<String>>` field
   - Added `with_stream_tokens()` builder method
   - `stream()` yields configured tokens or falls back to `complete()`

3. **AnthropicProvider** (`synapse-core/src/provider/anthropic.rs`)
   - Added fallback `stream()` implementation
   - Calls `complete()` and yields full response as single `TextDelta`
   - True Anthropic streaming deferred to future phase

4. **Dependencies**
   - `synapse-core/Cargo.toml`: Added `eventsource-stream = "0.2"`, `async-stream = "0.3"`, `futures = "0.3"`, enabled reqwest `stream` feature
   - `synapse-cli/Cargo.toml`: Added `futures = "0.3"`, enabled tokio `signal` and `io-std` features

---

## Key Decisions

### 1. Object-Safe Stream Return Type
The `stream()` method returns `Pin<Box<dyn Stream<...> + Send + '_>>` instead of `impl Stream`. This preserves object safety required for `Box<dyn LlmProvider>` used in the provider factory pattern.

### 2. No Default Trait Implementation
Each provider implements `stream()` explicitly rather than using a default implementation. This avoids complexity with async trait methods and makes provider behavior explicit.

### 3. Clone Messages at Stream Creation
Messages are cloned when creating the stream to avoid lifetime complexity. Since messages are typically small (a few KB), this is an acceptable tradeoff for simpler code.

### 4. Empty Content Filtering
Empty delta content from SSE events is filtered at the provider level. Only non-empty `TextDelta` events reach the CLI, simplifying output handling.

### 5. Ctrl+C via tokio::select!
Graceful interruption uses `tokio::select!` with `tokio::signal::ctrl_c()`. This integrates cleanly with the async stream consumption loop and properly handles cleanup.

### 6. Fallback for Non-Streaming Providers
Anthropic uses a fallback that wraps `complete()`, yielding the full response as a single `TextDelta`. This maintains API consistency while deferring true Anthropic streaming to a future phase.

---

## SSE Format (OpenAI-compatible)

DeepSeek uses the OpenAI-compatible SSE streaming format:

```
data: {"choices":[{"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"choices":[{"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"choices":[{"delta":{"content":" world"},"finish_reason":null}]}

data: {"choices":[{"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

Key parsing rules:
- Lines prefixed with `data: `
- Content in `delta.content` (not `message.content`)
- First event may have `delta.role` without content
- Final delta may have empty content with `finish_reason`
- Stream ends with literal `data: [DONE]`

---

## Testing

### Unit Tests for StreamEvent (3 tests)
- `test_stream_event_variants` - All 5 variants can be constructed
- `test_stream_event_debug` - Debug output contains variant name
- `test_stream_event_clone` - Clone semantics work correctly

### Unit Tests for DeepSeek Streaming (5 tests)
- `test_streaming_request_serialization` - JSON includes `stream: true`
- `test_parse_sse_text_delta` - Delta content extraction
- `test_parse_sse_done` - `[DONE]` marker and finish_reason handling
- `test_parse_sse_empty_content` - Empty content skipped
- `test_parse_sse_with_role` - First chunk with role only

### Unit Tests for MockProvider Streaming (3 tests)
- `test_mock_stream_tokens` - Configured tokens yielded
- `test_mock_stream_fallback` - Falls back to complete()
- `test_mock_stream_ends_with_done` - Always ends with Done

### Unit Tests for AnthropicProvider Streaming (1 test)
- `test_anthropic_provider_implements_stream` - Trait compliance

**Total new tests: 12**

---

## Usage

### Streaming Output (Default)
```bash
# Tokens appear progressively
DEEPSEEK_API_KEY=sk-... synapse "Count from 1 to 10 slowly"
# Output: 1... 2... 3... (appearing incrementally)
```

### Interrupt Long Response
```bash
DEEPSEEK_API_KEY=sk-... synapse "Write a long essay about AI"
# Press Ctrl+C after a few words
# Output: [partial text]
# [Interrupted]
```

### Anthropic Fallback
```bash
# Full response appears at once (not progressive)
ANTHROPIC_API_KEY=sk-ant-... synapse "Hello"
# (with provider = "anthropic" in config)
```

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-core/src/provider/streaming.rs` | New file - StreamEvent enum |
| `synapse-core/src/provider.rs` | Added `mod streaming`, `stream()` to trait |
| `synapse-core/src/provider/deepseek.rs` | SSE streaming implementation |
| `synapse-core/src/provider/anthropic.rs` | Fallback streaming |
| `synapse-core/src/provider/mock.rs` | Configurable stream tokens |
| `synapse-core/src/lib.rs` | Export StreamEvent |
| `synapse-core/Cargo.toml` | Added streaming dependencies |
| `synapse-cli/src/main.rs` | Stream consumption with Ctrl+C handling |
| `synapse-cli/Cargo.toml` | Added futures, tokio features |

---

## Module Structure

```
synapse-core/src/
  lib.rs              # pub use provider::{StreamEvent, ...}
  provider.rs         # mod streaming; LlmProvider::stream()
  provider/
    anthropic.rs      # stream() fallback to complete()
    deepseek.rs       # stream() with SSE parsing
    factory.rs        # create_provider() (unchanged)
    mock.rs           # stream() with configurable tokens
    streaming.rs      # StreamEvent enum

synapse-cli/src/
  main.rs             # Uses stream() with tokio::select!
```

---

## Performance Characteristics

| Aspect | Value |
|--------|-------|
| First token latency | Limited by provider, not client |
| Memory per token | O(1) - print and discard |
| Stdout flush | After each token |
| Backpressure | N/A (LLM output << terminal speed) |

---

## Security Considerations

- HTTPS enforced via hardcoded API endpoint
- API key never logged or included in error messages
- Clean stream termination on interrupt
- Partial output visible on error (expected behavior for streaming)

---

## Future Work

This implementation enables:
- **Anthropic Streaming** - True SSE streaming for Anthropic API
- **OpenAI Provider** - Can reuse same streaming pattern
- **MCP Integration** - ToolCall/ToolResult variants ready for Phase 11
- **REPL Mode** - Streaming output prepared for interactive mode
- **Response Buffering** - Optional full response capture for session storage
