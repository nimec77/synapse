# Phase 13: System Prompt

**Goal:** Wire `config.system_prompt` through the Agent to all provider calls.

## Tasks

- [x] 13.1 Add `system_prompt: Option<String>` to `Config` struct
- [x] 13.2 Add `system_prompt` field and `with_system_prompt()` builder to `Agent`
- [x] 13.3 Implement `build_messages()` helper to prepend `Role::System` on-the-fly
- [x] 13.4 Wire system prompt from config/session into Agent in CLI and Telegram
- [x] 13.5 Update `config.example.toml` with `system_prompt` example

## Acceptance Criteria

**Test:** Setting `system_prompt` in config causes a `Role::System` message to be prepended to every provider call.

## Dependencies

- Phase 12 complete (Telegram Bot)

## Implementation Notes

The plumbing already exists (`Role::System`, `Session.system_prompt`, DB column) but nothing injects a system prompt today. The goal is to wire `config.system_prompt` through the `Agent` so it prepends a `Role::System` message before every provider call without mutating the stored session history.

Key design points:
- `build_messages()` constructs the final `Vec<Message>` slice passed to the provider — it prepends the system message on-the-fly so the DB never stores duplicate system messages.
- `with_system_prompt()` is a builder method on `Agent` for ergonomic construction.
- Both `synapse-cli` and `synapse-telegram` should read `config.system_prompt` and pass it to the agent at startup.
