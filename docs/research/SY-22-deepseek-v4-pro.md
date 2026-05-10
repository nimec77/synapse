# SY-22 Research: DeepSeek V4 Pro Reasoning Model Wire Format

**Ticket:** SY-22
**Date:** 2026-05-10
**Reference:** https://api-docs.deepseek.com/guides/thinking_mode

---

## Summary

DeepSeek V4 Pro is a reasoning model that supports **thinking mode**: the model produces a
chain-of-thought trace (`reasoning_content`) alongside the user-visible final answer (`content`).
The wire format extends the standard OpenAI-compatible Chat Completions API with a small number of
additional fields.

---

## Request Fields (thinking-mode enabled)

Send these top-level fields on every request to a reasoning-capable model:

```json
{
  "model": "deepseek-v4-pro",
  "messages": [...],
  "max_tokens": 8192,
  "thinking": { "type": "enabled" },
  "reasoning_effort": "high"
}
```

| Field | Type | Values | Notes |
|-------|------|--------|-------|
| `thinking` | object | `{"type": "enabled"}` | Activates thinking mode. Must be exactly this shape. |
| `reasoning_effort` | string | `"low"`, `"medium"`, `"high"`, `"max"` | Controls thinking depth. DeepSeek docs suggest `"high"` as the default and `"max"` for complex agent flows. |

**Unsupported parameters in thinking mode:** `temperature`, `top_p`, `presence_penalty`,
`frequency_penalty`. Synapse never sends these, so no conflict.

---

## Response Fields

### Non-streaming

```json
{
  "choices": [{
    "message": {
      "role": "assistant",
      "content": "The final answer.",
      "reasoning_content": "Let me think step by step...",
      "tool_calls": null
    }
  }]
}
```

`reasoning_content` is a sibling of `content` on the choice message. Both are always present when
thinking mode is active; either may be `null` on a given turn.

### Streaming deltas

```
data: {"choices":[{"delta":{"reasoning_content":"Step 1: "},"finish_reason":null}]}
data: {"choices":[{"delta":{"content":"The answer is 42."},"finish_reason":null}]}
data: [DONE]
```

`delta.reasoning_content` and `delta.content` arrive as incremental tokens on separate SSE events.
Synapse yields `StreamEvent::ReasoningDelta` for reasoning tokens and `StreamEvent::TextDelta` for
answer tokens.

---

## Tool-Call History Rule (CRITICAL)

From the DeepSeek documentation:

> "The intermediate `assistant`'s `reasoning_content` must participate in the context concatenation
> and must be passed back to the API in all subsequent user interaction turns."

This rule **only applies to turns that also contain tool calls**. Concretely:

- **Tool-call turn** (`tool_calls.is_some() && reasoning_content.is_some()`): the
  `reasoning_content` MUST be included on the outbound `ApiMessage` for that historical assistant
  turn on every subsequent API call.
- **Text-only turn** (`tool_calls.is_none()`): `reasoning_content` MAY be dropped from outbound
  history to save tokens.

Synapse implements this policy in `build_api_messages` (`openai_compat.rs`) and persists
`reasoning_content` to SQLite so resumed sessions continue to honour the rule.

Violating this rule causes mid-conversation 400 errors from the DeepSeek API.

---

## Effort Auto-Escalation

When MCP tools are configured and the user has not set `reasoning_effort` explicitly in config,
Synapse auto-escalates to `"max"` (mirroring documented behaviour for complex agent flows). Explicit
user values always win. The factory emits a `tracing::info!` event with `auto_escalated = true` when
this occurs.

Effort resolution priority:
1. `config.reasoning_effort` (explicit) → use as-is, no escalation
2. `config.mcp.is_some()` AND no explicit effort → `"max"` (auto-escalated)
3. No MCP, no explicit → `"high"` (default)

---

## Known Reasoning-Capable DeepSeek Models

Kept in sync with `REASONING_MODELS` in `synapse-core/src/provider/deepseek.rs`:

```rust
pub(super) const REASONING_MODELS: &[&str] = &["deepseek-v4-pro"];
```

Add new models to this list as DeepSeek releases them. The list is the single source of truth for
whether thinking mode fields are sent.

---

## `max_tokens` Recommendation

`max_tokens` covers the combined output (reasoning + answer). The default of 4096 may be too low for
`reasoning_effort = "max"` on complex tasks. Recommendation: set `max_tokens = 8192` or higher when
using `"max"` effort.

---

## References

- DeepSeek API docs: https://api-docs.deepseek.com/
- Thinking mode guide: https://api-docs.deepseek.com/guides/thinking_mode
- Synapse implementation: `synapse-core/src/provider/openai_compat.rs`, `deepseek.rs`, `factory.rs`
- Tasklist: `docs/tasklist/SY-22.md`
