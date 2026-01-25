# SY-7: Phase 6: DeepSeek Provider

Status: IMPLEMENT_STEP_OK

Context: PRD `docs/prd/SY-7.prd.md`; Plan `docs/plan/SY-7.md`

---

## Tasks

### 1. Extend ProviderError with Factory Variants

- [x] Add `MissingApiKey(String)` and `UnknownProvider(String)` variants to `ProviderError` in `synapse-core/src/provider.rs`

**Acceptance Criteria:**
- `ProviderError::MissingApiKey("message")` compiles and displays "missing API key: message"
- `ProviderError::UnknownProvider("name")` compiles and displays "unknown provider: name"

---

### 2. Create DeepSeekProvider Module

- [x] Create `synapse-core/src/provider/deepseek.rs` with `DeepSeekProvider` struct
- [x] Add `mod deepseek;` and `pub use deepseek::DeepSeekProvider;` to `synapse-core/src/provider.rs`
- [x] Export `DeepSeekProvider` from `synapse-core/src/lib.rs`

**Acceptance Criteria:**
- `DeepSeekProvider::new(api_key, model)` compiles and creates provider instance
- `use synapse_core::DeepSeekProvider;` works from external crates

---

### 3. Implement LlmProvider Trait for DeepSeekProvider

- [x] Define internal API types: `ApiRequest`, `ApiMessage`, `ApiResponse`, `Choice`, `ChoiceMessage`, `ApiError`, `ErrorDetail`
- [x] Implement `LlmProvider::complete()` method with:
  - Request building with OpenAI-compatible format
  - `Authorization: Bearer <KEY>` header
  - POST to `https://api.deepseek.com/chat/completions`
  - Response parsing from `choices[0].message.content`
  - Error mapping (401 -> AuthenticationError, 4xx/5xx -> appropriate errors)

**Acceptance Criteria:**
- `test_api_request_serialization` passes: request JSON matches OpenAI spec
- `test_api_response_parsing` passes: content extracted from `choices[0].message.content`
- System messages included in `messages` array (not separate field)

---

### 4. Create Provider Factory Module

- [x] Create `synapse-core/src/provider/factory.rs` with `create_provider()` function
- [x] Implement `get_api_key()` helper with env var priority over config
- [x] Add `mod factory;` and `pub use factory::create_provider;` to `synapse-core/src/provider.rs`
- [x] Export `create_provider` from `synapse-core/src/lib.rs`

**Acceptance Criteria:**
- `test_create_provider_deepseek` passes: factory returns `DeepSeekProvider` for "deepseek"
- `test_create_provider_anthropic` passes: factory returns `AnthropicProvider` for "anthropic"
- `test_create_provider_unknown` passes: factory returns `UnknownProvider` error for invalid name

---

### 5. Implement API Key Resolution Logic

- [x] Implement `get_api_key()` to check `DEEPSEEK_API_KEY` for "deepseek" provider
- [x] Implement `get_api_key()` to check `ANTHROPIC_API_KEY` for "anthropic" provider
- [x] Fall back to `config.api_key` when env var is not set
- [x] Return `MissingApiKey` error with helpful message when neither is available

**Acceptance Criteria:**
- `test_get_api_key_from_env` passes: env var value is used when set
- `test_get_api_key_from_config` passes: config api_key used when env var absent
- `test_env_var_takes_precedence` passes: env var overrides config api_key
- `test_get_api_key_missing` passes: returns `MissingApiKey` error with helpful message

---

### 6. Update CLI to Use Provider Factory

- [x] Replace hardcoded `AnthropicProvider` with `create_provider(&config)` call in `synapse-cli/src/main.rs`
- [x] Add appropriate error context for factory failures

**Acceptance Criteria:**
- `DEEPSEEK_API_KEY=test cargo run -p synapse-cli -- "Hello"` attempts DeepSeek API (default provider)
- `cargo run -p synapse-cli -- "Hello"` (no API key) shows error: "missing API key: Set DEEPSEEK_API_KEY..."
- Anthropic provider still works when `provider = "anthropic"` is set in config

---

### 7. Add Unit Tests for DeepSeekProvider

- [x] `test_deepseek_provider_new` - constructor creates provider with correct fields
- [x] `test_api_request_serialization` - request JSON format matches OpenAI spec
- [x] `test_api_request_with_system_message` - system messages in messages array
- [x] `test_api_response_parsing` - response content extraction
- [x] `test_api_error_parsing` - error response parsing

**Acceptance Criteria:**
- `cargo test -p synapse-core deepseek` passes all tests
- Tests cover request/response serialization without making actual API calls

---

### 8. Add Unit Tests for Provider Factory

- [x] `test_create_provider_deepseek` - factory creates DeepSeekProvider
- [x] `test_create_provider_anthropic` - factory creates AnthropicProvider
- [x] `test_create_provider_unknown` - factory returns UnknownProvider error
- [x] `test_get_api_key_from_env` - env var is used
- [x] `test_get_api_key_from_config` - config key is used
- [x] `test_get_api_key_missing` - returns MissingApiKey error
- [x] `test_env_var_takes_precedence` - env var overrides config

**Acceptance Criteria:**
- `cargo test -p synapse-core factory` passes all tests
- Tests verify both happy path and error cases

---

### 9. Manual Integration Testing

- [ ] Test DeepSeek provider: `DEEPSEEK_API_KEY=sk-... cargo run -p synapse-cli -- "Hello, DeepSeek"`
- [ ] Test Anthropic provider: set `provider = "anthropic"` in config, run with `ANTHROPIC_API_KEY`
- [x] Test missing API key error message
- [x] Test unknown provider error message
- [ ] Test piped input: `echo "What is Rust?" | DEEPSEEK_API_KEY=sk-... cargo run -p synapse-cli`

**Acceptance Criteria:**
- DeepSeek API returns valid response with valid API key
- Anthropic API still works as before
- Error messages are clear and actionable

---

### 10. Final Verification

- [x] `cargo fmt --check` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo test` passes all tests
- [x] `cargo build --release` succeeds

**Acceptance Criteria:**
- All CI checks pass
- No warnings in release build
