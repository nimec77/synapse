# Phase File Template

Used by `sync-phases` and `phase-loop` when extracting an incomplete phase from `docs/tasklist.md` into its own `docs/phase/phase-N.md` file.

```markdown
# Phase N: <title from `## Phase N: <title>` in tasklist.md>

**Goal:** <copied from `**Goal:**` line, or derived from the phase description if missing>

## Tasks

- [ ] N.1 <task description, copied verbatim from the phase section in tasklist.md>
- [ ] N.2 <task description>
- [ ] N.3 <task description>

## Acceptance Criteria

**Test:** <copied from `**Test:**` or `**Verify:**` line, or "All tasks above pass `cargo fmt --check && cargo clippy -- -D warnings && cargo test`" if absent>

## Dependencies

- Phase N-1 complete
- <any other dependencies named in the Feature Phases table `Depends on` column>

## Implementation Notes

<empty placeholder; preserve any existing notes if the file was being regenerated>
```

## Rules

- The phase file is the **source of truth** for task completion within that phase. The tasklist's Feature Phases table mirrors it.
- Never overwrite an existing `## Implementation Notes` section.
- Preserve any extra sections an earlier author added (`## Open Questions`, `## Risks`, etc.).
- Task numbering uses `N.M` (`1.1`, `1.2`, …) to match the tasklist convention.
