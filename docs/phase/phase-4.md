# Phase 4: Provider Abstraction

**Goal:** Define LLM provider trait in core.

## Tasks

- [ ] 4.1 Create `synapse-core/src/message.rs` with `Role` enum and `Message` struct
- [ ] 4.2 Create `synapse-core/src/provider.rs` with `LlmProvider` trait (async `complete` method)
- [ ] 4.3 Create `MockProvider` in `provider/mock.rs` returning static response

## Acceptance Criteria

**Test:** Unit test calls `MockProvider::complete()` and gets response.

## Dependencies

- Phase 3 complete (Configuration loading works)

## Implementation Notes

### 4.1 Create message types

Create `synapse-core/src/message.rs` with:
- `Role` enum (User, Assistant, System)
- `Message` struct with `role` and `content` fields
- Export from `lib.rs`

### 4.2 Create LlmProvider trait

Create `synapse-core/src/provider.rs` with:
- `LlmProvider` trait with async `complete` method
- Takes `&[Message]` and returns `Result<Message, ProviderError>`
- Export from `lib.rs`

### 4.3 Create MockProvider

Create `synapse-core/src/provider/mock.rs` with:
- `MockProvider` struct implementing `LlmProvider`
- Returns a static response for testing
- Add unit test verifying the mock works
