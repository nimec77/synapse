# Research: SY-7 - Phase 6: DeepSeek Provider

## Resolved Questions

User confirmed: **Use defaults** - proceed with documented requirements only.

- **HTTP Client**: Use `reqwest` with `json` feature (already available in synapse-core)
- **API Format**: OpenAI-compatible chat/completions endpoint
- **Base URL**: `https://api.deepseek.com`
- **Default Model**: `deepseek-chat` (already configured as default)
- **Max Tokens**: Use 1024 as default (consistent with AnthropicProvider)
- **Environment Variable**: `DEEPSEEK_API_KEY` takes precedence over config file

---

## Related Modules and Services

### synapse-core Structure

| File | Purpose | Relevance to SY-7 |
|------|---------|-------------------|
| `synapse-core/src/lib.rs` | Module exports | Must export `DeepSeekProvider` and factory |
| `synapse-core/src/config.rs` | Configuration loading | Provides `api_key`, `model`, `provider` fields; defaults to "deepseek" |
| `synapse-core/src/message.rs` | `Role` and `Message` types | Used for request/response conversion |
| `synapse-core/src/provider.rs` | `LlmProvider` trait, `ProviderError` | Trait to implement, error type to extend |
| `synapse-core/src/provider/anthropic.rs` | `AnthropicProvider` implementation | Pattern to follow for DeepSeekProvider |
| `synapse-core/src/provider/mock.rs` | `MockProvider` reference implementation | Secondary pattern reference |

### synapse-cli Structure

| File | Purpose | Relevance to SY-7 |
|------|---------|-------------------|
| `synapse-cli/src/main.rs` | CLI entry point | Must switch from hardcoded `AnthropicProvider` to factory |

---

## Current Endpoints and Contracts

### LlmProvider Trait

Located in `synapse-core/src/provider.rs`:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

Key observations:
- Uses `async_trait` crate for async trait methods
- Requires `Send + Sync` for thread safety in async contexts
- Takes `&[Message]` and returns `Result<Message, ProviderError>`

### ProviderError Enum

Currently defined in `synapse-core/src/provider.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider error: {message}")]
    ProviderError { message: String },

    #[error("request failed: {0}")]
    RequestFailed(String),

    #[error("authentication failed: {0}")]
    AuthenticationError(String),
}
```

**Extension needed**: Add `MissingApiKey` and `UnknownProvider` variants for factory error handling.

### Message and Role Types

Located in `synapse-core/src/message.rs`:

```rust
pub enum Role {
    System,
    User,
    Assistant,
}

pub struct Message {
    pub role: Role,
    pub content: String,
}
```

**Note**: Role enum maps to OpenAI-compatible API role strings: `"system"`, `"user"`, `"assistant"`.

### Config Struct

Located in `synapse-core/src/config.rs`:

```rust
pub struct Config {
    pub provider: String,       // default: "deepseek"
    pub api_key: Option<String>,
    pub model: String,          // default: "deepseek-chat"
}
```

**Good news**: Defaults are already set for DeepSeek, making it the out-of-box experience.

---

## DeepSeek API Requirements

### Endpoint
```
POST https://api.deepseek.com/chat/completions
```

### Required Headers
```
Authorization: Bearer <API_KEY>
Content-Type: application/json
```

### Request Body Format (OpenAI-compatible)
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

### Response Format (OpenAI-compatible)
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

### Error Response Format
```json
{
  "error": {
    "message": "Incorrect API key provided",
    "type": "invalid_request_error",
    "code": "invalid_api_key"
  }
}
```

### HTTP Status Codes to Handle
| Status | Meaning | Map to |
|--------|---------|--------|
| 200 | Success | Parse response |
| 400 | Bad request | `ProviderError::ProviderError` |
| 401 | Unauthorized | `ProviderError::AuthenticationError` |
| 429 | Rate limited | `ProviderError::RequestFailed` |
| 500+ | Server error | `ProviderError::RequestFailed` |

---

## Patterns Used

### AnthropicProvider Pattern (Primary Reference)

The `AnthropicProvider` in `synapse-core/src/provider/anthropic.rs` demonstrates:

1. **Struct with HTTP client and credentials**:
   ```rust
   pub struct AnthropicProvider {
       client: reqwest::Client,
       api_key: String,
       model: String,
   }
   ```

2. **Simple constructor**:
   ```rust
   pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
       Self {
           client: reqwest::Client::new(),
           api_key: api_key.into(),
           model: model.into(),
       }
   }
   ```

3. **Internal API types for serialization**:
   ```rust
   #[derive(Debug, Serialize)]
   struct ApiRequest { ... }

   #[derive(Debug, Deserialize)]
   struct ApiResponse { ... }

   #[derive(Debug, Deserialize)]
   struct ApiError { ... }
   ```

4. **System message handling**: Anthropic requires system messages as a separate `system` field. DeepSeek (OpenAI-compatible) accepts them in the `messages` array, which is simpler.

5. **Error mapping**: Maps HTTP status codes to `ProviderError` variants:
   - 401 -> `AuthenticationError`
   - Other errors -> `RequestFailed` with HTTP status and body
   - Parse errors -> `ProviderError::ProviderError`

### Module System Pattern

Following Rust 2018+ style (no `mod.rs`):
```
synapse-core/src/
    provider.rs         # declares: mod anthropic; mod mock; mod deepseek;
    provider/
        anthropic.rs
        mock.rs
        deepseek.rs     # NEW FILE
```

The factory will be in `synapse-core/src/provider/factory.rs`:
```
synapse-core/src/
    provider.rs         # declares: mod anthropic; mod mock; mod deepseek; mod factory;
    provider/
        anthropic.rs
        mock.rs
        deepseek.rs     # NEW FILE
        factory.rs      # NEW FILE
```

---

## Differences: Anthropic vs OpenAI-compatible API

| Aspect | Anthropic API | DeepSeek (OpenAI-compatible) |
|--------|---------------|------------------------------|
| Endpoint | `/v1/messages` | `/chat/completions` |
| Auth Header | `x-api-key: <KEY>` | `Authorization: Bearer <KEY>` |
| Version Header | `anthropic-version: 2023-06-01` | Not required |
| System Messages | Separate `system` parameter | In `messages` array with `role: "system"` |
| Response Content | `content: [{type: "text", text: "..."}]` | `choices[0].message.content` |

---

## Implementation Approach

### 1. DeepSeekProvider Struct

```rust
// synapse-core/src/provider/deepseek.rs
pub struct DeepSeekProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}
```

### 2. Request/Response Types (Internal)

```rust
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

### 3. ProviderError Extension

Add to `ProviderError`:
```rust
#[error("missing API key: {0}")]
MissingApiKey(String),

#[error("unknown provider: {0}")]
UnknownProvider(String),
```

### 4. Provider Factory

```rust
// synapse-core/src/provider/factory.rs
pub fn create_provider(config: &Config) -> Result<Box<dyn LlmProvider>, ProviderError> {
    let api_key = get_api_key(config)?;

    match config.provider.as_str() {
        "deepseek" => Ok(Box::new(DeepSeekProvider::new(api_key, &config.model))),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(api_key, &config.model))),
        unknown => Err(ProviderError::UnknownProvider(unknown.to_string())),
    }
}

fn get_api_key(config: &Config) -> Result<String, ProviderError> {
    let env_var = match config.provider.as_str() {
        "deepseek" => "DEEPSEEK_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => return config.api_key.clone().ok_or_else(||
            ProviderError::MissingApiKey("API key not configured".to_string())),
    };

    std::env::var(env_var)
        .ok()
        .or(config.api_key.clone())
        .ok_or_else(|| ProviderError::MissingApiKey(
            format!("Set {} environment variable or add api_key to config.toml", env_var)
        ))
}
```

### 5. CLI Integration

Update `synapse-cli/src/main.rs`:
- Replace direct `AnthropicProvider::new()` with `create_provider(&config)?`
- Remove hardcoded API key check (factory handles it)
- Import `create_provider` from `synapse_core`

---

## Dependencies

### synapse-core/Cargo.toml (Already Present)

```toml
[dependencies]
async-trait = "0.1"
dirs = "6.0.0"
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt", "macros"] }
toml = "0.9.8"
```

No new dependencies required. All needed crates are already in place.

### synapse-cli/Cargo.toml (Already Present)

```toml
[dependencies]
anyhow = "1"
clap = { version = "4.5.54", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

No changes needed.

---

## Limitations and Risks

### Limitations

1. **No Streaming**: This phase implements non-streaming completion only. The CLI waits for the full response before displaying.

2. **Text Only**: Only text content supported. Tool calls, images, and other content types are out of scope.

3. **Two Providers Only**: DeepSeek and Anthropic. OpenAI provider planned for later phase.

4. **No Retry Logic**: First implementation has no automatic retry on transient failures.

5. **Default Timeouts**: Uses reqwest defaults, which should be acceptable for typical requests.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| DeepSeek API format differs from OpenAI | Low | Medium | DeepSeek documents OpenAI compatibility |
| Rate limiting | Medium | Low | Return clear error message |
| Network timeouts | Low | Medium | Use sensible reqwest defaults |
| Response parsing failures | Low | High | Defensive deserialization |
| Factory error propagation | Low | Low | Use Result type with clear errors |

---

## New Technical Questions

These questions emerged during research and may need follow-up in future phases:

1. **Model Validation**: Should factory validate that model name matches provider (e.g., warn if using `claude-3-5-sonnet` with DeepSeek)?
   - **For SY-7**: No, trust user configuration.

2. **Custom Base URL**: Should DeepSeekProvider support configurable base URL for self-hosted instances?
   - **For SY-7**: No, hardcode to `https://api.deepseek.com`.

3. **Timeout Configuration**: Should providers support configurable request timeouts?
   - **For SY-7**: No, use reqwest defaults.

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/src/provider/deepseek.rs` | Create | `DeepSeekProvider` implementation |
| `synapse-core/src/provider/factory.rs` | Create | `create_provider()` factory function |
| `synapse-core/src/provider.rs` | Modify | Add `mod deepseek; mod factory;`, export types, extend `ProviderError` |
| `synapse-core/src/lib.rs` | Modify | Export `DeepSeekProvider` and `create_provider` |
| `synapse-cli/src/main.rs` | Modify | Use factory instead of hardcoded provider |

---

## Test Plan

### Unit Tests

1. **DeepSeekProvider**:
   - `test_api_request_serialization` - Verify request JSON format
   - `test_api_response_parsing` - Verify response parsing
   - `test_api_error_parsing` - Verify error response parsing
   - `test_deepseek_provider_new` - Constructor test

2. **Factory**:
   - `test_create_provider_deepseek` - Factory creates DeepSeekProvider
   - `test_create_provider_anthropic` - Factory creates AnthropicProvider
   - `test_create_provider_unknown` - Factory returns UnknownProvider error
   - `test_get_api_key_from_env` - Environment variable is used
   - `test_get_api_key_from_config` - Config api_key is used
   - `test_get_api_key_missing` - Returns MissingApiKey error

### Manual Integration Tests

```bash
# Test DeepSeek provider
DEEPSEEK_API_KEY=sk-... cargo run -p synapse-cli -- "Hello"

# Test Anthropic provider (ensure backward compatibility)
ANTHROPIC_API_KEY=sk-ant-... cargo run -p synapse-cli -- "Hello"
# (requires setting provider = "anthropic" in config.toml)
```

---

## References

- `docs/prd/SY-7.prd.md` - PRD document
- `docs/phase/phase-6.md` - Phase task breakdown
- `docs/vision.md` - Architecture patterns
- `docs/conventions.md` - Code conventions
- `synapse-core/src/provider/anthropic.rs` - Reference implementation
- [DeepSeek API Documentation](https://api-docs.deepseek.com/)
