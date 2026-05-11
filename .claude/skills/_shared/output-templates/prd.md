# PRD Template

Produced by `analysis` at `docs/prd/$TICKET_ID.prd.md`.

```markdown
# <TICKET_ID>: <feature title>

**Status:** <DRAFT | PRD_READY>

## Context / Idea

<Verbatim copy of the user's prompt and `$ARGUMENTS`. If a DESCRIPTION_FILE was provided, paste its technical specifications here without paraphrasing.>

## Goals

- <user-facing or system-level goal 1>
- <goal 2>

## User Stories

- As a <persona>, I want <capability> so that <outcome>.

## Scenarios

1. **<scenario name>** — <preconditions> → <action> → <expected result>

## Metrics

- <measurable success indicator: e.g. "p95 latency < 200ms", "0 panics under load test", "all 12 phase tasks pass">

## Constraints

- <technical, compatibility, or scope constraint>

## Risks

- <risk> — *Mitigation:* <how it's addressed>

## Open Questions

- <question for the user> (resolve before status flips to `PRD_READY`)
```

## Rules

- If any "Open Questions" remain unresolved, status stays `DRAFT`.
- All technical specifications from a `DESCRIPTION_FILE` must appear verbatim — see `_shared/requirements-immutability.md`.
- Sections may be empty (`<none>`) but must not be omitted.
