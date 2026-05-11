# Tasklist Template

Produced by the `tasklist` skill at `docs/tasklist/$TICKET_ID.md`. Read by `implement-orchestrated`, `dev-cycle`, `run-reviewer`, and `phase-loop`.

```markdown
# <TICKET_ID>: <ticket title>

**Status:** TASKLIST_READY

## Context

<2-4 sentence summary: what this ticket delivers and which PRD/plan it derives from>

## Tasks

- [ ] 1.1 <task description>
  - **Acceptance:** <verifiable criterion derived from the PRD>
- [ ] 1.2 <task description>
  - **Acceptance:** <criterion>
- [ ] 2.1 <task description>
  - **Acceptance:** <criterion>

## Deviations to Fix

<List items here only if the plan identified existing-code deviations. Each becomes a task above. Delete this whole section if there are none.>

## Review Fixes

<Empty placeholder. `run-reviewer` adds tasks here as `RF1`, `RF2`, … when issues are found. Delete this section if no review has run yet.>
```

## Rules

- Each task has at least one **Acceptance** line, derived from the PRD/plan, not invented.
- Numbering is `<phase>.<task>` (e.g. `1.1`, `1.2`, `2.1`). Phases group related tasks.
- The `Status:` line is the only one the orchestrators read for state. Permitted values: `TASKLIST_READY`, `IMPLEMENT_STEP_OK`, `REVIEW_BLOCKED`, `REVIEW_NEEDS_FIXES` — see `_shared/status-markers.md`.
- Don't pre-mark tasks `[x]` based on existing code. If existing code already satisfies a requirement, the task is still pending until verified by a review pass.
