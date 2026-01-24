# SY-6: Phase 5: Anthropic Provider

Status: PRD_READY

## Context / Idea

Phase 5 of the Synapse project focuses on implementing real API calls to Claude (Anthropic's LLM). This is the first concrete LLM provider implementation, moving from the mock provider created in Phase 4 to actual API integration.

The goal is to enable users to send messages to Claude and receive real responses through the CLI. This phase builds upon the existing `LlmProvider` trait and `Message` types from Phase 4, implementing a working Anthropic provider that uses the Messages API.

### Background from Phase 4

Phase 4 established:
- `Role` enum (System, User, Assistant) in `synapse-core/src/message.rs`
- `Message` struct with role and content fields
- `LlmProvider` trait with async `complete` method in `synapse-core/src/provider.rs`
- `MockProvider` for testing purposes
- `ProviderError` enum for error handling

### Current State

- Configuration system loads API key from `config.toml` (optional field)
- CLI currently echoes input back with "Echo: " prefix
- Provider abstraction exists but no real provider is implemented

## Goals

1. **Implement Anthropic Provider**: Create `AnthropicProvider` struct that implements `LlmProvider` trait and makes real API calls to Claude.

2. **Add HTTP Client Support**: Integrate `reqwest` with JSON support for making HTTPS requests to the Anthropic Messages API.

3. **Extend Error Handling**: Enhance `ProviderError` to handle API-specific errors including HTTP status codes and Anthropic error responses.

4. **Wire Provider to CLI**: Connect the configuration, provider, and CLI so that running `synapse "message"` sends the message to Claude and displays the response.

5. **Validate API Key**: Implement fail-fast validation when API key is missing or invalid.

## User Stories

### US-1: Send Message to Claude
**As a** developer using Synapse
**I want to** send a message from the CLI and receive a Claude response
**So that** I can interact with the AI assistant through my terminal

**Acceptance Criteria:**
- Running `synapse "Say hello"` returns a real response from Claude
- The response is printed to stdout
- Works with piped input: `echo "Say hello" | synapse`

### US-2: Configure Anthropic API Key
**As a** user
**I want to** configure my Anthropic API key in the config file
**So that** Synapse can authenticate with the Anthropic API

**Acceptance Criteria:**
- API key is read from `config.toml` under `api_key` field
- Clear error message if API key is missing
- Application exits with non-zero code if key is missing

### US-3: Handle API Errors Gracefully
**As a** user
**I want to** see meaningful error messages when API calls fail
**So that** I can understand and fix the problem

**Acceptance Criteria:**
- Invalid API key shows authentication error
- Network failures show connection error
- Rate limits show appropriate message
- All errors are human-readable

## Main Scenarios

### Scenario 1: Successful API Call
1. User runs `synapse "What is Rust?"`
2. CLI loads configuration from `config.toml`
3. CLI validates API key is present
4. CLI creates `AnthropicProvider` with API key and model
5. CLI creates user message
6. Provider sends request to `https://api.anthropic.com/v1/messages`
7. Provider receives response and parses it
8. CLI prints Claude's response to stdout

### Scenario 2: Missing API Key
1. User runs `synapse "Hello"` without configuring API key
2. CLI loads configuration
3. CLI detects API key is None
4. CLI prints error: "Error: API key not configured. Add api_key to config.toml"
5. CLI exits with code 1

### Scenario 3: Invalid API Key
1. User runs `synapse "Hello"` with invalid API key
2. CLI creates provider and sends request
3. Anthropic API returns 401 Unauthorized
4. Provider maps to `ProviderError::AuthenticationError`
5. CLI prints: "Error: Authentication failed. Check your API key."
6. CLI exits with code 1

### Scenario 4: Network Failure
1. User runs `synapse "Hello"` while offline
2. Provider attempts to connect to API
3. `reqwest` returns connection error
4. Provider maps to `ProviderError::RequestFailed`
5. CLI prints: "Error: Failed to connect to Anthropic API"
6. CLI exits with code 1

## Success / Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Successful API call | Works with valid key | Manual test: `synapse "Say hello"` returns response |
| Error handling | All error types handled | Unit tests for each ProviderError variant |
| Response time | < 3 seconds for short prompts | Manual timing (depends on API) |
| Test coverage | MockProvider and unit tests pass | `cargo test` passes |

## Constraints and Assumptions

### Constraints

1. **No Streaming (This Phase)**: This phase implements non-streaming completion only. SSE streaming is planned for a future phase.

2. **Single Provider**: Only Anthropic provider is implemented. Provider selection logic is not needed yet.

3. **Simple Message Format**: Only text content is supported. Tool calls, images, and other content types are out of scope.

4. **Synchronous CLI Flow**: The CLI waits for the complete response before printing. No progress indicators.

### Assumptions

1. User has a valid Anthropic API key
2. Network access to `api.anthropic.com` is available
3. Phase 4 is complete (LlmProvider trait and Message types exist)
4. Configuration loading works correctly (from SY-4)

### Technical Constraints

- Use `reqwest` with `json` feature for HTTP requests
- Follow Anthropic Messages API v1 specification
- Required headers: `x-api-key`, `anthropic-version`, `content-type`
- API endpoint: `https://api.anthropic.com/v1/messages`

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API changes | Low | Medium | Pin `anthropic-version` header, handle version errors |
| Rate limiting | Medium | Low | Implement proper error handling, document in user guide |
| Async runtime integration | Low | Medium | Use tokio runtime in CLI main, test thoroughly |
| Error response format changes | Low | Medium | Defensive parsing, log unexpected responses |

## Open Questions

None. The scope is well-defined by:
- Phase 5 task breakdown
- Anthropic Messages API documentation
- Existing LlmProvider trait interface
- Current CLI implementation

## Implementation Notes

### Anthropic Messages API Request Format

```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 1024,
  "messages": [
    {"role": "user", "content": "Hello, Claude"}
  ]
}
```

### Required Headers

```
x-api-key: <API_KEY>
anthropic-version: 2023-06-01
content-type: application/json
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

### Task Breakdown (from Phase 5)

- [ ] 5.1 Create `synapse-core/src/provider/anthropic.rs` with `AnthropicProvider` struct
- [ ] 5.2 Add `reqwest` (with `json` feature), implement Messages API request
- [ ] 5.3 Create `synapse-core/src/error.rs` with `ProviderError` enum (extend existing)
- [ ] 5.4 Wire provider into CLI: load config -> create provider -> call API
- [ ] 5.5 Add API key validation (fail fast if missing)

## References

- `docs/phase/phase-5.md` - Phase task breakdown
- `docs/vision.md` - Technical architecture
- `synapse-core/src/provider.rs` - LlmProvider trait
- `synapse-core/src/message.rs` - Message types
- `synapse-core/src/config.rs` - Configuration loading
- [Anthropic Messages API](https://docs.anthropic.com/en/api/messages)
