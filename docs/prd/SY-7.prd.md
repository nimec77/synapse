# SY-7: Phase 6: DeepSeek Provider

Status: PRD_READY

## Context / Idea

Phase 6 of the Synapse project focuses on implementing the DeepSeek LLM provider as the default provider. DeepSeek uses an OpenAI-compatible API, which makes integration straightforward. This phase also introduces the provider factory pattern to enable dynamic provider selection based on configuration.

The goal is to have a working default provider that functions out of the box, along with a factory mechanism that selects the appropriate provider (DeepSeek or Anthropic) based on the `config.provider` setting.

### Background from Previous Phases

Phase 5 (SY-6) established:
- `AnthropicProvider` implementing `LlmProvider` trait in `synapse-core/src/provider/anthropic.rs`
- Working API calls to Claude via the Anthropic Messages API
- `ProviderError` enum with `ProviderError`, `RequestFailed`, and `AuthenticationError` variants
- Async CLI with tokio runtime

Phase 4 (SY-5) established:
- `LlmProvider` trait with async `complete` method
- `Message` and `Role` types for conversation handling
- `MockProvider` for testing

Phase 3 (SY-4) established:
- Configuration loading from TOML with priority: `SYNAPSE_CONFIG` env var > `./config.toml` > `~/.config/synapse/config.toml`
- Default provider set to "deepseek" and default model set to "deepseek-chat"

### Current State

- Default configuration specifies `provider = "deepseek"` and `model = "deepseek-chat"`
- Only `AnthropicProvider` is currently implemented
- No provider factory exists; provider is hardcoded in CLI
- No support for `DEEPSEEK_API_KEY` environment variable

### DeepSeek API Details

- **Base URL:** `https://api.deepseek.com`
- **Endpoint:** `POST /chat/completions`
- **Format:** OpenAI-compatible (same request/response structure as OpenAI Chat API)
- **Models:** `deepseek-chat`, `deepseek-reasoner`
- **Authentication:** Bearer token in Authorization header

## Goals

1. **Implement DeepSeek Provider**: Create `DeepSeekProvider` struct that implements `LlmProvider` trait and makes API calls to the DeepSeek chat/completions endpoint using the OpenAI-compatible format.

2. **Create Provider Factory**: Implement `synapse-core/src/provider/factory.rs` with a function that creates the appropriate provider based on configuration settings.

3. **Support Environment Variable**: Add support for `DEEPSEEK_API_KEY` environment variable as an alternative to config file API key.

4. **Update CLI Integration**: Modify the CLI to use the provider factory instead of hardcoded provider creation.

5. **Default Out-of-Box Experience**: Ensure users with DeepSeek API keys can run `synapse "Hello"` immediately without additional configuration beyond the API key.

## User Stories

### US-1: Use DeepSeek as Default Provider
**As a** developer using Synapse
**I want to** use DeepSeek as the default LLM provider
**So that** I can interact with a capable AI assistant out of the box

**Acceptance Criteria:**
- Running `synapse "Hello"` with a DeepSeek API key returns a response from DeepSeek
- Works with default configuration (no `provider` setting required)
- The response is printed to stdout

### US-2: Configure DeepSeek API Key via Environment Variable
**As a** user
**I want to** provide my DeepSeek API key via environment variable
**So that** I do not need to store secrets in config files

**Acceptance Criteria:**
- `DEEPSEEK_API_KEY` environment variable is recognized
- Environment variable takes precedence over config file `api_key`
- Clear error message if neither env var nor config key is set

### US-3: Switch Between Providers via Configuration
**As a** user
**I want to** select my preferred LLM provider in the config file
**So that** I can easily switch between DeepSeek and Anthropic

**Acceptance Criteria:**
- Setting `provider = "deepseek"` uses DeepSeek
- Setting `provider = "anthropic"` uses Anthropic
- Invalid provider name shows helpful error message
- Default provider is "deepseek" when not specified

### US-4: Handle DeepSeek API Errors Gracefully
**As a** user
**I want to** see meaningful error messages when DeepSeek API calls fail
**So that** I can understand and fix problems

**Acceptance Criteria:**
- Invalid API key shows authentication error
- Network failures show connection error
- Rate limits show appropriate message
- All errors are human-readable

## Main Scenarios

### Scenario 1: Successful DeepSeek API Call (Default)
1. User runs `synapse "What is Rust?"` with `DEEPSEEK_API_KEY` set
2. CLI loads configuration (default provider is "deepseek")
3. Provider factory creates `DeepSeekProvider`
4. Provider sends request to `https://api.deepseek.com/chat/completions`
5. Provider receives response and parses it
6. CLI prints DeepSeek's response to stdout

### Scenario 2: Switch to Anthropic Provider
1. User sets `provider = "anthropic"` in config.toml
2. User runs `synapse "Hello"` with `ANTHROPIC_API_KEY` set
3. CLI loads configuration
4. Provider factory creates `AnthropicProvider`
5. Request goes to Anthropic API
6. CLI prints Claude's response

### Scenario 3: Missing DeepSeek API Key
1. User runs `synapse "Hello"` without API key (no env var, no config)
2. CLI loads configuration (default provider: deepseek)
3. CLI detects API key is None
4. CLI prints error: "Error: DeepSeek API key not configured. Set DEEPSEEK_API_KEY environment variable or add api_key to config.toml"
5. CLI exits with code 1

### Scenario 4: Invalid Provider Name
1. User sets `provider = "invalid"` in config.toml
2. User runs `synapse "Hello"`
3. Provider factory encounters unknown provider
4. CLI prints error: "Error: Unknown provider 'invalid'. Supported: deepseek, anthropic"
5. CLI exits with code 1

### Scenario 5: DeepSeek API Key via Environment Variable
1. User runs `DEEPSEEK_API_KEY=sk-xxx synapse "Hello"`
2. CLI loads configuration
3. Environment variable overrides any config file api_key
4. Provider factory creates `DeepSeekProvider` with env var key
5. API call succeeds, response printed

## Success / Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Successful DeepSeek API call | Works with valid key | Manual test: `DEEPSEEK_API_KEY=xxx synapse "Hello"` returns response |
| Provider factory selection | Correct provider based on config | Unit tests for factory function |
| Environment variable support | DEEPSEEK_API_KEY works | Manual test with env var only |
| Error handling | All error types handled | Unit tests for each error variant |
| Backward compatibility | Anthropic still works | Test: `provider = "anthropic"` with ANTHROPIC_API_KEY |
| Test coverage | All unit tests pass | `cargo test` passes |

## Constraints and Assumptions

### Constraints

1. **No Streaming (This Phase)**: Non-streaming completion only. SSE streaming is planned for a future phase.

2. **Text Content Only**: Only text content is supported. Tool calls, images, and other content types are out of scope.

3. **Two Providers**: Only DeepSeek and Anthropic providers are implemented. OpenAI is planned for a later phase.

4. **Synchronous CLI Flow**: The CLI waits for the complete response before printing. No progress indicators.

### Assumptions

1. User has access to DeepSeek API (valid API key)
2. Network access to `api.deepseek.com` is available
3. DeepSeek API maintains OpenAI-compatible format
4. Phase 5 (Anthropic provider) is complete and working
5. Configuration loading works correctly (from SY-4)

### Technical Constraints

- Use `reqwest` with `json` feature for HTTP requests (already in use)
- Follow OpenAI Chat Completions API specification (DeepSeek-compatible)
- Required headers: `Authorization: Bearer <API_KEY>`, `Content-Type: application/json`
- API endpoint: `https://api.deepseek.com/chat/completions`

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| DeepSeek API format changes | Low | Medium | Follow OpenAI spec, add version handling if needed |
| Rate limiting | Medium | Low | Implement proper error handling, document limits |
| Provider factory complexity | Low | Low | Keep factory simple with match statement |
| Environment variable conflicts | Low | Low | Clear precedence: env var > config file |

## Open Questions

None. The scope is well-defined by:
- Phase 6 task breakdown in `docs/phase/phase-6.md`
- DeepSeek API documentation (OpenAI-compatible)
- Existing `AnthropicProvider` implementation pattern
- Current configuration system design

## Implementation Notes

### DeepSeek API Request Format (OpenAI-compatible)

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

### Required Headers

```
Authorization: Bearer <API_KEY>
Content-Type: application/json
```

### Response Format

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

### Provider Factory Pattern

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
    // Check provider-specific env var first
    let env_var = match config.provider.as_str() {
        "deepseek" => "DEEPSEEK_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => return config.api_key.clone().ok_or(ProviderError::MissingApiKey),
    };

    std::env::var(env_var)
        .ok()
        .or(config.api_key.clone())
        .ok_or(ProviderError::MissingApiKey)
}
```

### Task Breakdown (from Phase 6)

- [ ] 6.1 Create `synapse-core/src/provider/deepseek.rs` with `DeepSeekProvider`
- [ ] 6.2 Implement OpenAI-compatible chat/completions API request
- [ ] 6.3 Create `synapse-core/src/provider/factory.rs` with provider selection
- [ ] 6.4 Update CLI to use factory based on `config.provider`
- [ ] 6.5 Support `DEEPSEEK_API_KEY` environment variable

## References

- `docs/phase/phase-6.md` - Phase task breakdown
- `docs/vision.md` - Technical architecture
- `synapse-core/src/provider.rs` - LlmProvider trait and ProviderError
- `synapse-core/src/provider/anthropic.rs` - Reference implementation pattern
- `synapse-core/src/config.rs` - Configuration loading
- `docs/prd/SY-6.prd.md` - Anthropic provider PRD (reference)
- [DeepSeek API Documentation](https://api-docs.deepseek.com/)
