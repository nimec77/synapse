---
description: "Update documentation based on ticket work"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep
model: inherit
---

Use the `tech-writer` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## Docs-update steps

1. Read the artifacts for ticket `$1`:
- `docs/prd/$1.prd.md`,
- `docs/plan/$1.md`,
- `docs/tasklist/$1.md`,
- `reports/qa/$1.md` (if any).
2. Based on these artifacts and the code diff:
- create <ticket>-summary.md (a summary of the work done and decisions made on the ticket),
- add an entry to `CHANGELOG.md` (a brief description of the changes).
3. Show the diff from the user documentation.
