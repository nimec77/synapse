# Phase 17: Configurable max_tokens (SY-18)

**Goal:** Allow `max_tokens` to be set in `config.toml` with a sensible default of 4096 so long LLM responses are no longer silently truncated.

## Tasks

- [x] 17.1 Add `max_tokens: Option<u32>` to `Config` struct; default to `4096` via `fn default_max_tokens()`
- [x] 17.2 Pass `max_tokens` through `create_provider()` factory and store in each provider (`AnthropicProvider`, `DeepSeekProvider`, `OpenAIProvider`)
- [x] 17.3 Use the stored value in every API request (complete, complete_with_tools, stream, stream_with_tools) replacing the hardcoded `1024`
- [x] 17.4 Update `config.example.toml` with `max_tokens` field and inline comment; add unit test `test_config_default_max_tokens`

## Acceptance Criteria

**Test:** `cargo test -p synapse-core` green; setting `max_tokens = 8192` in config produces a request body with `"max_tokens": 8192`.

## Dependencies

- Phase 16 complete

## Implementation Notes

- Default value of `4096` applied via `#[serde(default = "default_max_tokens")]` to avoid `Option` unwrapping at every call site
- All three providers (`AnthropicProvider`, `DeepSeekProvider`, `OpenAIProvider`) store `max_tokens: u32` and use it in all four provider methods
