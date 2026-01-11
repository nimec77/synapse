---
description: "Gather technical context and create a research document for the ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, rust-analyzer-lsp
model: inherit
---

Use the `researcher` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `doc/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in doc/.active_ticket" and terminate immediately.

## Research Steps

1. Read `doc/prd/$1.prd.md`, if the file exists.
2. Scan key project directories (src, doc, configs) for entities and modules related to the ticket.
3. Document the following in `doc/research/$1.md`:
- existing endpoints and contracts,
- layers and dependencies,
- patterns used,
- limitations and risks,
- open technical questions.
4. Do not change the code; only gather information.
