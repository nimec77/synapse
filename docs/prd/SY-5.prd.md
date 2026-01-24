# SY-5: Phase 4: Provider Abstraction

Status: PRD_READY

## Context / Idea

This phase establishes the foundational LLM provider abstraction layer in synapse-core. Following the hexagonal architecture outlined in the vision document, this phase creates the "ports" (traits) that define how the application interacts with LLM providers, without implementing actual provider adapters yet.

The goal is to define the core message types (`Role`, `Message`) and the `LlmProvider` trait with an async `complete` method. A `MockProvider` will be implemented for testing purposes, enabling development and testing of higher-level components without requiring real API calls.

This is a critical architectural milestone as it establishes:
- The contract that all LLM providers must fulfill
- Standard message types used throughout the application
- A testable foundation for the agent orchestrator (future phase)

### Project Context

From `docs/idea.md`: Synapse is an AI agent supporting multiple LLM providers (Anthropic Claude, OpenAI, etc.) configured via TOML, with session-based conversation memory.

From `docs/vision.md`: The architecture follows hexagonal patterns where `LlmProvider` is a port (trait) with adapters like Anthropic and OpenAI. The `complete` method takes `&[Message]` and returns a response, supporting both streaming and non-streaming modes (streaming comes in a later phase).

### Phase Dependencies

- Phase 3 (SY-4) complete: Configuration loading works
- This phase provides the foundation for Phase 5 (Anthropic provider implementation)

## Goals

1. **Define standard message types** - Create `Role` enum and `Message` struct that represent conversation messages across all providers
2. **Establish provider abstraction** - Define the `LlmProvider` trait as the contract for all LLM provider implementations
3. **Enable testing without real APIs** - Implement `MockProvider` for unit and integration testing
4. **Maintain clean architecture** - Follow hexagonal architecture with traits as ports

## User Stories

1. **As a developer**, I want message types that represent conversation roles and content, so that I can work with a consistent data model across different LLM providers.

2. **As a developer**, I want a provider trait with a clear async interface, so that I can implement new LLM providers following a defined contract.

3. **As a developer**, I want a mock provider implementation, so that I can test agent logic without making real API calls or incurring costs.

4. **As a future consumer of synapse-core**, I want the provider abstraction to be flexible enough to support different providers (Claude, OpenAI, etc.), so that I can switch providers via configuration.

## Main Scenarios

### Scenario 1: Creating Messages
```rust
use synapse_core::message::{Role, Message};

let system_msg = Message::new(Role::System, "You are a helpful assistant.");
let user_msg = Message::new(Role::User, "Hello!");
let assistant_msg = Message::new(Role::Assistant, "Hi there!");
```

### Scenario 2: Using MockProvider for Testing
```rust
use synapse_core::provider::{LlmProvider, MockProvider};
use synapse_core::message::{Role, Message};

#[tokio::test]
async fn test_mock_provider() {
    let provider = MockProvider::new();
    let messages = vec![Message::new(Role::User, "Hello")];

    let response = provider.complete(&messages).await.unwrap();

    assert_eq!(response.role, Role::Assistant);
    assert!(!response.content.is_empty());
}
```

### Scenario 3: Provider-Agnostic Agent Code (Future Use)
```rust
// Agent can work with any LlmProvider implementation
async fn run_agent<P: LlmProvider>(provider: &P, messages: &[Message]) -> Result<Message> {
    provider.complete(messages).await
}
```

## Success / Metrics

| Criterion | Measure |
|-----------|---------|
| Message types defined | `Role` enum with System, User, Assistant variants; `Message` struct with role and content |
| Provider trait defined | `LlmProvider` trait with async `complete` method |
| MockProvider works | Unit test calls `MockProvider::complete()` and receives valid response |
| Code quality | Passes `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` |
| Documentation | All public items have doc comments |

## Constraints and Assumptions

### Constraints

1. **No external HTTP calls** - This phase only defines types and traits; no real API integration
2. **Async runtime** - Use tokio for async operations
3. **Error handling** - Use `thiserror` for typed errors in synapse-core
4. **Module structure** - Follow new Rust module system (no `mod.rs` files)
5. **No `unwrap()`/`expect()`** - Use proper error handling in library code

### Assumptions

1. Phase 3 (Configuration) is complete and working
2. The `complete` method will be extended for streaming in a future phase
3. Tool/function calling support will be added in later phases
4. The `Role::Tool` variant mentioned in vision.md can be added later when MCP support is implemented

### Design Decisions

1. **Simple Message struct first** - Start with basic `role` and `content` fields; add `tool_calls`, `tool_results`, `timestamp` in later phases when needed
2. **Result-based API** - `complete` returns `Result<Message, ProviderError>` for proper error propagation
3. **Trait object compatibility** - Ensure `LlmProvider` is object-safe for dynamic dispatch if needed

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Trait design too restrictive | Medium - Hard to adapt for different providers | Study Claude and OpenAI APIs before finalizing; keep trait minimal |
| Missing fields in Message | Low - Can extend later | Start simple, add fields as needed in subsequent phases |
| Async trait complexity | Low - Well-understood patterns | Use standard async trait patterns with trait_variant or async-trait if needed |

## Open Questions

None - The phase requirements are well-defined in the phase document and align with the architecture in vision.md. The scope is intentionally narrow (message types, trait, mock) to establish the foundation without over-engineering.
