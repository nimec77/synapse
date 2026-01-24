# SY-5 Summary: Provider Abstraction

**Status:** Complete
**Date:** 2026-01-24

---

## Overview

SY-5 implements the LLM provider abstraction layer for Synapse (Phase 4). This phase establishes the foundational types and traits that all future LLM provider implementations will use. Following hexagonal architecture principles, this implementation defines the "port" (trait) that represents how the application interacts with LLM providers, without implementing actual provider adapters yet.

The abstraction layer introduces standard message types (`Role`, `Message`), the `LlmProvider` trait with an async `complete` method, and a `MockProvider` for testing purposes. This enables development and testing of higher-level components without requiring real API calls.

---

## What Was Implemented

### Message Types

The `message` module provides the standard conversation representation:

- `Role` enum with three variants: `System`, `User`, `Assistant`
- `Message` struct containing `role` and `content` fields
- `Message::new()` constructor accepting `&str` or `String`

### Provider Trait

The `provider` module defines the contract for all LLM providers:

- `LlmProvider` trait with async `complete(&self, &[Message]) -> Result<Message, ProviderError>`
- `Send + Sync` bounds for thread-safe async usage
- Object-safe design enabling dynamic dispatch via `Box<dyn LlmProvider>`

### Error Handling

The `ProviderError` enum provides typed errors:

| Variant | Description |
|---------|-------------|
| `ProviderError { message }` | Error response from the provider |
| `RequestFailed(String)` | Network or connection issues |

### Mock Provider

The `MockProvider` enables testing without real API calls:

- Builder pattern with `with_response()` for configurable responses
- LIFO ordering: last added response is returned first
- Falls back to "Mock response" when queue is exhausted
- Thread-safe with `Mutex<Vec<Message>>` for response storage

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| async-trait crate | `async-trait = "0.1"` | Ergonomic async traits; minor heap allocation acceptable for LLM calls |
| Role::Tool deferred | Not included | Postponed to Phase 10 (MCP Integration) |
| Message fields | Minimal: role + content | Fields like timestamp, tool_calls added when needed |
| MockProvider mutex | `Mutex<Vec<Message>>` | Thread-safe LIFO queue for test responses |
| Mutex poisoning | Graceful recovery via `into_inner()` | No unwrap/expect in library code |
| Builder pattern | `#[must_use]` on `with_response()` | Prevents forgotten builder calls |

---

## Files Created

| File | Purpose |
|------|---------|
| `synapse-core/src/message.rs` | Role enum, Message struct with constructor |
| `synapse-core/src/provider.rs` | LlmProvider trait, ProviderError enum, mock submodule |
| `synapse-core/src/provider/mock.rs` | MockProvider implementation with tests |

---

## Files Modified

| File | Change |
|------|--------|
| `synapse-core/Cargo.toml` | Added tokio (rt, macros) and async-trait dependencies |
| `synapse-core/src/lib.rs` | Added pub mod message/provider and re-exports |

---

## API Contract

### Role Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,     // System instructions for the model
    User,       // User input
    Assistant,  // Model response
}
```

### Message Struct

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self;
}
```

### LlmProvider Trait

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

### MockProvider

```rust
#[derive(Debug, Default)]
pub struct MockProvider { /* ... */ }

impl MockProvider {
    pub fn new() -> Self;
    #[must_use]
    pub fn with_response(self, content: impl Into<String>) -> Self;
}
```

### Public Exports

From `synapse_core`:
- `synapse_core::Message`
- `synapse_core::Role`
- `synapse_core::LlmProvider`
- `synapse_core::MockProvider`
- `synapse_core::ProviderError`
- `synapse_core::message` (module)
- `synapse_core::provider` (module)

---

## Module Structure

The implementation follows the new Rust module system (no mod.rs files):

```
synapse-core/src/
  lib.rs              # pub mod message; pub mod provider;
  message.rs          # Role, Message
  provider.rs         # mod mock; LlmProvider, ProviderError
  provider/
    mock.rs           # MockProvider
```

---

## Testing Summary

### Unit Tests (10 tests)

| Module | Test | Coverage |
|--------|------|----------|
| message.rs | `test_message_new_with_str` | &str input |
| message.rs | `test_message_new_with_string` | String input |
| message.rs | `test_role_equality` | Equality/inequality |
| message.rs | `test_message_clone` | Clone trait |
| message.rs | `test_role_copy` | Copy trait |
| mock.rs | `test_mock_provider_default_response` | Default behavior |
| mock.rs | `test_mock_provider_configured_response` | Custom response |
| mock.rs | `test_mock_provider_multiple_responses` | LIFO ordering |
| mock.rs | `test_mock_provider_with_string` | String input |
| mock.rs | `test_llmprovider_is_object_safe` | Trait object usage |

Run tests:
```bash
cargo test -p synapse-core
```

### Quality Checks

| Check | Status |
|-------|--------|
| `cargo fmt --check` | Pass |
| `cargo clippy -- -D warnings` | Pass |
| `cargo test` | Pass (10 tests for SY-5) |
| `cargo build` | Pass |

---

## Usage Examples

### Creating Messages

```rust
use synapse_core::{Message, Role};

let system_msg = Message::new(Role::System, "You are a helpful assistant.");
let user_msg = Message::new(Role::User, "Hello!");
let assistant_msg = Message::new(Role::Assistant, "Hi there!");
```

### Using MockProvider for Testing

```rust
use synapse_core::{LlmProvider, MockProvider, Message, Role};

#[tokio::test]
async fn test_with_mock() {
    let provider = MockProvider::new()
        .with_response("Hello from mock!");
    let messages = vec![Message::new(Role::User, "Hi")];

    let response = provider.complete(&messages).await.unwrap();
    assert_eq!(response.content, "Hello from mock!");
}
```

### Provider-Agnostic Code

```rust
use synapse_core::{LlmProvider, Message, ProviderError};

async fn run_agent<P: LlmProvider>(
    provider: &P,
    messages: &[Message]
) -> Result<Message, ProviderError> {
    provider.complete(messages).await
}
```

### Trait Object Usage

```rust
use synapse_core::{LlmProvider, MockProvider};

let provider: Box<dyn LlmProvider> = Box::new(MockProvider::new());
```

---

## Dependencies Added

### synapse-core

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 (rt, macros) | Async runtime for tests |
| `async-trait` | 0.1 | Async trait support |

---

## Design Rationale

### Hexagonal Architecture

The `LlmProvider` trait serves as a "port" in hexagonal architecture terminology. Each future provider (Anthropic, OpenAI) will be an "adapter" that implements this port. This separation allows:

- Swapping providers via configuration without code changes
- Testing business logic with MockProvider
- Adding new providers without modifying existing code

### Thread Safety

The `Send + Sync` bounds on `LlmProvider` enable:

- Sharing providers across tokio tasks
- Using providers with `Arc` for concurrent access
- Safe integration with async runtimes

### Object Safety

The trait design ensures object safety, allowing:

- Runtime provider selection: `Box<dyn LlmProvider>`
- Provider factories that return different implementations
- Dynamic dispatch when static polymorphism is not needed

---

## Limitations

1. **No streaming support**: The `complete` method returns a full response; streaming will be added in a future phase
2. **No tool/function calling**: `Role::Tool` and tool-related fields deferred to Phase 10
3. **Minimal error types**: `ProviderError` will be extended in Phase 5 with HTTP-specific variants
4. **No message metadata**: Fields like timestamp or token counts not included yet

---

## Future Enhancements

This abstraction layer will be extended in subsequent phases:

1. **Phase 5**: Anthropic provider implementation with HTTP client
2. **Phase 6**: Streaming response support
3. **Phase 9**: OpenAI provider implementation
4. **Phase 10**: Tool/function calling with `Role::Tool`
5. **Later**: Additional message fields (timestamp, tool_calls, token_usage)

---

## QA Status

The QA report (`reports/qa/SY-5.md`) recommends **RELEASE**:

- All 6 tasks complete
- All 4 PRD goals met
- All 4 user stories satisfied
- 10 unit tests passing
- Clean architecture maintained
- Thread-safe design verified
- Implementation matches approved plan
