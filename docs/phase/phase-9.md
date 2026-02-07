# Phase 9: CLI REPL

**Goal:** Interactive chat mode.

## Tasks

- [ ] 9.1 Add `ratatui` + `crossterm` to CLI
- [ ] 9.2 Create `synapse-cli/src/repl.rs` with input loop
- [ ] 9.3 Implement `--repl` flag to enter interactive mode
- [ ] 9.4 Add session resume: `synapse --repl --session <id>`

## Acceptance Criteria

**Test:** `synapse --repl` allows multi-turn conversation with history.

## Dependencies

- Phase 8 complete (Session Storage)

## Implementation Notes

