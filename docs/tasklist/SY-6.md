# SY-6: Phase 5: Anthropic Provider

Status: IMPLEMENT_STEP_OK

Context: Implements real Claude API integration. PRD `docs/prd/SY-6.prd.md`; Plan `docs/plan/SY-6.md`.

---

## Tasks

### 1. Add Dependencies to synapse-core

- [x] Add `reqwest` (with `json` feature) and `serde_json` to `synapse-core/Cargo.toml`
  - **AC1:** `cargo check -p synapse-core` passes
  - **AC2:** `Cargo.toml` contains `reqwest = { version = "0.12", features = ["json"] }` and `serde_json = "1"`

### 2. Add Dependencies to synapse-cli

- [x] Add `tokio` (with `rt-multi-thread`, `macros` features) and `anyhow` to `synapse-cli/Cargo.toml`
  - **AC1:** `cargo check -p synapse-cli` passes
  - **AC2:** `Cargo.toml` contains `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }` and `anyhow = "1"`

### 3. Extend ProviderError

- [x] Add `AuthenticationError(String)` variant to `ProviderError` in `synapse-core/src/provider.rs`
  - **AC1:** Enum has `#[error("authentication failed: {0}")]` variant
  - **AC2:** `cargo check -p synapse-core` passes

### 4. Create AnthropicProvider Module

- [x] Create `synapse-core/src/provider/anthropic.rs` with `AnthropicProvider` struct and internal API types
  - **AC1:** File contains `AnthropicProvider` struct with `client: reqwest::Client`, `api_key: String`, `model: String` fields
  - **AC2:** File contains private types: `ApiRequest`, `ApiMessage`, `ApiResponse`, `ContentBlock`, `ApiError`, `ErrorDetail`
  - **AC3:** `AnthropicProvider::new(api_key, model)` constructor exists

### 5. Implement LlmProvider Trait for AnthropicProvider

- [x] Implement `LlmProvider` trait with async `complete` method
  - **AC1:** Method sends POST to `https://api.anthropic.com/v1/messages` with correct headers (`x-api-key`, `anthropic-version: 2023-06-01`, `content-type: application/json`)
  - **AC2:** System messages are extracted to `system` field in request; user/assistant messages go to `messages` array
  - **AC3:** HTTP errors are mapped: 401 -> `AuthenticationError`, 4xx/5xx/network -> `RequestFailed` or `ProviderError`

### 6. Update Module Exports

- [x] Update `synapse-core/src/provider.rs` to declare `mod anthropic` and `pub use anthropic::AnthropicProvider`
  - **AC1:** `use synapse_core::AnthropicProvider;` compiles from external crate
- [x] Update `synapse-core/src/lib.rs` to export `AnthropicProvider`
  - **AC1:** `lib.rs` exports `AnthropicProvider` in public API

### 7. Make CLI Async with Tokio

- [x] Convert `synapse-cli/src/main.rs` to use `#[tokio::main]` and return `Result<()>` via `anyhow`
  - **AC1:** `main` function is `async fn main() -> anyhow::Result<()>`
  - **AC2:** Uses `anyhow::Context` for error context

### 8. Wire AnthropicProvider into CLI

- [x] Replace echo logic with provider-based completion
  - **AC1:** CLI loads config, validates `api_key` is present (fails with clear message if missing)
  - **AC2:** Creates `AnthropicProvider` with api_key and model from config
  - **AC3:** Calls `provider.complete(&messages).await` and prints `response.content`
  - **AC4:** Both one-shot (`synapse "msg"`) and piped input (`echo "msg" | synapse`) work

### 9. Unit Tests for AnthropicProvider

- [x] Add unit tests in `synapse-core/src/provider/anthropic.rs`
  - **AC1:** `test_api_request_serialization` verifies JSON structure matches Anthropic API format
  - **AC2:** `test_api_response_parsing` verifies response deserialization extracts text content
  - **AC3:** `test_system_message_extraction` verifies system role messages are separated to `system` field
  - **AC4:** All tests pass with `cargo test -p synapse-core`

### 10. Final Verification

- [x] Run full CI checks
  - **AC1:** `cargo fmt --check` passes
  - **AC2:** `cargo clippy -- -D warnings` passes
  - **AC3:** `cargo test` passes (all crates)
