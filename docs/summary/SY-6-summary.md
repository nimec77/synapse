# SY-6 Summary: Phase 5 - Anthropic Provider

**Status:** COMPLETE
**Date:** 2026-01-24

---

## Overview

SY-6 implements the Anthropic Claude provider for Synapse, transforming the CLI from an echo tool into a working AI assistant that communicates with Claude via the Anthropic Messages API. This is the first real LLM provider implementation, building on the provider abstraction layer established in SY-5.

---

## What Was Built

### New Components

1. **AnthropicProvider** (`synapse-core/src/provider/anthropic.rs`)
   - Implements the `LlmProvider` trait for Anthropic's Messages API
   - Struct with `reqwest::Client`, `api_key`, and `model` fields
   - Constructor accepts generic `impl Into<String>` for flexibility
   - Handles system message extraction to separate API field
   - Maps HTTP errors to appropriate `ProviderError` variants

2. **API Types** (private to anthropic module)
   - `ApiRequest`: Serializable request body with model, max_tokens, messages, and optional system prompt
   - `ApiMessage`: Role and content for API messages
   - `ApiResponse` and `ContentBlock`: Deserializable response structures
   - `ApiError` and `ErrorDetail`: Error response parsing

3. **Extended Error Handling**
   - Added `AuthenticationError(String)` variant to `ProviderError`
   - HTTP 401 responses map to `AuthenticationError`
   - Other HTTP errors map to `RequestFailed`
   - Parse failures map to `ProviderError`

### Modified Components

1. **synapse-core/Cargo.toml**
   - Added `reqwest` with `json` feature for HTTP requests
   - Added `serde_json` for JSON serialization

2. **synapse-cli/Cargo.toml**
   - Added `tokio` with `rt-multi-thread` and `macros` features
   - Added `anyhow` for application error handling

3. **synapse-cli/src/main.rs**
   - Converted to async with `#[tokio::main]`
   - Replaced echo logic with provider-based completion
   - Added API key validation with clear error message
   - Returns `anyhow::Result<()>` for proper error handling

4. **Module Exports**
   - `synapse-core/src/provider.rs`: Added `mod anthropic` and `pub use AnthropicProvider`
   - `synapse-core/src/lib.rs`: Exports `AnthropicProvider` in public API

---

## Key Decisions

### 1. System Message Handling
System role messages are extracted from the messages array and concatenated (with `\n\n` separator) into the `system` field of the API request. This matches Anthropic's API expectations where system prompts are separate from conversation messages.

### 2. Max Tokens Default
Hardcoded to 1024 tokens for this phase. Future phases can add this as a configuration option.

### 3. API Version Pinning
Uses `anthropic-version: 2023-06-01` header to ensure compatibility. This protects against future API changes.

### 4. No Retry Logic
Explicit decision to defer retry with exponential backoff to a future phase. Simple fail-fast behavior for now.

### 5. Client Reuse
The `reqwest::Client` is stored in the provider struct for connection pooling benefits across multiple requests.

---

## API Integration Details

### Request Format
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 1024,
  "messages": [{"role": "user", "content": "Hello"}],
  "system": "Optional system prompt"
}
```

### Required Headers
- `x-api-key`: API key from configuration
- `anthropic-version`: `2023-06-01`
- `content-type`: `application/json`

### Error Mapping
| HTTP Status | ProviderError Variant |
|-------------|----------------------|
| 401 | `AuthenticationError` |
| 4xx/5xx | `RequestFailed` |
| Network error | `RequestFailed` |
| Parse error | `ProviderError` |

---

## Testing

### Unit Tests (8 new tests)
- `test_api_request_serialization` - Basic request JSON structure
- `test_api_request_serialization_with_system` - System field inclusion
- `test_api_response_parsing` - Single content block parsing
- `test_api_response_parsing_multiple_blocks` - Multi-block response handling
- `test_system_message_extraction` - Single system message filtering
- `test_system_message_extraction_multiple` - Multiple system message concatenation
- `test_anthropic_provider_new` - Constructor verification
- `test_api_error_parsing` - Error response deserialization

### CLI Test
- `test_args_parse` - Argument parsing with and without message

---

## Usage

### Configuration
Add API key to `config.toml`:
```toml
api_key = "sk-ant-..."
model = "claude-3-5-sonnet-20241022"
```

### One-Shot Mode
```bash
synapse "What is Rust?"
```

### Piped Input
```bash
echo "Explain async/await" | synapse
```

### Error Handling
- Missing API key: "API key not configured. Add api_key to config.toml"
- Invalid API key: "authentication failed: invalid x-api-key"
- Network failure: "request failed: [connection error]"

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-core/Cargo.toml` | Added reqwest, serde_json |
| `synapse-core/src/provider.rs` | Added mod anthropic, AuthenticationError variant |
| `synapse-core/src/provider/anthropic.rs` | New file - AnthropicProvider implementation |
| `synapse-core/src/lib.rs` | Export AnthropicProvider |
| `synapse-cli/Cargo.toml` | Added tokio, anyhow |
| `synapse-cli/src/main.rs` | Async main with provider integration |

---

## Security Considerations

- API key is read from config file only, not CLI arguments
- HTTPS enforced via hardcoded endpoint URL
- API key is never logged or included in error messages
- Config file should be protected with appropriate permissions

---

## Future Work

This implementation enables:
- **Streaming responses** (SSE via eventsource-stream)
- **Multi-turn conversations** (session memory)
- **Additional providers** (OpenAI, DeepSeek via same trait)
- **Configurable max_tokens**
- **Retry with exponential backoff**
