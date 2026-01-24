# Implementation Plan: SY-6 - Phase 5: Anthropic Provider

Status: PLAN_APPROVED

## Overview

This plan details the implementation of the Anthropic Claude provider for Synapse. It transforms the CLI from an echo tool into a working AI assistant that communicates with Claude via the Anthropic Messages API.

---

## Components

### 1. AnthropicProvider (`synapse-core/src/provider/anthropic.rs`)

New file implementing the `LlmProvider` trait for Anthropic's Messages API.

**Struct definition:**
```rust
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}
```

**Public API:**
```rust
impl AnthropicProvider {
    /// Create a new Anthropic provider.
    ///
    /// # Arguments
    /// * `api_key` - Anthropic API key
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self;
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
```

**Internal types (private to module):**
```rust
#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}
```

### 2. ProviderError Extension (`synapse-core/src/provider.rs`)

Add new error variant for authentication failures:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider error: {message}")]
    ProviderError { message: String },

    #[error("request failed: {0}")]
    RequestFailed(String),

    // NEW
    #[error("authentication failed: {0}")]
    AuthenticationError(String),
}
```

### 3. Module Exports (`synapse-core/src/provider.rs`)

Update to include anthropic module:

```rust
mod anthropic;
mod mock;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;
```

### 4. Library Exports (`synapse-core/src/lib.rs`)

Update to export AnthropicProvider:

```rust
pub use provider::{AnthropicProvider, LlmProvider, MockProvider, ProviderError};
```

### 5. CLI Integration (`synapse-cli/src/main.rs`)

Transform from synchronous echo to async provider-based completion:

```rust
use anyhow::{Context, Result, bail};
use synapse_core::{AnthropicProvider, Config, LlmProvider, Message, Role};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    let message = get_message(&args)?;

    // Validate API key
    let api_key = config.api_key
        .context("API key not configured. Add api_key to config.toml")?;

    // Create provider
    let provider = AnthropicProvider::new(api_key, &config.model);

    // Send request
    let messages = vec![Message::new(Role::User, message)];
    let response = provider.complete(&messages).await?;

    println!("{}", response.content);
    Ok(())
}
```

---

## API Contract

### Anthropic Messages API

**Endpoint:** `POST https://api.anthropic.com/v1/messages`

**Required Headers:**
| Header | Value |
|--------|-------|
| `x-api-key` | API key from config |
| `anthropic-version` | `2023-06-01` (pinned) |
| `content-type` | `application/json` |

**Request Body:**
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 1024,
  "messages": [
    {"role": "user", "content": "Hello, Claude"}
  ],
  "system": "Optional system prompt"
}
```

**Success Response (200):**
```json
{
  "content": [
    {"type": "text", "text": "Response text here"}
  ]
}
```

**Error Response:**
```json
{
  "type": "error",
  "error": {
    "type": "authentication_error",
    "message": "invalid x-api-key"
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

---

## Data Flows

### Successful Request Flow

```
1. User: synapse "What is Rust?"
         |
2. CLI: Args::parse()
         |
3. CLI: Config::load() -> Config { api_key, model, ... }
         |
4. CLI: Validate api_key.is_some()
         |
5. CLI: AnthropicProvider::new(api_key, model)
         |
6. CLI: provider.complete(&messages)
         |
7. AnthropicProvider: Build ApiRequest
         |
8. AnthropicProvider: POST to api.anthropic.com
         |
9. AnthropicProvider: Parse ApiResponse
         |
10. AnthropicProvider: Return Message { role: Assistant, content }
         |
11. CLI: println!("{}", response.content)
```

### Error Flow (Missing API Key)

```
1. User: synapse "Hello"
         |
2. CLI: Config::load() -> Config { api_key: None, ... }
         |
3. CLI: api_key.context("API key not configured...")
         |
4. CLI: bail!() with error message
         |
5. User sees: "Error: API key not configured. Add api_key to config.toml"
         |
6. Exit code 1
```

### Error Flow (API Error)

```
1-6. (Same as successful flow through HTTP request)
         |
7. Anthropic API: Returns 401 Unauthorized
         |
8. AnthropicProvider: Maps to ProviderError::AuthenticationError
         |
9. CLI: Receives Err(ProviderError)
         |
10. CLI: anyhow formats and prints error
         |
11. User sees: "Error: authentication failed: invalid x-api-key"
         |
12. Exit code 1
```

---

## Dependencies

### synapse-core/Cargo.toml

```toml
[dependencies]
async-trait = "0.1"
dirs = "6.0.0"
reqwest = { version = "0.12", features = ["json"] }  # NEW
serde = { version = "1", features = ["derive"] }
serde_json = "1"                                      # NEW
thiserror = "2"
tokio = { version = "1", features = ["rt", "macros"] }
toml = "0.9.8"
```

### synapse-cli/Cargo.toml

```toml
[dependencies]
anyhow = "1"                                                        # NEW
clap = { version = "4.5.54", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] } # NEW
```

---

## Non-Functional Requirements

### Performance

| Requirement | Target | Notes |
|-------------|--------|-------|
| Response latency | < 30s | Depends on Anthropic API; use default reqwest timeout |
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
| HTTPS only | Hardcoded `https://api.anthropic.com` |
| No key in CLI args | Read from config file only |

---

## File Changes Summary

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/Cargo.toml` | Modify | Add `reqwest`, `serde_json` |
| `synapse-core/src/provider.rs` | Modify | Add `mod anthropic`, export `AnthropicProvider`, add `AuthenticationError` |
| `synapse-core/src/provider/anthropic.rs` | Create | `AnthropicProvider` implementation |
| `synapse-core/src/lib.rs` | Modify | Export `AnthropicProvider` |
| `synapse-cli/Cargo.toml` | Modify | Add `tokio`, `anyhow` |
| `synapse-cli/src/main.rs` | Modify | Async runtime, provider integration |

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API version incompatibility | Low | High | Pin `anthropic-version: 2023-06-01` header |
| Rate limiting during development | Medium | Low | Clear error message, document in user guide |
| Response parsing failures | Low | High | Defensive deserialization, log unexpected responses |
| Long response times | Medium | Low | Default timeouts, no special handling this phase |
| Model name mismatch | Medium | Low | Use config model as-is, let API validate |

---

## Design Decisions

### 1. System Message Handling

**Decision:** Extract `Role::System` messages to the `system` parameter in API request.

**Rationale:** Anthropic API expects system prompt as a separate field, not in messages array. We filter system messages from the messages array and concatenate them for the `system` field.

### 2. Model Default

**Decision:** Use model from config as-is, do not override.

**Rationale:** The config file should specify the correct model. If user sets `provider = "anthropic"` but forgets to change model, the API will return an error with a clear message. This is fail-fast behavior.

### 3. Max Tokens

**Decision:** Hardcode `max_tokens: 1024` for this phase.

**Rationale:** PRD specifies 1024 as default. Future phases can add config option.

### 4. Client Reuse

**Decision:** Store `reqwest::Client` in provider struct.

**Rationale:** Enables connection pooling. Client is cheap to clone (uses Arc internally).

### 5. No Retry Logic

**Decision:** No automatic retry for this phase.

**Rationale:** PRD explicitly notes this as out of scope. Future phase can add retry with exponential backoff.

---

## Testing Strategy

### Unit Tests (in `synapse-core/src/provider/anthropic.rs`)

1. `test_api_request_serialization` - Verify request JSON format
2. `test_api_response_parsing` - Verify response parsing
3. `test_system_message_extraction` - Verify system messages go to `system` field
4. `test_role_to_string_mapping` - Verify Role enum maps to API strings

### Integration Tests (manual)

1. Successful completion with valid API key
2. Error with invalid API key (401)
3. Error with missing API key (CLI validation)
4. Piped input: `echo "Hello" | synapse`

### Mock Tests

Existing `MockProvider` tests remain unchanged. No mocking of HTTP for unit tests in this phase.

---

## Open Questions

None. All decisions are resolved based on:
- PRD requirements
- Research document findings
- Conventions document rules

---

## References

- `docs/prd/SY-6.prd.md` - Requirements
- `docs/research/SY-6.md` - Technical research
- `docs/conventions.md` - Code standards
- `synapse-core/src/provider/mock.rs` - Pattern reference
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
