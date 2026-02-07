# Phase 10: OpenAI Provider

**Goal:** Support OpenAI alongside DeepSeek and Anthropic.

## Tasks

- [ ] 10.1 Create `synapse-core/src/provider/openai.rs` implementing `LlmProvider`
- [ ] 10.2 Add provider selection in config and CLI flag (`-p openai`)
- [ ] 10.3 Implement streaming for OpenAI API

## Acceptance Criteria

**Test:** `synapse -p openai "Hello"` uses GPT, default uses DeepSeek.

## Dependencies

- Phase 9 complete (CLI REPL)

## Implementation Notes

