# SY-7 Summary: Phase 6 - DeepSeek Provider

**Status:** COMPLETE
**Date:** 2026-01-25

---

## Overview

SY-7 implements the DeepSeek LLM provider as the default provider for Synapse, along with a provider factory pattern that enables dynamic provider selection based on configuration. DeepSeek uses an OpenAI-compatible API format, which establishes the pattern for future OpenAI and similar providers.

The key change is that users can now run `synapse "Hello"` with a DeepSeek API key and receive responses from DeepSeek out of the box, without any additional configuration.

---

## What Was Built

### New Components

1. **DeepSeekProvider** (`synapse-core/src/provider/deepseek.rs`)
   - Implements the `LlmProvider` trait for DeepSeek's Chat Completions API
   - Struct with `reqwest::Client`, `api_key`, and `model` fields
   - Uses OpenAI-compatible request/response format
   - System messages included in `messages` array (not a separate field like Anthropic)
   - Authorization via `Bearer` token header (vs Anthropic's `x-api-key`)

2. **Provider Factory** (`synapse-core/src/provider/factory.rs`)
   - `create_provider(config)` function returns `Box<dyn LlmProvider>`
   - Selects provider based on `config.provider` setting ("deepseek" or "anthropic")
   - `get_api_key(config)` helper resolves API key with environment variable priority
   - Provides descriptive error messages for missing keys and unknown providers

3. **API Types** (private to deepseek module)
   - `ApiRequest`: model, messages array, max_tokens
   - `ApiMessage`: role and content
   - `ApiResponse`, `Choice`, `ChoiceMessage`: response parsing
   - `ApiError`, `ErrorDetail`: error response handling

4. **Extended Error Handling**
   - Added `MissingApiKey(String)` variant for missing API key scenarios
   - Added `UnknownProvider(String)` variant for invalid provider names
   - Both include helpful error messages guiding the user

### Modified Components

1. **synapse-core/src/provider.rs**
   - Added `mod deepseek;` and `mod factory;` declarations
   - Added `pub use deepseek::DeepSeekProvider;` and `pub use factory::create_provider;`
   - Extended `ProviderError` with two new variants

2. **synapse-core/src/lib.rs**
   - Added exports for `DeepSeekProvider` and `create_provider`

3. **synapse-cli/src/main.rs**
   - Replaced hardcoded `AnthropicProvider` with `create_provider(&config)`
   - Removed explicit API key check (factory handles it)
   - Now uses dynamic provider selection based on configuration

---

## Key Decisions

### 1. System Message Handling (OpenAI Format)
System messages are included directly in the `messages` array with `role: "system"`, unlike Anthropic which requires a separate `system` field. This follows the OpenAI Chat Completions API specification that DeepSeek is compatible with.

### 2. Environment Variable Priority
Environment variable takes precedence over config file API key. This follows the 12-factor app principle and prevents accidental exposure of keys in config files.

| Provider | Environment Variable |
|----------|---------------------|
| deepseek | `DEEPSEEK_API_KEY` |
| anthropic | `ANTHROPIC_API_KEY` |

### 3. Provider Name Case Sensitivity
Provider names are case-sensitive. `"deepseek"` works, but `"DeepSeek"` returns an unknown provider error. This is a deliberate design choice for simplicity.

### 4. Empty Environment Variable Handling
An empty environment variable (`DEEPSEEK_API_KEY=""`) is treated as absent, falling back to the config file. This prevents confusion when env vars are set to empty strings.

### 5. Max Tokens Consistency
Hardcoded to 1024 tokens, matching the Anthropic provider. Future phases can add configuration options.

---

## API Integration Details

### DeepSeek Request Format (OpenAI-compatible)
```json
{
  "model": "deepseek-chat",
  "max_tokens": 1024,
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello, DeepSeek"}
  ]
}
```

### Required Headers
- `Authorization`: `Bearer <API_KEY>`
- `Content-Type`: `application/json`

### Response Format
```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      }
    }
  ]
}
```

### Differences from Anthropic API

| Aspect | Anthropic | DeepSeek (OpenAI-compatible) |
|--------|-----------|------------------------------|
| Endpoint | `/v1/messages` | `/chat/completions` |
| Auth Header | `x-api-key: <KEY>` | `Authorization: Bearer <KEY>` |
| Version Header | `anthropic-version: 2023-06-01` | Not required |
| System Messages | Separate `system` parameter | In `messages` array |
| Response Content | `content[0].text` | `choices[0].message.content` |

### Error Mapping
| HTTP Status | ProviderError Variant |
|-------------|----------------------|
| 401 | `AuthenticationError` |
| 4xx/5xx | `RequestFailed` |
| Network error | `RequestFailed` |
| Parse error | `ProviderError` |
| Missing API key | `MissingApiKey` |
| Invalid provider | `UnknownProvider` |

---

## Testing

### Unit Tests for DeepSeekProvider (5 tests)
- `test_deepseek_provider_new` - Constructor creates provider with correct fields
- `test_api_request_serialization` - Request JSON format matches OpenAI spec
- `test_api_request_with_system_message` - System messages in messages array
- `test_api_response_parsing` - Content extracted from choices[0].message.content
- `test_api_error_parsing` - Error response parsing

### Unit Tests for Provider Factory (8 tests)
- `test_create_provider_deepseek` - Factory creates DeepSeekProvider for "deepseek"
- `test_create_provider_anthropic` - Factory creates AnthropicProvider for "anthropic"
- `test_create_provider_unknown` - Factory returns UnknownProvider error
- `test_get_api_key_from_env` - Environment variable is used when set
- `test_get_api_key_from_config` - Config api_key used when env var absent
- `test_env_var_takes_precedence` - Env var overrides config api_key
- `test_get_api_key_missing` - Returns MissingApiKey error for DeepSeek
- `test_get_api_key_missing_anthropic` - Returns MissingApiKey error for Anthropic

**Total new tests: 13**

---

## Usage

### Default Provider (DeepSeek)
```bash
# One-shot message
DEEPSEEK_API_KEY=sk-... synapse "What is Rust?"

# Piped input
echo "Explain async/await" | DEEPSEEK_API_KEY=sk-... synapse
```

### Switch to Anthropic
Set in `config.toml`:
```toml
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
```

Then run:
```bash
ANTHROPIC_API_KEY=sk-ant-... synapse "Hello, Claude"
```

### Error Messages
- Missing API key: "missing API key: Set DEEPSEEK_API_KEY environment variable or add api_key to config.toml"
- Unknown provider: "unknown provider: invalid"
- Invalid API key: "authentication failed: Incorrect API key provided"

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-core/src/provider/deepseek.rs` | New file - DeepSeekProvider implementation |
| `synapse-core/src/provider/factory.rs` | New file - Provider factory with create_provider() |
| `synapse-core/src/provider.rs` | Added mod deepseek, mod factory, MissingApiKey, UnknownProvider |
| `synapse-core/src/lib.rs` | Export DeepSeekProvider, create_provider |
| `synapse-cli/src/main.rs` | Use factory instead of hardcoded AnthropicProvider |

---

## Module Structure

```
synapse-core/src/
  lib.rs              # pub use provider::{DeepSeekProvider, create_provider, ...}
  provider.rs         # mod anthropic; mod deepseek; mod factory; mod mock;
  provider/
    anthropic.rs      # AnthropicProvider (unchanged)
    deepseek.rs       # DeepSeekProvider, API types
    factory.rs        # create_provider(), get_api_key()
    mock.rs           # MockProvider (existing)

synapse-cli/src/
  main.rs             # Uses create_provider(&config)
```

---

## Security Considerations

- API key is resolved from environment variable (preferred) or config file
- Environment variable takes precedence to avoid secrets in config files
- HTTPS enforced via hardcoded `https://api.deepseek.com` endpoint
- API key is never logged or included in error messages
- Bearer token format used for authorization (industry standard)

---

## Future Work

This implementation enables:
- **OpenAI Provider** - Can reuse DeepSeek's OpenAI-compatible format
- **Streaming responses** - SSE streaming for both providers
- **Additional providers** - Factory pattern makes adding providers simple
- **Configurable max_tokens** - Currently hardcoded to 1024
- **Model validation** - Currently passes model name as-is to API
