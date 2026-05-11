# Plan Template

Produced by `plan` at `docs/plan/$TICKET_ID.md`.

```markdown
# <TICKET_ID> Plan

**Status:** PLAN_APPROVED

## Components

- **<component / module>** — <responsibility>; lives in `<crate/path>`

## API Contract

<Public types, traits, and functions added or changed. Include signatures.>

## Data Flows

1. <entry point> → <intermediate step> → <result>

## Non-Functional Requirements

- **Performance:** <target>
- **Error handling:** <which `thiserror` enum gets which variants>
- **Concurrency:** <sync vs async, locking expectations>
- **Testing:** <which tests are required: unit, integration, snapshot>

## Risks

- <risk> — *Mitigation:* <approach>

## Deviations to Fix

<Existing-code deviations from PRD requirements that must be corrected. Each becomes a task in the tasklist. Delete this section if none.>

## Open Questions

<Should be empty when status is `PLAN_APPROVED`. If any remain, status stays at draft.>
```

## Rules

- The plan must implement **every** PRD requirement; if existing code deviates, the plan must include tasks to fix it (`Deviations to Fix` section).
- API contract is binding — the tasklist and implementation must produce these exact signatures.
- For architectural alternatives, write a separate `docs/adr/$TICKET_ID.md` with the options and chosen solution.
- See `_shared/requirements-immutability.md` for what cannot change.
