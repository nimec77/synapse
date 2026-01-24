# Research: SY-6 - Phase 5: Anthropic Provider

## Resolved Questions

User confirmed: **Use defaults** - proceed with documented requirements only.

- **HTTP Client**: Use `reqwest` with `json` feature (as specified in PRD)
- **Timeouts**: Use reqwest defaults initially (no custom timeout configuration)
- **API Version**: Pin to `2023-06-01` as documented
- **Max Tokens**: Use 1024 as default (per PRD example)

---

## Related Modules and Services

### synapse-core Structure

| File | Purpose | Relevance to SY-6 |
|------|---------|-------------------|
| `synapse-core/src/lib.rs` | Module exports | Must export `AnthropicProvider` |
| `synapse-core/src/config.rs` | Configuration loading | Provides `api_key`, `model`, `provider` fields |
| `synapse-core/src/message.rs` | `Role` and `Message` types | Used for request/response conversion |
| `synapse-core/src/provider.rs` | `LlmProvider` trait, `ProviderError` | Trait to implement, error type to extend |
| `synapse-core/src/provider/mock.rs` | `MockProvider` reference implementation | Pattern to follow |

### synapse-cli Structure

| File | Purpose | Relevance to SY-6 |
|------|---------|-------------------|
| `synapse-cli/src/main.rs` | CLI entry point | Must wire provider, add tokio runtime |

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
}
```

**Extension needed**: Add `AuthenticationError` variant for 401 responses.

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

**Note**: Role enum does not derive `Serialize`/`Deserialize`. Will need to map to Anthropic API role strings manually.

### Config Struct

Located in `synapse-core/src/config.rs`:

```rust
pub struct Config {
    pub provider: String,      // default: "deepseek"
    pub api_key: Option<String>,
    pub model: String,         // default: "deepseek-chat"
}
```

**Issue**: Default model is `deepseek-chat`, not Anthropic. When using Anthropic provider, may need to use a sensible default like `claude-3-5-sonnet-20241022`.

---

## Anthropic Messages API Requirements

### Endpoint
```
POST https://api.anthropic.com/v1/messages
```

### Required Headers
```
x-api-key: <API_KEY>
anthropic-version: 2023-06-01
content-type: application/json
```

### Request Body Format
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 1024,
  "messages": [
    {"role": "user", "content": "Hello, Claude"}
  ]
}
```

### Response Format
```json
{
  "id": "msg_...",
  "type": "message",
  "role": "assistant",
  "content": [
    {"type": "text", "text": "Hello! How can I help you today?"}
  ],
  "model": "claude-3-5-sonnet-20241022",
  "stop_reason": "end_turn",
  "usage": {"input_tokens": 10, "output_tokens": 15}
}
```

### Error Response Format
```json
{
  "type": "error",
  "error": {
    "type": "authentication_error",
    "message": "invalid x-api-key"
  }
}
```

### HTTP Status Codes to Handle
| Status | Meaning | Map to |
|--------|---------|--------|
| 200 | Success | Parse response |
| 400 | Bad request | `ProviderError::ProviderError` |
| 401 | Unauthorized | `ProviderError::AuthenticationError` (new) |
| 429 | Rate limited | `ProviderError::RequestFailed` |
| 500+ | Server error | `ProviderError::RequestFailed` |

---

## Patterns Used

### MockProvider Pattern

The `MockProvider` in `synapse-core/src/provider/mock.rs` demonstrates:

1. **Struct with internal state**:
   ```rust
   pub struct MockProvider {
       responses: Mutex<Vec<Message>>,
   }
   ```

2. **Builder pattern for configuration**:
   ```rust
   pub fn new() -> Self { ... }
   pub fn with_response(self, content: impl Into<String>) -> Self { ... }
   ```

3. **Async trait implementation**:
   ```rust
   #[async_trait]
   impl LlmProvider for MockProvider {
       async fn complete(&self, _messages: &[Message]) -> Result<Message, ProviderError> { ... }
   }
   ```

### Module System Pattern

Following Rust 2018+ style (no `mod.rs`):
```
synapse-core/src/
    provider.rs         # declares: mod mock; (will add: mod anthropic;)
    provider/
        mock.rs
        anthropic.rs    # NEW FILE
```

### Error Handling Pattern

Uses `thiserror` for typed errors in library code:
```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("...")]
    VariantName { ... },
}
```

---

## Implementation Approach

### 1. AnthropicProvider Struct

```rust
// synapse-core/src/provider/anthropic.rs
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}
```

### 2. Request/Response Types (Internal)

```rust
#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    // ... other fields ignored
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}
```

### 3. Error Extension

Add to `ProviderError`:
```rust
#[error("authentication failed: {0}")]
AuthenticationError(String),
```

### 4. CLI Integration

Update `synapse-cli/src/main.rs`:
- Add `#[tokio::main]` async runtime
- Check for API key presence
- Create `AnthropicProvider`
- Call `provider.complete(&messages).await`
- Print response or error

---

## Dependencies to Add

### synapse-core/Cargo.toml
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1"
```

Note: `serde` is already present with `derive` feature.

### synapse-cli/Cargo.toml
```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
anyhow = "1"
```

Note: `anyhow` is specified in `docs/vision.md` for CLI error handling.

---

## Limitations and Risks

### Limitations

1. **No Streaming**: This phase implements blocking (non-streaming) completion only. The CLI waits for the full response before displaying.

2. **Single Provider**: No provider selection logic. Hardcoded to use Anthropic regardless of `config.provider` value.

3. **Text Only**: Only text content supported. Tool calls, images, and other content types are out of scope.

4. **No Retry Logic**: First implementation has no automatic retry on transient failures.

5. **Default Timeouts**: Uses reqwest defaults, which may be insufficient for slow responses.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Anthropic API format changes | Low | Medium | Pin `anthropic-version` header |
| Rate limiting | Medium | Low | Return clear error message |
| Network timeouts | Low | Medium | Use sensible reqwest defaults |
| Response parsing failures | Low | High | Defensive deserialization, log unexpected |

---

## New Technical Questions

These questions emerged during research and may need follow-up:

1. **Model Default**: Should `AnthropicProvider` have its own default model (`claude-3-5-sonnet-20241022`) when config model is `deepseek-chat`?
   - **Suggested**: Yes, check if model matches Anthropic naming pattern, else use provider default.

2. **Reuse reqwest::Client**: Should the client be stored in the provider struct or created per-request?
   - **Suggested**: Store in struct for connection pooling (as shown in approach).

3. **System Message Handling**: Anthropic API has a separate `system` parameter, not in `messages` array. Current `Message` type supports `Role::System`.
   - **Suggested**: For this phase, skip system messages or extract to `system` parameter.

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/src/provider/anthropic.rs` | Create | `AnthropicProvider` implementation |
| `synapse-core/src/provider.rs` | Modify | Add `mod anthropic;`, export `AnthropicProvider`, extend `ProviderError` |
| `synapse-core/src/lib.rs` | Modify | Export `AnthropicProvider` |
| `synapse-core/Cargo.toml` | Modify | Add `reqwest`, `serde_json` |
| `synapse-cli/src/main.rs` | Modify | Add tokio runtime, wire provider |
| `synapse-cli/Cargo.toml` | Modify | Add `tokio`, `anyhow` |

---

## References

- `docs/prd/SY-6.prd.md` - PRD document
- `docs/phase/phase-5.md` - Phase task breakdown
- `docs/vision.md` - Architecture patterns
- `docs/conventions.md` - Code conventions
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
