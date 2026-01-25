# SY-8: Streaming Responses

Status: PRD_READY

## Context / Idea

### Feature Description

Phase 7: Streaming Responses - Implement token-by-token output to the terminal for real-time response display.

### Background

Currently, the Synapse CLI waits for the entire LLM response before displaying it to the user. This creates a suboptimal user experience, especially for longer responses where the user must wait several seconds before seeing any output. Implementing streaming responses will display tokens as they arrive from the LLM provider, providing immediate feedback and a more responsive feel.

This feature is fundamental to the project's goal of creating a "daily usability" tool (per `docs/idea.md`). Real-time streaming is a standard expectation for modern AI chat interfaces.

### Technical Context

From `docs/vision.md`, the architecture already anticipates streaming:
- **Dependencies planned**: `eventsource-stream` for SSE parsing, `async-stream` and `futures` for async stream handling
- **Data Flow**: "LlmProvider streams response tokens back" and "Response streams back to Interface -> User"
- **StreamEvent enum**: Already designed in the vision document
- **Performance target**: First token latency < 500ms

The DeepSeek provider (current default) uses an OpenAI-compatible API that supports SSE streaming via the `stream: true` parameter. The Anthropic provider also supports SSE streaming.

### Phase Description (from `docs/phase/phase-7.md`)

The phase includes:
- Adding streaming dependencies to synapse-core
- Creating `StreamEvent` enum for event types
- Implementing SSE parsing in `DeepSeekProvider::stream()` method
- Updating CLI to print tokens as they arrive

## Goals

1. **Improve User Experience**: Provide immediate feedback by displaying response tokens as they are generated, eliminating the perceived delay of batch responses.

2. **Extend Provider Abstraction**: Add a `stream()` method to the `LlmProvider` trait, maintaining the clean provider abstraction while supporting streaming.

3. **Implement SSE Parsing**: Parse Server-Sent Events (SSE) from the DeepSeek API (OpenAI-compatible format) for real-time token delivery.

4. **Enable Progressive CLI Output**: Update the CLI to handle streamed responses, printing tokens incrementally without buffering.

5. **Support Graceful Interruption**: Allow users to cancel streaming responses with Ctrl+C without crashing.

## User Stories

### US-1: Progressive Response Display
**As a** CLI user
**I want** to see the AI response appear token-by-token
**So that** I get immediate feedback and can start reading while the response is being generated

### US-2: Interruptible Streaming
**As a** CLI user
**I want** to cancel a streaming response with Ctrl+C
**So that** I can stop a long or irrelevant response without waiting for completion

### US-3: Provider-Agnostic Streaming
**As a** developer extending Synapse
**I want** streaming to be defined in the `LlmProvider` trait
**So that** I can implement streaming for new providers consistently

## Main Scenarios

### Scenario 1: Normal Streaming Response
1. User runs `synapse "Count from 1 to 10 slowly"`
2. CLI sends request to DeepSeek with `stream: true`
3. Tokens arrive via SSE: "1", " ", "2", " ", "3", ...
4. CLI prints each token immediately as it arrives
5. When stream completes, CLI returns to prompt

### Scenario 2: Streaming with Ctrl+C Interruption
1. User runs `synapse "Write a long essay about AI"`
2. Tokens begin streaming to terminal
3. User presses Ctrl+C after a few sentences
4. Streaming stops gracefully
5. Partial response displayed, CLI exits cleanly

### Scenario 3: Network Error During Streaming
1. User runs `synapse "Tell me a story"`
2. Streaming begins, tokens arrive
3. Network connection drops mid-stream
4. CLI displays error message after partial response
5. User sees what was received plus error notification

### Scenario 4: Empty or Error Response
1. User runs `synapse "..."`
2. API returns an error or empty stream
3. CLI displays appropriate error message
4. User can retry the command

## Success / Metrics

### Functional Criteria
- [ ] `StreamEvent` enum defined with: `TextDelta`, `ToolCall`, `ToolResult`, `Done`, `Error`
- [ ] `LlmProvider` trait extended with `stream()` method
- [ ] `DeepSeekProvider` implements streaming via SSE
- [ ] CLI displays tokens progressively (no buffering)
- [ ] Ctrl+C cleanly interrupts streaming

### Technical Criteria
- [ ] First token displayed within 500ms of request (provider latency permitting)
- [ ] No memory leaks during long streaming responses
- [ ] Stream properly handles SSE `[DONE]` event
- [ ] Error events propagated correctly through stream

### Test Criteria
- [ ] Unit tests for SSE event parsing
- [ ] Unit tests for `StreamEvent` serialization
- [ ] Manual test: `cargo run -p synapse-cli -- "Count from 1 to 10 slowly"` shows progressive output
- [ ] Manual test: Ctrl+C during streaming exits cleanly

## Constraints and Assumptions

### Constraints
1. **Rust Nightly/Edition 2024**: Must use async streams compatible with current toolchain
2. **Trait Object Safety**: The `stream()` method signature must work with `dyn LlmProvider` if needed (may require `async-trait` or return type adjustments)
3. **DeepSeek First**: Implement streaming only for DeepSeek provider in this phase; Anthropic streaming is a separate task

### Assumptions
1. DeepSeek API supports SSE streaming with `stream: true` parameter (confirmed by OpenAI compatibility)
2. SSE format follows OpenAI standard: `data: {...}\n\n` with `data: [DONE]` for completion
3. The `reqwest` client can be configured for streaming responses
4. Terminal output (stdout) can handle unbuffered writes efficiently

### Dependencies
- Phase 6 complete (DeepSeek Provider with factory pattern) - **Confirmed complete**
- `eventsource-stream` crate available and compatible
- `async-stream` crate for stream generation
- `futures` crate for `Stream` trait and combinators

## Risks

### R-1: Trait Object Compatibility (Medium)
**Risk**: The `stream()` method may not be object-safe due to `impl Stream` return type.
**Mitigation**: Use `Pin<Box<dyn Stream>>` return type or keep the trait generic with associated types. Consider keeping `LlmProvider` non-object-safe if only used generically.

### R-2: SSE Parsing Edge Cases (Low)
**Risk**: DeepSeek SSE format may have undocumented quirks.
**Mitigation**: Test with real API responses; implement robust parsing that handles unexpected formats gracefully.

### R-3: Backpressure and Terminal Performance (Low)
**Risk**: Very fast token streams could overwhelm terminal rendering.
**Mitigation**: Unlikely for LLM output speeds; if needed, add minimal buffering or rate limiting.

### R-4: Ctrl+C Signal Handling (Low)
**Risk**: Improper signal handling could leave resources in inconsistent state.
**Mitigation**: Use tokio's signal handling; ensure streams are properly dropped on cancellation.

## Open Questions

None blocking. The phase description and vision document provide sufficient detail for implementation. Questions that may arise during implementation:

1. **Tool Calls in Streaming**: Should `ToolCall` events be handled in this phase, or deferred until MCP integration (Phase 11)?
   *Recommendation*: Define the enum variants now, implement parsing only for `TextDelta` and `Done` in this phase.

2. **Anthropic Streaming**: Should this phase also implement streaming for `AnthropicProvider`?
   *Recommendation*: Defer to Phase 10 (OpenAI Provider) or a separate ticket, keeping this phase focused on DeepSeek.

3. **Fallback to Non-Streaming**: Should streaming be optional with automatic fallback?
   *Recommendation*: Make streaming the default behavior; the `complete()` method remains for programmatic use cases.
