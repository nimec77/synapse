---
description: "Check which quality gates have been passed for a ticket or release"
argument-hint: "[ticket-or-release-id]"
allowed-tools: Read, Glob, Grep
model: sonnet
---

Use the `validator` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## Validate steps

1. If `$1` starts with `R-`, consider it a release:
- Read `docs/releases/$1.md` and extract the associated tickets.
2. For the ticket or each release ticket, find the following artifacts:
- PRD: `docs/prd/<ticket>.prd.md`,
- Plan: `docs/plan/<ticket>.md`,
- Tasklist: `docs/tasklist/<ticket>.md`,
- QA: `reports/qa/<ticket>.*` (if any).
3. Using these files, evaluate the gates:
- PRD_READY,
- PLAN_APPROVED,
- TASKLIST_READY,
- IMPLEMENT_STEP_OK (for current tasks),
- REVIEW_OK,
- RELEASE_READY (for the release),
- DOCS_UPDATED.
4. Return a summary in text form:
- the status of each gate,
- what needs to be done if the gate is not passed. 5. Don't change anything in the files - just analyze them.
