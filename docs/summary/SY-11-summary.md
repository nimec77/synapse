# SY-11 Summary: Phase 10 - OpenAI Provider

**Status:** COMPLETE
**Date:** 2026-02-07

---

## Overview

SY-11 adds an `OpenAiProvider` implementing the `LlmProvider` trait for the OpenAI Chat Completions API, registers it in the provider factory, and introduces a `-p` / `--provider` CLI flag for runtime provider override. Both `complete()` and `stream()` methods are fully implemented.

The key insight driving the implementation is that OpenAI and DeepSeek share an identical wire protocol (Chat Completions API with the same JSON request/response shapes and SSE streaming format). This allowed the `OpenAiProvider` to follow the established `DeepSeekProvider` pattern exactly, differing only in the API endpoint URL (`https://api.openai.com/v1/chat/completions`) and environment variable name (`OPENAI_API_KEY`).

---

## What Was Built

### New Components

1. **OpenAI Provider** (`synapse-core/src/provider/openai.rs` -- 556 lines)
   - `OpenAiProvider` struct with `client`, `api_key`, `model` fields and `new()` constructor
   - Private API types: `ApiRequest`, `StreamingApiRequest`, `ApiMessage`, `ApiResponse`, `Choice`, `ChoiceMessage`, `ApiError`, `ErrorDetail`, `StreamChunk`, `StreamChoice`, `StreamDelta`
   - `LlmProvider::complete()`: POST to OpenAI Chat Completions API, handle 401 as `AuthenticationError`, extract content from first choice
   - `LlmProvider::stream()`: POST with `stream: true`, parse SSE via `eventsource_stream::Eventsource`, yield `TextDelta` for each content delta, handle `[DONE]` marker
   - 10 unit tests covering construction, serialization, parsing, and SSE handling

### Modified Components

1. **Module Registration** (`synapse-core/src/provider.rs`)
   - Added `mod openai;` declaration and `pub use openai::OpenAiProvider;` re-export

2. **Public Export** (`synapse-core/src/lib.rs`)
   - Added `OpenAiProvider` to the `pub use provider::{ ... }` statement

3. **Provider Factory** (`synapse-core/src/provider/factory.rs`)
   - Added `"openai"` to the provider name validation match arm
   - Added `"openai" => "OPENAI_API_KEY"` mapping in `get_api_key()`
   - Added `"openai" => Ok(Box::new(OpenAiProvider::new(...)))` in provider creation match
   - 2 new unit tests: `test_create_provider_openai`, `test_get_api_key_missing_openai`

4. **CLI Provider Flag** (`synapse-cli/src/main.rs`)
   - Added `provider: Option<String>` field to `Args` with `#[arg(short = 'p', long)]`
   - Changed `config` binding to `mut` and added provider override before any `create_provider()` call
   - 4 new unit tests for flag parsing

---

## Key Decisions

### 1. Separate Provider File (Code Duplication Accepted)

**Decision:** Create `openai.rs` as a standalone module duplicating the `DeepSeekProvider` pattern rather than extracting a shared base.

**Rationale:** The PRD explicitly acknowledged this as a conscious trade-off (PRD Risk 1, Plan Risk table). Extracting a shared `OpenAiCompatibleProvider` base is deferred to a future refactoring ticket. The separate-file approach matches the project's existing pattern and keeps each provider self-contained with its own private API types.

### 2. Private API Types Redeclared Per Module

**Decision:** Redeclare `ApiRequest`, `ApiResponse`, etc. privately within `openai.rs` rather than sharing types with `deepseek.rs`.

**Rationale:** Each provider's API types are implementation details, not part of the public interface. Keeping them private per module avoids coupling between providers and allows each to evolve independently if API differences emerge.

### 3. CLI Flag Applies Before All Paths

**Decision:** Apply the `-p` provider override immediately after config loading, before any branching (REPL, one-shot, session resume, stdin).

**Rationale:** A single override point at the top of `main()` ensures consistent behavior across all CLI modes. The `config` is made `mut` and the provider string is replaced before any call to `create_provider()`.

### 4. No Per-Provider Default Models

**Decision:** Do not add per-provider default model fallbacks for this ticket.

**Rationale:** When `-p openai` is used with the default config, `config.model` is `"deepseek-chat"` which is invalid for OpenAI. The OpenAI API returns a descriptive error ("model not found"). This was acknowledged as a known limitation in the PRD (Risk 3) and Plan (Risk table), with per-provider defaults deferred to a future ticket.

### 5. No New Dependencies

**Decision:** No new crates were added to `synapse-core/Cargo.toml`.

**Rationale:** The OpenAI API uses the same request/response JSON format and SSE streaming protocol as DeepSeek. All required crates (`reqwest`, `serde`, `serde_json`, `async-stream`, `eventsource-stream`, `futures`) are already present.

---

## Data Flow

### One-shot with CLI Flag
```
User runs: synapse -p openai "Hello"
  -> Args::parse() captures provider = Some("openai")
  -> Config::load() returns config with provider = "deepseek" (default)
  -> Override: config.provider = "openai"
  -> create_provider(&config) matches "openai"
  -> get_api_key() checks OPENAI_API_KEY env var, falls back to config.api_key
  -> OpenAiProvider::new(api_key, &config.model)
  -> provider.stream(&messages) -> POST to https://api.openai.com/v1/chat/completions
  -> SSE events parsed, TextDelta tokens printed to stdout
  -> StreamEvent::Done -> store response, exit
```

### Config-based Provider Selection
```
config.toml: provider = "openai", model = "gpt-4o"
  -> Config::load() returns config with provider = "openai"
  -> No -p flag override needed
  -> create_provider(&config) matches "openai"
  -> Normal flow via OpenAI API
```

---

## Testing

### OpenAI Provider Unit Tests (10 tests in `openai.rs`)

| Test | Coverage |
|------|----------|
| `test_openai_provider_new` | Constructor stores key and model |
| `test_api_request_serialization` | Non-streaming request JSON structure |
| `test_api_request_with_system_message` | System message in messages array |
| `test_api_response_parsing` | Response with extra fields parses correctly |
| `test_api_error_parsing` | Error response with extra fields parses correctly |
| `test_streaming_request_serialization` | Streaming request has `stream: true` |
| `test_parse_sse_text_delta` | SSE chunk with delta content |
| `test_parse_sse_done` | Final chunk with finish_reason and `[DONE]` marker |
| `test_parse_sse_empty_content` | Empty content filtered out |
| `test_parse_sse_with_role` | First SSE event with role only (no content) |

### Factory Tests (2 new tests in `factory.rs`)

| Test | Coverage |
|------|----------|
| `test_create_provider_openai` | Factory creates provider with `OPENAI_API_KEY` set |
| `test_get_api_key_missing_openai` | Missing key error mentions `OPENAI_API_KEY` |

### CLI Flag Tests (4 new tests in `main.rs`)

| Test | Coverage |
|------|----------|
| `test_args_with_provider_flag` | `-p openai "Hello"` parses correctly |
| `test_args_with_provider_long_flag` | `--provider openai "Hello"` parses correctly |
| `test_args_provider_with_repl` | `-p openai --repl` parses both flags |
| `test_args_provider_default_none` | No `-p` flag results in `None` |

**Total new tests: 16 (10 provider + 2 factory + 4 CLI)**
**Total project tests: 147 passed, 0 failed**

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-core/src/provider/openai.rs` | New -- `OpenAiProvider` struct, `LlmProvider` impl, API types, 10 unit tests |
| `synapse-core/src/provider.rs` | Modified -- `mod openai;` declaration, `pub use openai::OpenAiProvider;` |
| `synapse-core/src/provider/factory.rs` | Modified -- `"openai"` in validation, key resolution, creation matches; 2 new tests |
| `synapse-core/src/lib.rs` | Modified -- `OpenAiProvider` added to public exports |
| `synapse-cli/src/main.rs` | Modified -- `-p`/`--provider` flag, config override logic, 4 new tests |

---

## Module Structure

```
synapse-core/src/
  provider.rs               # mod openai; pub use openai::OpenAiProvider;
  provider/
    openai.rs               # OpenAiProvider, LlmProvider impl, API types, tests
    factory.rs              # "openai" case in validation, key lookup, creation
    deepseek.rs             # (unchanged)
    anthropic.rs            # (unchanged)
    mock.rs                 # (unchanged)
    streaming.rs            # (unchanged)

synapse-cli/src/
  main.rs                   # -p/--provider flag, config.provider override
```

---

## Usage

### Runtime Provider Override
```bash
# One-shot with OpenAI
synapse -p openai "Hello"

# Long form
synapse --provider openai "Hello"

# REPL mode with OpenAI
synapse -p openai --repl

# Session resume with OpenAI
synapse -p openai -s <uuid> "Hello"

# Stdin with OpenAI
echo "Hello" | synapse -p openai
```

### Config-based Selection
```toml
# config.toml
provider = "openai"
model = "gpt-4o"
api_key = "sk-..."
```

### Environment Variable
```bash
export OPENAI_API_KEY="sk-..."
synapse -p openai "Hello"
```

---

## Known Limitations

1. **Default model mismatch**: When `-p openai` is used without explicit model configuration, the default model (`"deepseek-chat"`) is sent to OpenAI, which rejects it. The API error message provides sufficient guidance. Per-provider default models deferred to a future ticket.
2. **Code duplication with DeepSeek**: `OpenAiProvider` is a near-exact copy of `DeepSeekProvider` with different constants. Extracting a shared `OpenAiCompatibleProvider` base deferred to a future refactoring ticket.
3. **No integration tests with live API**: All tests are unit tests. Live API testing requires an API key and is not automated.
4. **Env var test race conditions**: Factory tests manipulate global environment variables with `unsafe` blocks, matching the pre-existing pattern for DeepSeek and Anthropic factory tests.

---

## Future Work

This implementation enables:
- **Per-provider default models** -- Automatic model selection based on provider name (e.g., `"openai"` defaults to `"gpt-4o"`)
- **Shared OpenAI-compatible base** -- Extract common logic from `DeepSeekProvider` and `OpenAiProvider` into a parameterized `OpenAiCompatibleProvider`
- **Additional OpenAI-compatible providers** -- Groq, Together.ai, Mistral, etc. using the same API format
- **Model listing** -- `synapse models` command to show available models per provider
