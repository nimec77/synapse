---
name: qa
description: "Use when a ticket or release needs a QA plan and report"
argument-hint: "[ticket-id-or-release-id]"
allowed-tools: Read, Write, Glob, Grep
model: sonnet
---

You generate a QA report for a ticket (one report) or a release (one consolidated report covering every ticket in the release manifest).

## Argument Triage

`$1` is either a ticket ID (e.g. `SY-12`) or a release ID (prefix `R-`, e.g. `R-2026.05`).

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line.
3. If still empty: print `Error: No ticket specified. Provide a ticket or release ID as a parameter or set it in docs/.active_ticket` and stop.

## Steps

1. **If `$1` starts with `R-`:** read `docs/releases/$1.md` and extract the list of tickets it covers. Otherwise treat `$1` as a single ticket ID.
2. For the ticket (or each ticket in the release), read:
   - `docs/prd/<ticket>.prd.md`,
   - `docs/plan/<ticket>.md`,
   - `docs/tasklist/<ticket>.md`.
3. Generate `reports/qa/$1.md` using the layout in `../_shared/output-templates/qa-report.md`. Include:
   - positive scenarios,
   - negative and edge cases,
   - split between automated tests and manual checks,
   - risk zones,
   - final verdict — exactly one of `release`, `with reservations`, `do not release`.
4. If the verdict is `do not release`, name the gap explicitly so a follow-up ticket or `REVIEW_BLOCKED` task can be opened.
