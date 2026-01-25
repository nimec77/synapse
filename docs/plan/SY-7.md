# Implementation Plan: SY-7 - Phase 6: DeepSeek Provider

Status: PLAN_APPROVED

## Overview

This plan details the implementation of the DeepSeek LLM provider as the default provider for Synapse, along with a provider factory pattern that enables dynamic provider selection based on configuration. DeepSeek uses an OpenAI-compatible API, making integration straightforward while establishing the pattern for future OpenAI and similar providers.

---

## Components

### 1. DeepSeekProvider (`synapse-core/src/provider/deepseek.rs`)

New file implementing the `LlmProvider` trait for DeepSeek's OpenAI-compatible Chat Completions API.

**Struct definition:**
```rust
pub struct DeepSeekProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}
```

**Public API:**
```rust
impl DeepSeekProvider {
    /// Create a new DeepSeek provider.
    ///
    /// # Arguments
    /// * `api_key` - DeepSeek API key
    /// * `model` - Model identifier (e.g., "deepseek-chat")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self;
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

**Internal types (private to module):**
```rust
/// API endpoint constant.
const API_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

/// Default max tokens for API responses.
const DEFAULT_MAX_TOKENS: u32 = 1024;

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
}
```

### 2. Provider Factory (`synapse-core/src/provider/factory.rs`)

New file providing dynamic provider creation based on configuration.

**Public API:**
```rust
use crate::config::Config;
use crate::provider::{AnthropicProvider, DeepSeekProvider, LlmProvider, ProviderError};

/// Create an LLM provider based on configuration.
///
/// Selects the appropriate provider based on `config.provider` and retrieves
/// the API key from environment variable or config file.
///
/// # Environment Variables
/// - `DEEPSEEK_API_KEY` for "deepseek" provider
/// - `ANTHROPIC_API_KEY` for "anthropic" provider
///
/// # Errors
/// - `ProviderError::MissingApiKey` if no API key is found
/// - `ProviderError::UnknownProvider` if provider name is not recognized
pub fn create_provider(config: &Config) -> Result<Box<dyn LlmProvider>, ProviderError>;
```

**Internal function:**
```rust
/// Retrieve API key from environment variable or config file.
///
/// Priority: environment variable > config.api_key
fn get_api_key(config: &Config) -> Result<String, ProviderError>;
```

### 3. ProviderError Extension (`synapse-core/src/provider.rs`)

Add two new error variants for factory error handling:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The provider returned an error response.
    #[error("provider error: {message}")]
    ProviderError { message: String },

    /// Request failed due to network or connection issues.
    #[error("request failed: {0}")]
    RequestFailed(String),

    /// Authentication failed (e.g., invalid API key).
    #[error("authentication failed: {0}")]
    AuthenticationError(String),

    // NEW
    /// API key not configured.
    #[error("missing API key: {0}")]
    MissingApiKey(String),

    // NEW
    /// Unknown provider name in configuration.
    #[error("unknown provider: {0}")]
    UnknownProvider(String),
}
```

### 4. Module Exports (`synapse-core/src/provider.rs`)

Update to include new modules:

```rust
mod anthropic;
mod deepseek;  // NEW
mod factory;   // NEW
mod mock;

pub use anthropic::AnthropicProvider;
pub use deepseek::DeepSeekProvider;  // NEW
pub use factory::create_provider;     // NEW
pub use mock::MockProvider;
```

### 5. Library Exports (`synapse-core/src/lib.rs`)

Update to export new types:

```rust
pub use provider::{
    create_provider,      // NEW
    AnthropicProvider,
    DeepSeekProvider,     // NEW
    LlmProvider,
    MockProvider,
    ProviderError,
};
```

### 6. CLI Integration (`synapse-cli/src/main.rs`)

Replace hardcoded AnthropicProvider with factory:

```rust
use synapse_core::{create_provider, Config, Message, Role};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    let message = match get_message(&args) {
        Ok(msg) => msg,
        Err(_) => {
            Args::parse_from(["synapse", "--help"]);
            return Ok(());
        }
    };

    // Create provider using factory (handles API key lookup)
    let provider = create_provider(&config)
        .context("Failed to create LLM provider")?;

    // Send request
    let messages = vec![Message::new(Role::User, message)];
    let response = provider
        .complete(&messages)
        .await
        .context("Failed to get response from LLM")?;

    println!("{}", response.content);
    Ok(())
}
```

---

## API Contract

### DeepSeek Chat Completions API (OpenAI-compatible)

**Endpoint:** `POST https://api.deepseek.com/chat/completions`

**Required Headers:**
| Header | Value |
|--------|-------|
| `Authorization` | `Bearer <API_KEY>` |
| `Content-Type` | `application/json` |

**Request Body:**
```json
{
  "model": "deepseek-chat",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello, DeepSeek"}
  ],
  "max_tokens": 1024
}
```

**Success Response (200):**
```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "deepseek-chat",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 15,
    "total_tokens": 25
  }
}
```

**Error Response:**
```json
{
  "error": {
    "message": "Incorrect API key provided",
    "type": "invalid_request_error",
    "code": "invalid_api_key"
  }
}
```

### Error Mapping

| HTTP Status | Error Type | Maps To |
|-------------|------------|---------|
| 200 | Success | Parse response, return `Message` |
| 400 | Bad Request | `ProviderError::ProviderError` |
| 401 | Unauthorized | `ProviderError::AuthenticationError` |
| 429 | Rate Limited | `ProviderError::RequestFailed` |
| 500+ | Server Error | `ProviderError::RequestFailed` |
| Network Error | Connection Failed | `ProviderError::RequestFailed` |

### Provider Factory Contract

| Provider Name | Environment Variable | Default Model |
|---------------|---------------------|---------------|
| `deepseek` | `DEEPSEEK_API_KEY` | `deepseek-chat` |
| `anthropic` | `ANTHROPIC_API_KEY` | `claude-3-5-sonnet-20241022` |

---

## Data Flows

### Successful DeepSeek Request Flow (Default)

```
1. User: DEEPSEEK_API_KEY=sk-... synapse "What is Rust?"
         |
2. CLI: Args::parse()
         |
3. CLI: Config::load() -> Config { provider: "deepseek", model: "deepseek-chat", ... }
         |
4. CLI: create_provider(&config)
         |
5. Factory: get_api_key(&config)
         |
6. Factory: std::env::var("DEEPSEEK_API_KEY") -> Ok("sk-...")
         |
7. Factory: DeepSeekProvider::new(api_key, model)
         |
8. CLI: provider.complete(&messages)
         |
9. DeepSeekProvider: Build ApiRequest with messages
         |
10. DeepSeekProvider: POST to api.deepseek.com/chat/completions
          |
11. DeepSeekProvider: Parse ApiResponse.choices[0].message.content
          |
12. DeepSeekProvider: Return Message { role: Assistant, content }
          |
13. CLI: println!("{}", response.content)
```

### Provider Selection Flow (Anthropic)

```
1. User: config.toml has provider = "anthropic"
         |
2. User: ANTHROPIC_API_KEY=sk-ant-... synapse "Hello"
         |
3. CLI: Config::load() -> Config { provider: "anthropic", ... }
         |
4. CLI: create_provider(&config)
         |
5. Factory: match "anthropic" -> get ANTHROPIC_API_KEY
         |
6. Factory: AnthropicProvider::new(api_key, model)
         |
7. CLI: provider.complete(&messages) -> (uses Anthropic API)
```

### Error Flow (Missing API Key)

```
1. User: synapse "Hello" (no env var, no config api_key)
         |
2. CLI: Config::load() -> Config { provider: "deepseek", api_key: None, ... }
         |
3. CLI: create_provider(&config)
         |
4. Factory: get_api_key(&config)
         |
5. Factory: std::env::var("DEEPSEEK_API_KEY") -> Err
         |
6. Factory: config.api_key.is_none() -> true
         |
7. Factory: Return ProviderError::MissingApiKey(
              "Set DEEPSEEK_API_KEY environment variable or add api_key to config.toml"
           )
         |
8. CLI: context("Failed to create LLM provider")
         |
9. User sees: "Error: Failed to create LLM provider: missing API key: Set DEEPSEEK_API_KEY..."
         |
10. Exit code 1
```

### Error Flow (Unknown Provider)

```
1. User: config.toml has provider = "invalid"
         |
2. CLI: create_provider(&config)
         |
3. Factory: match "invalid" -> no match
         |
4. Factory: Return ProviderError::UnknownProvider("invalid")
         |
5. User sees: "Error: Failed to create LLM provider: unknown provider: invalid"
         |
6. Exit code 1
```

---

## Non-Functional Requirements

### Performance

| Requirement | Target | Notes |
|-------------|--------|-------|
| Response latency | < 30s | Depends on DeepSeek API; use default reqwest timeout |
| Memory usage | < 50MB | Typical for HTTP client + response buffer |
| Startup time | < 100ms | Config loading + client creation |

### Reliability

| Requirement | Implementation |
|-------------|----------------|
| Connection pooling | Reuse `reqwest::Client` instance |
| Timeout handling | Use reqwest defaults (30s connect, no read timeout) |
| Error messages | Human-readable, actionable |

### Security

| Requirement | Implementation |
|-------------|----------------|
| API key protection | Never log API key |
| HTTPS only | Hardcoded `https://api.deepseek.com` |
| Env var priority | Environment variable takes precedence over config file |

### Compatibility

| Requirement | Implementation |
|-------------|----------------|
| Backward compatibility | Existing Anthropic provider continues to work |
| Default experience | DeepSeek works out-of-box with just API key |

---

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/src/provider/deepseek.rs` | Create | `DeepSeekProvider` implementation |
| `synapse-core/src/provider/factory.rs` | Create | `create_provider()` factory function |
| `synapse-core/src/provider.rs` | Modify | Add `mod deepseek; mod factory;`, export types, add error variants |
| `synapse-core/src/lib.rs` | Modify | Export `DeepSeekProvider` and `create_provider` |
| `synapse-cli/src/main.rs` | Modify | Use factory instead of hardcoded provider |

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| DeepSeek API differs from OpenAI spec | Low | Medium | DeepSeek documents OpenAI compatibility; defensive parsing |
| Rate limiting during testing | Medium | Low | Clear error message, use test API key sparingly |
| Response parsing failures | Low | High | Defensive deserialization with clear error messages |
| Environment variable conflicts | Low | Low | Clear precedence: env var > config file |
| Breaking Anthropic provider | Low | High | Keep AnthropicProvider unchanged; test both paths |

---

## Design Decisions

### 1. System Message Handling

**Decision:** Include system messages in the `messages` array with `role: "system"`.

**Rationale:** Unlike Anthropic API (which requires separate `system` field), OpenAI-compatible APIs accept system messages directly in the messages array. This simplifies implementation.

### 2. Provider Factory Location

**Decision:** Place factory in `synapse-core/src/provider/factory.rs`.

**Rationale:** Keeps factory close to provider implementations while maintaining clean module separation. Factory has access to all provider types through the parent module.

### 3. API Key Resolution

**Decision:** Environment variable takes precedence over config file.

**Rationale:** This follows the 12-factor app principle where env vars override config for deployment flexibility. Also prevents accidental exposure of keys in config files.

### 4. Error Variant Naming

**Decision:** Use `MissingApiKey(String)` and `UnknownProvider(String)` with descriptive messages.

**Rationale:** The string parameter allows for helpful error messages like "Set DEEPSEEK_API_KEY environment variable or add api_key to config.toml" rather than generic errors.

### 5. No Model Validation

**Decision:** Pass model name to provider as-is without validation.

**Rationale:** API will validate model names and return clear error messages. Client-side validation would require maintaining a list of valid models.

### 6. Max Tokens

**Decision:** Hardcode `max_tokens: 1024` matching AnthropicProvider.

**Rationale:** Consistent with existing implementation. Future phases can add configuration option.

---

## Testing Strategy

### Unit Tests (in `synapse-core/src/provider/deepseek.rs`)

1. `test_deepseek_provider_new` - Constructor creates provider with correct fields
2. `test_api_request_serialization` - Verify request JSON format matches OpenAI spec
3. `test_api_request_with_system_message` - Verify system messages are included in messages array
4. `test_api_response_parsing` - Verify response content extraction from choices[0]
5. `test_api_error_parsing` - Verify error response parsing

### Unit Tests (in `synapse-core/src/provider/factory.rs`)

1. `test_create_provider_deepseek` - Factory creates DeepSeekProvider for "deepseek"
2. `test_create_provider_anthropic` - Factory creates AnthropicProvider for "anthropic"
3. `test_create_provider_unknown` - Factory returns UnknownProvider error
4. `test_get_api_key_from_env` - Environment variable is used when set
5. `test_get_api_key_from_config` - Config api_key is used when env var absent
6. `test_get_api_key_missing` - Returns MissingApiKey error when neither available
7. `test_env_var_takes_precedence` - Env var used even when config has api_key

### Integration Tests (manual)

```bash
# Test DeepSeek provider (default)
DEEPSEEK_API_KEY=sk-... cargo run -p synapse-cli -- "Hello, DeepSeek"

# Test Anthropic provider (requires config change)
# First set provider = "anthropic" in config.toml
ANTHROPIC_API_KEY=sk-ant-... cargo run -p synapse-cli -- "Hello, Claude"

# Test missing API key error
unset DEEPSEEK_API_KEY
cargo run -p synapse-cli -- "Hello"
# Expected: error message about missing API key

# Test piped input
echo "What is Rust?" | DEEPSEEK_API_KEY=sk-... cargo run -p synapse-cli
```

---

## Differences from AnthropicProvider

| Aspect | Anthropic API | DeepSeek (OpenAI-compatible) |
|--------|---------------|------------------------------|
| Endpoint | `/v1/messages` | `/chat/completions` |
| Auth Header | `x-api-key: <KEY>` | `Authorization: Bearer <KEY>` |
| Version Header | `anthropic-version: 2023-06-01` | Not required |
| System Messages | Separate `system` parameter | In `messages` array with `role: "system"` |
| Response Content | `content: [{type: "text", text: "..."}]` | `choices[0].message.content` |

---

## Open Questions

None. All decisions are resolved based on:
- PRD requirements
- Research document findings
- Existing AnthropicProvider pattern
- Conventions document rules

---

## References

- `docs/prd/SY-7.prd.md` - Requirements
- `docs/research/SY-7.md` - Technical research
- `docs/conventions.md` - Code standards
- `synapse-core/src/provider/anthropic.rs` - Pattern reference
- [DeepSeek API Documentation](https://api-docs.deepseek.com/)
