# SY-5 Implementation Plan: Phase 4 - Provider Abstraction

Status: PLAN_APPROVED

## Overview

This plan establishes the LLM provider abstraction layer in synapse-core, defining the foundational types and traits that all future provider implementations will use.

---

## Components

### 1. message.rs

**Location:** `synapse-core/src/message.rs`

**Purpose:** Define standard message types for conversation representation.

**Types:**
- `Role` enum - conversation participant roles
- `Message` struct - single conversation message

### 2. provider.rs

**Location:** `synapse-core/src/provider.rs`

**Purpose:** Define the provider trait and error types.

**Types:**
- `ProviderError` enum - typed errors for provider operations
- `LlmProvider` trait - async contract for LLM providers

**Submodules:**
- `mod mock;` - declares the mock provider submodule

### 3. provider/mock.rs

**Location:** `synapse-core/src/provider/mock.rs`

**Purpose:** Test-friendly mock provider implementation.

**Types:**
- `MockProvider` struct - configurable mock for testing

---

## API Contract

### Role Enum

```rust
/// Role of a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// System instructions for the model.
    System,
    /// User input.
    User,
    /// Model response.
    Assistant,
}
```

**Notes:**
- `Role::Tool` deferred to Phase 10 (MCP Integration)
- `Copy` trait for ergonomic use without cloning

### Message Struct

```rust
/// A single message in a conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The role of this message.
    pub role: Role,
    /// The text content of this message.
    pub content: String,
}

impl Message {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
}
```

**Notes:**
- Fields like `timestamp`, `tool_calls`, `tool_results` deferred to later phases
- `impl Into<String>` for ergonomic construction from `&str` or `String`

### ProviderError Enum

```rust
/// Error type for provider operations.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The provider returned an error response.
    #[error("provider error: {message}")]
    ProviderError { message: String },

    /// Request failed due to network or connection issues.
    #[error("request failed: {0}")]
    RequestFailed(String),
}
```

**Notes:**
- Minimal error variants for Phase 4
- Phase 5 (Anthropic) will extend with HTTP-specific errors
- Follows existing `ConfigError` pattern from `config.rs`

### LlmProvider Trait

```rust
use async_trait::async_trait;

/// Trait for LLM providers.
///
/// Implementations must be thread-safe (`Send + Sync`) for use
/// in async contexts.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages to the LLM and get a response.
    ///
    /// # Arguments
    /// * `messages` - Conversation history to send to the model
    ///
    /// # Returns
    /// The assistant's response message, or an error if the request failed.
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

**Notes:**
- Uses `async-trait` crate per research decision
- `Send + Sync` bounds required for tokio async contexts
- Object-safe design (no generics in trait, no `Self` in return types)

### MockProvider Struct

```rust
use std::sync::Mutex;

/// A mock LLM provider for testing.
///
/// Returns configurable responses. If no responses are configured,
/// returns a default response.
#[derive(Debug, Default)]
pub struct MockProvider {
    responses: Mutex<Vec<Message>>,
}

impl MockProvider {
    /// Create a new mock provider with no predefined responses.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a response to be returned on the next call to `complete`.
    ///
    /// Responses are returned in LIFO order (last added = first returned).
    pub fn with_response(self, content: impl Into<String>) -> Self {
        // Note: Uses expect() with message - acceptable in test-only code
        self.responses
            .lock()
            .expect("MockProvider mutex poisoned")
            .push(Message::new(Role::Assistant, content));
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _messages: &[Message]) -> Result<Message, ProviderError> {
        let mut responses = self.responses
            .lock()
            .expect("MockProvider mutex poisoned");
        if let Some(response) = responses.pop() {
            Ok(response)
        } else {
            Ok(Message::new(Role::Assistant, "Mock response"))
        }
    }
}
```

**Notes:**
- Builder pattern with `with_response()` for fluent test setup
- Uses `Mutex` for interior mutability (thread-safe configurable responses)
- `expect()` with descriptive message acceptable in test-only code
- LIFO ordering (stack behavior) - documented in method doc
- Default fallback response when queue is empty

---

## Data Flow

### Complete Request Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Consumer Code                               │
│  (Agent, CLI, Tests)                                                │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ messages: &[Message]
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    LlmProvider::complete()                          │
│                    (trait method)                                   │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
                    ▼                               ▼
    ┌───────────────────────────┐   ┌───────────────────────────┐
    │      MockProvider         │   │   Future: Anthropic,      │
    │   (returns configured     │   │   OpenAI providers        │
    │    or default response)   │   │   (Phase 5, Phase 9)      │
    └───────────────────────────┘   └───────────────────────────┘
                    │                               │
                    ▼                               ▼
            Result<Message, ProviderError>
```

### Message Construction Flow

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Role::System    │     │  Role::User      │     │  Role::Assistant │
│  + content       │     │  + content       │     │  + content       │
└────────┬─────────┘     └────────┬─────────┘     └────────┬─────────┘
         │                        │                        │
         ▼                        ▼                        ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Vec<Message>                                │
│                    (conversation history)                           │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
                         LlmProvider::complete()
```

---

## File Structure

### After Implementation

```
synapse-core/
├── Cargo.toml              # + tokio, async-trait dependencies
└── src/
    ├── lib.rs              # + pub mod message; pub mod provider;
    ├── config.rs           # Unchanged
    ├── message.rs          # NEW: Role, Message
    ├── provider.rs         # NEW: LlmProvider, ProviderError, mod mock
    └── provider/
        └── mock.rs         # NEW: MockProvider
```

### Module Exports (lib.rs)

```rust
pub mod config;
pub mod message;
pub mod provider;

pub use config::{Config, ConfigError};
pub use message::{Message, Role};
pub use provider::{LlmProvider, MockProvider, ProviderError};
```

---

## Dependencies

### New Dependencies for synapse-core

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | Async runtime |
| `async-trait` | 0.1 | Async trait support |

### Cargo.toml Addition

```toml
[dependencies]
# ... existing dependencies ...
tokio = { version = "1", features = ["rt"] }
async-trait = "0.1"
```

**Note:** Minimal tokio features (`rt` only). Full features will be added in Phase 5 when HTTP client is needed.

---

## Non-Functional Requirements

| Requirement | Target | Validation |
|-------------|--------|------------|
| Code compiles | `cargo build` passes | CI check |
| Tests pass | All unit tests green | `cargo test` |
| No clippy warnings | Zero warnings | `cargo clippy -- -D warnings` |
| Code formatted | Consistent style | `cargo fmt --check` |
| Documentation | All pub items documented | Clippy `missing_docs` check |
| No unwrap in library | Only expect() in test code | Manual review |
| Thread safety | `Send + Sync` bounds on trait | Compile-time guarantee |

---

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Trait design too restrictive for future providers | Medium | Low | Keep trait minimal; study Claude/OpenAI APIs (already done in research) |
| `async-trait` heap allocation overhead | Low | Certain | Acceptable for LLM calls (network latency dominates); can optimize later |
| Missing fields in Message struct | Low | Medium | Design allows extension; add fields in future phases as needed |
| MockProvider mutex poisoning | Low | Very Low | Uses `expect()` with clear message; only in test code |
| Error type insufficient for real providers | Low | Medium | Minimal starter set; Phase 5 will extend for HTTP errors |

---

## Testing Strategy

### Unit Tests

**Location:** Inline in `synapse-core/src/provider/mock.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_default_response() {
        let provider = MockProvider::new();
        let messages = vec![Message::new(Role::User, "Hello")];

        let response = provider.complete(&messages).await.unwrap();

        assert_eq!(response.role, Role::Assistant);
        assert_eq!(response.content, "Mock response");
    }

    #[tokio::test]
    async fn test_mock_provider_configured_response() {
        let provider = MockProvider::new()
            .with_response("Custom response");
        let messages = vec![Message::new(Role::User, "Hello")];

        let response = provider.complete(&messages).await.unwrap();

        assert_eq!(response.role, Role::Assistant);
        assert_eq!(response.content, "Custom response");
    }

    #[tokio::test]
    async fn test_mock_provider_multiple_responses() {
        let provider = MockProvider::new()
            .with_response("First")
            .with_response("Second");
        let messages = vec![Message::new(Role::User, "Hello")];

        // LIFO order: Second returned first
        let r1 = provider.complete(&messages).await.unwrap();
        assert_eq!(r1.content, "Second");

        let r2 = provider.complete(&messages).await.unwrap();
        assert_eq!(r2.content, "First");

        // Falls back to default
        let r3 = provider.complete(&messages).await.unwrap();
        assert_eq!(r3.content, "Mock response");
    }
}
```

**Additional tests for message.rs:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_new_with_str() {
        let msg = Message::new(Role::User, "Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_message_new_with_string() {
        let msg = Message::new(Role::Assistant, String::from("Response"));
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Response");
    }

    #[test]
    fn test_role_equality() {
        assert_eq!(Role::System, Role::System);
        assert_ne!(Role::User, Role::Assistant);
    }
}
```

---

## Implementation Tasks

1. **Add dependencies** to `synapse-core/Cargo.toml`
   - Add `tokio = { version = "1", features = ["rt"] }`
   - Add `async-trait = "0.1"`

2. **Create message.rs**
   - Define `Role` enum with `System`, `User`, `Assistant`
   - Define `Message` struct with `role` and `content`
   - Implement `Message::new()` constructor
   - Add unit tests

3. **Create provider.rs**
   - Define `ProviderError` enum with `thiserror`
   - Define `LlmProvider` trait with `async complete()`
   - Declare `mod mock;` submodule
   - Re-export `MockProvider`

4. **Create provider/mock.rs**
   - Implement `MockProvider` struct
   - Add `new()` and `with_response()` builder methods
   - Implement `LlmProvider` trait
   - Add unit tests

5. **Update lib.rs**
   - Add `pub mod message;` and `pub mod provider;`
   - Add re-exports for key types

6. **Verify**
   - Run `cargo fmt`
   - Run `cargo clippy -- -D warnings`
   - Run `cargo test`

---

## Open Questions

None. All design decisions have been made:
- Use `async-trait` crate (confirmed in research)
- MockProvider uses builder pattern with configurable responses (confirmed in research)
- Error type is minimal, will be extended in Phase 5
- Message type is minimal, will be extended as needed
