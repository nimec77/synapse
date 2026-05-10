# QA Report Template

Produced by `qa` at `reports/qa/$TICKET_ID.md` (or `reports/qa/$RELEASE_ID.md` for releases).

```markdown
# <TICKET_ID> QA Report

## Scope

<Tickets covered. For a single ticket, name it. For a release, list every ticket in `docs/releases/$RELEASE_ID.md`.>

## Positive Scenarios

| # | Scenario | Steps | Expected | Result |
|---|---|---|---|---|
| 1 | <happy path> | <action> | <observable outcome> | ✅ pass / ⚠️ partial / ❌ fail |

## Negative & Edge Cases

| # | Scenario | Steps | Expected | Result |
|---|---|---|---|---|
| 1 | <invalid input / boundary> | <action> | <observable outcome> | ✅/⚠️/❌ |

## Coverage Split

- **Automated tests** (run by `cargo test`): <list test functions / files>
- **Manual checks** (operator runs): <list>

## Risk Zones

- <area> — <why it's risky, what to watch>

## Verdict

**<release | with reservations | do not release>**

<One-paragraph justification.>
```

## Rules

- For a release (ID prefix `R-`), generate one consolidated report covering every ticket from the release manifest.
- Verdict must be one of the three exact strings: `release`, `with reservations`, `do not release`.
- A `do not release` verdict requires a follow-up task or `REVIEW_BLOCKED` marker — name the gap explicitly.
