---
description: "Create the architecture and implementation plan for the ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, rust-analyzer-lsp, AskUserQuestion
model: inherit
---

Use the `planner` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## Planner steps

1. Read:
- `docs/prd/$1.prd.md`,
- `docs/research/$1.md` (if available),
- `docs/conventions.md`.
2. Create or update `docs/plan/$1.md` with the following structure:
- Components
- API contract
- Data flows
- NFR
- Risks
- Open questions (if any).
3. If there are architectural alternatives, create `docs/adr/$1.md` with the options and the chosen solution.
4. If the plan is approved, set the line `Status: PLAN_APPROVED` in `docs/plan/$1.md`.
