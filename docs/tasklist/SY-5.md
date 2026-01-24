# SY-5: Phase 4 - Provider Abstraction

Status: IMPLEMENT_STEP_OK

Context: PRD `docs/prd/SY-5.prd.md`; Plan `docs/plan/SY-5.md`

This phase establishes the LLM provider abstraction layer in synapse-core, defining foundational types (`Role`, `Message`), the `LlmProvider` trait, and a `MockProvider` for testing.

---

## Tasks

- [x] **Task 1: Add async dependencies to synapse-core**
  - Add `tokio = { version = "1", features = ["rt"] }` to `synapse-core/Cargo.toml`
  - Add `async-trait = "0.1"` to `synapse-core/Cargo.toml`
  - **Acceptance Criteria:**
    - `cargo build -p synapse-core` succeeds
    - Both dependencies appear in `Cargo.lock`

- [x] **Task 2: Create message.rs with Role and Message types**
  - Create `synapse-core/src/message.rs`
  - Define `Role` enum with `System`, `User`, `Assistant` variants (derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`)
  - Define `Message` struct with `role: Role` and `content: String` fields (derive `Debug`, `Clone`, `PartialEq`)
  - Implement `Message::new(role, content: impl Into<String>)` constructor
  - Add doc comments for all public items
  - Add unit tests for Message construction and Role equality
  - **Acceptance Criteria:**
    - `cargo test -p synapse-core message` passes
    - `Message::new(Role::User, "test")` creates a message with correct role and content
    - `Message::new(Role::User, String::from("test"))` also works (Into<String>)

- [x] **Task 3: Create provider.rs with ProviderError and LlmProvider trait**
  - Create `synapse-core/src/provider.rs`
  - Define `ProviderError` enum using `thiserror` with `ProviderError { message }` and `RequestFailed(String)` variants
  - Define `LlmProvider` trait with `async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>`
  - Use `#[async_trait]` attribute and require `Send + Sync` bounds
  - Declare `mod mock;` submodule
  - Re-export `MockProvider` from submodule
  - Add doc comments for all public items
  - **Acceptance Criteria:**
    - `cargo build -p synapse-core` succeeds
    - `LlmProvider` trait is object-safe (can be used as `dyn LlmProvider`)

- [x] **Task 4: Create provider/mock.rs with MockProvider**
  - Create `synapse-core/src/provider/mock.rs`
  - Implement `MockProvider` struct with `Mutex<Vec<Message>>` for configurable responses
  - Implement `MockProvider::new()` constructor
  - Implement `with_response(content: impl Into<String>)` builder method (returns Self)
  - Implement `LlmProvider` trait for `MockProvider` (returns configured response or default "Mock response")
  - Add doc comments for all public items
  - Add unit tests: default response, configured response, multiple responses (LIFO order)
  - **Acceptance Criteria:**
    - `cargo test -p synapse-core mock` passes
    - `MockProvider::new().complete(&messages).await` returns default "Mock response"
    - `MockProvider::new().with_response("Hello").complete(&messages).await` returns "Hello"
    - Multiple responses are returned in LIFO order

- [x] **Task 5: Update lib.rs with module exports**
  - Add `pub mod message;` to `synapse-core/src/lib.rs`
  - Add `pub mod provider;` to `synapse-core/src/lib.rs`
  - Add re-exports: `pub use message::{Message, Role};`
  - Add re-exports: `pub use provider::{LlmProvider, MockProvider, ProviderError};`
  - **Acceptance Criteria:**
    - `cargo build -p synapse-core` succeeds
    - External crates can use `synapse_core::{Message, Role, LlmProvider, MockProvider, ProviderError}`

- [x] **Task 6: Final verification**
  - Run `cargo fmt` on workspace
  - Run `cargo clippy -- -D warnings` on workspace
  - Run `cargo test` on workspace
  - Verify all public items have documentation
  - **Acceptance Criteria:**
    - `cargo fmt --check` passes (no formatting issues)
    - `cargo clippy -- -D warnings` passes (zero warnings)
    - `cargo test` passes (all tests green)
    - All new public types and functions have doc comments
