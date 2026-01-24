# Phase 5: Anthropic Provider

**Goal:** Real API calls to Claude.

## Tasks

- [ ] 5.1 Create `synapse-core/src/provider/anthropic.rs` with `AnthropicProvider` struct
- [ ] 5.2 Add `reqwest` (with `json` feature), implement Messages API request
- [ ] 5.3 Create `synapse-core/src/error.rs` with `ProviderError` enum
- [ ] 5.4 Wire provider into CLI: load config → create provider → call API
- [ ] 5.5 Add API key validation (fail fast if missing)

## Acceptance Criteria

**Test:** `synapse "Say hello"` returns real Claude response.

## Dependencies

- Phase 4 complete (Provider trait and Message types exist)
- Anthropic API key configured

## Implementation Notes

### 5.1 Create AnthropicProvider struct

Create `synapse-core/src/provider/anthropic.rs` with:
- `AnthropicProvider` struct holding API key and model
- Implement `LlmProvider` trait

### 5.2 Add reqwest and implement API request

Add to `synapse-core/Cargo.toml`:
```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
```

Implement Messages API call to `https://api.anthropic.com/v1/messages`

### 5.3 Create error types

Create `synapse-core/src/error.rs` with:
- Extended `ProviderError` variants for API errors
- HTTP status code handling

### 5.4 Wire into CLI

Update CLI to:
- Load config with API key
- Create `AnthropicProvider`
- Call API with user message

### 5.5 API key validation

Fail fast if API key is missing or invalid format.
