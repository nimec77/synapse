---
description: "Review changes for a ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Glob, Grep, rust-analyzer-lsp
model: inherit
---

Use the `reviewer` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## Run-reviewer Steps

1. Read:
- `docs/prd/$1.prd.md`,
- `docs/plan/$1.md`,
- `docs/tasklist/$1.md`,
- `conventions.md`.
2. Analyze the diff for the changes related to ticket `$1`.
3. Generate a review:
- blocking notes (what needs to be fixed before merging),
- important (recommended fixes),
- etc. (cosmetics).
4. If you see unclosed scenarios or debts, suggest adding tasks to `docs/tasklist/$1.md` (but do not edit the file itself without a separate command).
