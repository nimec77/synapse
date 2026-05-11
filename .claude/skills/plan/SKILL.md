---
name: plan
description: "Use when a PRD is approved and architecture or implementation decisions are needed before tasks are broken down"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, rust-analyzer-lsp, AskUserQuestion
model: opus
---

You produce an implementation plan for a ticket whose PRD is `PRD_READY`.

## Ticket Resolution

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line as the ticket ID.
3. If still empty: print `Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket` and stop.

## Source-of-Truth Rules

Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`. The PRD, phase docs, and `docs/vision.md` are immutable; the plan implements them as written. If existing code deviates, the plan adds tasks to fix the code.

## Steps

1. Read:
   - `docs/prd/$1.prd.md`,
   - `docs/research/$1.md` (if it exists),
   - `docs/conventions.md`,
   - `CLAUDE.md`.
2. Verify the PRD's `Status:` is `PRD_READY` or `PRD_APPROVED`. If not, print `Error: PRD for $1 is not ready. Run /analysis first.` and stop.
3. **Verify alignment**: confirm the research doc (if present) flagged any deviations from requirements.
4. Create or update `docs/plan/$1.md` using the layout in `../_shared/output-templates/plan.md`.
5. If there are architectural alternatives worth recording, write `docs/adr/$1.md` listing the options and the chosen one.
6. The plan must implement every PRD requirement; if existing code deviates, list each deviation under the **Deviations to Fix** section so the tasklist picks them up.
7. When the plan is complete and unambiguous, set `Status: PLAN_APPROVED` in `docs/plan/$1.md` (see `../_shared/status-markers.md`).
