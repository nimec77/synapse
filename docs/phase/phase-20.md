# Phase 20: Improve /history Command (SY-20)

**Goal:** Replace the full message dump in `/history` with a compact "last 10 messages" view, filtering out System/Tool roles and truncating content.

## Tasks

- [x] 20.1 Update `cmd_history` in `synapse-telegram/src/commands.rs`: filter to `User`/`Assistant` roles only, take last 10, truncate each message content to 150 chars
- [x] 20.2 Update `Command::History` description from "Show conversation history" to "Show recent messages"
- [x] 20.3 Add unit tests for the truncation and filtering logic
- [x] 20.4 Update documentation: CLAUDE.md, README.md, CHANGELOG.md

## Acceptance Criteria

**Test:** `/history` returns at most 10 messages, System and Tool role messages are absent, long messages are truncated at 150 chars with an ellipsis.

## Dependencies

- Phase 19 complete

## Implementation Notes

