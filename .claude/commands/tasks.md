---
description: "Break down the plan for the ticket into a list of small tasks (tasklist)"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep
model: inherit
---

Use the `task-planner` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## Plan Status Check

Before proceeding, read `docs/plan/$1.md` and verify it contains `Status: PLAN_APPROVED`.
If the status is not `PLAN_APPROVED`, display an error message: "Error: Plan for ticket $1 is not approved. Run /plan to create and approve the plan first." and terminate immediately.

## Tasks Steps

1. Read:
- `docs/prd/$1.prd.md`,
- `docs/plan/$1.md`.
2. Create `docs/tasklist/$1.md`:
- title and brief context,
- a list of tasks with `- [ ]`,
- for each task, 1-2 acceptance criteria.
3. If the tasklist looks complete and covers the plan, set `Status: TASKLIST_READY`.
