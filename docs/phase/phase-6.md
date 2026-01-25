# Phase 6: DeepSeek Provider

**Ticket:** SY-7
**Goal:** Default provider works out of the box.

## Overview

Implement the DeepSeek LLM provider as the default provider for Synapse. DeepSeek uses an OpenAI-compatible API, making it straightforward to integrate.

## Tasks

- [x] 6.1 Create `synapse-core/src/provider/deepseek.rs` with `DeepSeekProvider`
- [x] 6.2 Implement OpenAI-compatible chat/completions API request
- [x] 6.3 Create `synapse-core/src/provider/factory.rs` with provider selection
- [x] 6.4 Update CLI to use factory based on `config.provider`
- [x] 6.5 Support `DEEPSEEK_API_KEY` environment variable

## Technical Details

### DeepSeek API

- **Base URL:** `https://api.deepseek.com`
- **Endpoint:** `POST /chat/completions`
- **Format:** OpenAI-compatible (same request/response structure)
- **Models:** `deepseek-chat`, `deepseek-reasoner`

### Provider Factory

The factory will select providers based on config:

```rust
pub fn create_provider(config: &Config) -> Box<dyn LlmProvider> {
    match config.provider.as_str() {
        "deepseek" => Box::new(DeepSeekProvider::new(config)),
        "anthropic" => Box::new(AnthropicProvider::new(config)),
        _ => Box::new(DeepSeekProvider::new(config)), // default
    }
}
```

### Configuration

```toml
[provider]
default = "deepseek"

[provider.deepseek]
api_key = "sk-..."  # or use DEEPSEEK_API_KEY env var
model = "deepseek-chat"
```

## Acceptance Criteria

1. `DeepSeekProvider` implements `LlmProvider` trait
2. Provider factory selects correct provider based on config
3. `DEEPSEEK_API_KEY` environment variable is supported
4. `synapse "Hello"` with default config returns DeepSeek response
5. All existing tests continue to pass

## Test Plan

```bash
# Unit tests
cargo test -p synapse-core

# Integration test (requires API key)
DEEPSEEK_API_KEY=sk-... cargo run -p synapse-cli -- "Hello"
```
