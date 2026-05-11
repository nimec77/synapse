---
name: validate
description: "Use when checking which quality gates a ticket or release has passed"
argument-hint: "[ticket-or-release-id]"
allowed-tools: Read, Glob, Grep
model: sonnet
---

You inspect the artifact files for a ticket (or every ticket in a release) and report which workflow gates have been passed. **Read-only — never modify any file.**

## Argument Triage

`$1` is either a ticket ID or a release ID (prefix `R-`).

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line.
3. If still empty: print `Error: No ticket specified. Provide a ticket or release ID as a parameter or set it in docs/.active_ticket` and stop.

## Gate Definitions

For each ticket, check the artifacts and report `PASS`/`FAIL` per gate. The status strings come from `../_shared/status-markers.md`.

| Gate | Passes when |
|---|---|
| `PRD_READY` | `docs/prd/<ticket>.prd.md` exists and contains `Status: PRD_READY` (or `PRD_APPROVED`). |
| `PLAN_APPROVED` | `docs/plan/<ticket>.md` exists and contains `Status: PLAN_APPROVED`. |
| `TASKLIST_READY` | `docs/tasklist/<ticket>.md` exists and contains `Status: TASKLIST_READY` (or any later status: `IMPLEMENT_STEP_OK`, `REVIEW_OK`). |
| `IMPLEMENT_STEP_OK` | `docs/tasklist/<ticket>.md` contains `Status: IMPLEMENT_STEP_OK` **and** every task is `[x]`. |
| `REVIEW_OK` | The most recent run of `run-reviewer` produced `REVIEW_OK` (no `## Review Fixes` section with unchecked tasks remains). |
| `DOCS_UPDATED` | `docs/summaries/<ticket>-summary.md` exists **and** `CHANGELOG.md` has an `## [Unreleased]` entry for this ticket. |
| `RELEASE_READY` (release only) | `docs/releases/<release>.md` exists and contains `Status: RELEASE_READY`, and every ticket it lists has `DOCS_UPDATED`. |

## Steps

1. **If `$1` starts with `R-`:** read `docs/releases/$1.md` and extract every ticket. Otherwise treat `$1` as one ticket.
2. For the ticket (or each release ticket), locate:
   - PRD: `docs/prd/<ticket>.prd.md`,
   - Plan: `docs/plan/<ticket>.md`,
   - Tasklist: `docs/tasklist/<ticket>.md`,
   - QA: `reports/qa/<ticket>.md` (if any),
   - Summary: `docs/summaries/<ticket>-summary.md` (if any).
3. Apply the table above. For each gate, record `PASS` / `FAIL` and (when failing) the reason.
4. Print a single table to the user:

   ```
   Ticket: <ticket>
   | Gate              | Status | Notes                                       |
   |-------------------|--------|---------------------------------------------|
   | PRD_READY         | PASS   |                                             |
   | PLAN_APPROVED     | PASS   |                                             |
   | TASKLIST_READY    | PASS   |                                             |
   | IMPLEMENT_STEP_OK | FAIL   | 2 tasks still `[ ]`                          |
   | REVIEW_OK         | —      | gated on IMPLEMENT_STEP_OK                  |
   | DOCS_UPDATED      | —      | gated on REVIEW_OK                          |
   ```

   For releases, repeat the table per ticket and add a final `RELEASE_READY` summary row.

5. **Do not modify any file.** This skill is purely diagnostic.
