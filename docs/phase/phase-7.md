# Phase 7: Streaming Responses

**Goal:** Token-by-token output to terminal.

## Overview

Implement streaming responses from LLM providers using Server-Sent Events (SSE). This enables real-time token display as the model generates output, providing a better user experience.

## Tasks

- [ ] 7.1 Add `eventsource-stream`, `async-stream`, `futures` to core
- [ ] 7.2 Create `synapse-core/src/provider/streaming.rs` with `StreamEvent` enum
- [ ] 7.3 Implement SSE parsing in `DeepSeekProvider::stream()` method
- [ ] 7.4 Update CLI to print tokens as they arrive

## Technical Details

### StreamEvent Enum

```rust
pub enum StreamEvent {
    TextDelta(String),
    ToolCall { id: String, name: String, input: Value },
    ToolResult { id: String, output: Value },
    Done,
    Error(ProviderError),
}
```

### Provider Trait Extension

```rust
pub trait LlmProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
    fn stream(&self, messages: &[Message]) -> impl Stream<Item = Result<StreamEvent, ProviderError>>;
}
```

### SSE Parsing

Use `eventsource-stream` to parse SSE events from the HTTP response stream:

```rust
use eventsource_stream::Eventsource;

let stream = response.bytes_stream().eventsource();
while let Some(event) = stream.next().await {
    // Parse event data and yield StreamEvent
}
```

## Acceptance Criteria

1. `StreamEvent` enum defined with all event types
2. `LlmProvider` trait has `stream()` method
3. DeepSeek provider implements streaming (default provider)
4. CLI displays tokens as they arrive (progressive output)
5. Streaming can be interrupted with Ctrl+C

## Dependencies

- Phase 6 complete (DeepSeek Provider with factory pattern)
- Providers already support non-streaming API calls

## Test Plan

```bash
# Manual test - should see progressive output
cargo run -p synapse-cli -- "Count from 1 to 10 slowly"

# Unit tests for SSE parsing
cargo test -p synapse-core streaming
```
