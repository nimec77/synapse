---
description: "Prepare a QA plan and report for a ticket or release"
argument-hint: "[ticket]"
allowed-tools: Read, Write, Glob, Grep
model: inherit
---

Use the `qa` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## QA steps

1. If the identifier starts with `R-`, consider it a release:
- Read `docs/releases/$1.md` and extract the list of tickets.
2. For the ticket or each ticket in the release, read:
- `docs/prd/<ticket>.prd.md`,
- `docs/plan/<ticket>.md`,
- `docs/tasklist/<ticket>.md`.
3. Generate reports/qa/$1.md:
- positive scenarios,
- negative and edge cases,
- division into automated tests and manual checks,
- risk zone,
- final verdict (release / with reservations / do not release).
