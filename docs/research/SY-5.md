# SY-5 Research: Phase 4 - Provider Abstraction

## Resolved Questions

User preferences confirmed during research initiation:

1. **Async traits implementation**: Use the `async-trait` crate (not native async in traits)
   - Rationale: More mature, well-documented approach with broad ecosystem support

2. **MockProvider behavior**: Configurable responses (accepts predefined responses for testing)
   - Rationale: Enables flexible testing scenarios without hardcoded responses

---

## Related Modules/Services

### Existing synapse-core Structure

| File | Purpose | Relevance |
|------|---------|-----------|
| `synapse-core/src/lib.rs` | Public API exports | Will export new `message` and `provider` modules |
| `synapse-core/src/config.rs` | Configuration management | Reference pattern for module structure, error handling |

### Current lib.rs Exports

```rust
pub mod config;
pub use config::{Config, ConfigError};
```

The new modules will follow the same pattern:
- `pub mod message;`
- `pub mod provider;`
- Re-export key types at crate root

---

## Current Endpoints and Contracts

### No External APIs Yet

This phase introduces the **internal contract** (trait) that future phases will implement:

- Phase 5 will add `AnthropicProvider` implementing `LlmProvider`
- Phase 9 will add `OpenAIProvider` implementing `LlmProvider`

### Trait Contract (to be defined)

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

Key requirements:
- `Send + Sync` bounds for use in async contexts
- Takes message slice (conversation history)
- Returns `Result` with typed error

---

## Patterns Used

### Error Handling Pattern (from config.rs)

The config module uses `thiserror` with structured error variants:

```rust
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file '{path}': {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
    // ...
}
```

**Apply to `ProviderError`:**
- Create `synapse-core/src/error.rs` (or inline in `provider.rs` initially)
- Use `#[error(...)]` with descriptive messages
- Include context in error variants

### Module Organization Pattern

Following Rust 2018+ style (no `mod.rs`):

```
synapse-core/src/
├── lib.rs              # pub mod provider; pub mod message;
├── config.rs
├── message.rs          # Role, Message types
├── provider.rs         # LlmProvider trait + mod mock;
└── provider/
    └── mock.rs         # MockProvider implementation
```

### Derive Pattern

Config uses:
```rust
#[derive(Debug, Clone, PartialEq, Deserialize)]
```

Message types should use:
```rust
#[derive(Debug, Clone, PartialEq)]  // No Deserialize needed yet
```

---

## Current Dependencies

### synapse-core/Cargo.toml

```toml
[dependencies]
dirs = "6.0.0"
serde = { version = "1", features = ["derive"] }
thiserror = "2"
toml = "0.9.8"
```

### New Dependencies Required

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | Async runtime (already in vision.md) |
| `async-trait` | 0.1 | Async trait support |

**Note:** `tokio` may only need `rt` feature for now; full features come with actual providers in Phase 5.

---

## Implementation Notes

### 1. message.rs

**Location:** `synapse-core/src/message.rs`

**Types to define:**

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
- `Role::Tool` is deferred to Phase 10 (MCP Integration) per PRD
- `timestamp`, `tool_calls`, `tool_results` fields deferred to later phases
- Use `impl Into<String>` for ergonomic construction

### 2. provider.rs

**Location:** `synapse-core/src/provider.rs`

**Structure:**

```rust
//! LLM provider abstraction layer.

mod mock;  // Submodule declaration

pub use mock::MockProvider;

use async_trait::async_trait;
use crate::message::Message;

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

/// Trait for LLM providers.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages to the LLM and get a response.
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

**Notes:**
- Error variants are minimal; Phase 5 will expand for HTTP errors
- Trait is object-safe (no generics, no `Self` in return types)
- `Send + Sync` required for use with tokio

### 3. provider/mock.rs

**Location:** `synapse-core/src/provider/mock.rs`

**Implementation (configurable responses):**

```rust
//! Mock provider for testing.

use std::sync::Mutex;

use async_trait::async_trait;

use super::{LlmProvider, ProviderError};
use crate::message::{Message, Role};

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
    pub fn with_response(self, content: impl Into<String>) -> Self {
        self.responses.lock().unwrap().push(Message::new(Role::Assistant, content));
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _messages: &[Message]) -> Result<Message, ProviderError> {
        let mut responses = self.responses.lock().unwrap();
        if let Some(response) = responses.pop() {
            Ok(response)
        } else {
            Ok(Message::new(Role::Assistant, "Mock response"))
        }
    }
}
```

**Notes:**
- Uses `Mutex` for interior mutability (configurable responses)
- Builder pattern with `with_response()` for test setup
- Default response when queue is empty
- `unwrap()` is acceptable in test-only code, but consider `expect()` with message

**Alternative consideration:** Could use `VecDeque` for FIFO ordering if tests need sequential responses.

---

## Limitations and Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `async-trait` overhead | Low - small runtime cost for heap allocation | Acceptable for this use case; can optimize later if needed |
| Trait not object-safe | Medium - would limit dynamic dispatch | Current design is object-safe; maintain this |
| Error type too limited | Low - can extend later | Start minimal, add variants in Phase 5 |
| Mock uses `unwrap()` | Low - only in test code | Use `expect()` with descriptive message |

---

## New Technical Questions

1. **Should `ProviderError` be in its own `error.rs` file?**
   - PRD mentions Phase 5 will create `synapse-core/src/error.rs`
   - For Phase 4, keeping error inline in `provider.rs` is cleaner
   - Can refactor to separate file in Phase 5 when more error types are needed

2. **Should `MockProvider` use `VecDeque` for FIFO responses?**
   - Current design uses `Vec` with `pop()` (LIFO)
   - FIFO might be more intuitive for sequential test scenarios
   - Decision: Start with simple `Vec`, refactor if tests need FIFO

3. **Integration tests location?**
   - PRD mentions `tests/` directory
   - Currently no integration tests exist
   - Phase 4 can use unit tests in `provider/mock.rs`; integration tests can come later

---

## File Structure After Phase 4

```
synapse-core/
├── Cargo.toml              # + tokio, async-trait
└── src/
    ├── lib.rs              # + pub mod message; pub mod provider;
    ├── config.rs           # Unchanged
    ├── message.rs          # NEW: Role, Message
    ├── provider.rs         # NEW: LlmProvider trait, ProviderError
    └── provider/
        └── mock.rs         # NEW: MockProvider
```

---

## Checklist for Implementation

- [ ] Add `tokio` and `async-trait` to `synapse-core/Cargo.toml`
- [ ] Create `synapse-core/src/message.rs` with `Role` and `Message`
- [ ] Create `synapse-core/src/provider.rs` with `LlmProvider` trait and `ProviderError`
- [ ] Create `synapse-core/src/provider/mock.rs` with `MockProvider`
- [ ] Update `synapse-core/src/lib.rs` to export new modules
- [ ] Add unit tests for `MockProvider`
- [ ] Run `cargo fmt`, `cargo clippy`, `cargo test`
