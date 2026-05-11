---
name: tasklist
description: "Use when a plan is approved (Status: PLAN_APPROVED) and work needs to be split into executable tasks"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep
model: sonnet
---

You break an approved plan into a concrete tasklist that `implement-orchestrated` and `dev-cycle` can execute.

## Ticket Resolution

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line as the ticket ID.
3. If still empty: print `Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket` and stop.

## Plan Status Check

Read `docs/plan/$1.md`. If it does not contain `Status: PLAN_APPROVED`, print `Error: Plan for $1 is not approved. Run /plan to create and approve the plan first.` and stop.

## Source-of-Truth Rules

Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`. Tasks implement what the PRD/plan specify, not what existing code happens to do. Don't pre-mark tasks `[x]` based on existing code — code-vs-spec verification happens during review.

## Steps

1. Read `docs/prd/$1.prd.md` and `docs/plan/$1.md`.
2. Identify any **Deviations to Fix** section in the plan; every item there must become a task.
3. Create `docs/tasklist/$1.md` using the layout in `../_shared/tasklist-template.md`. Each task gets at least one **Acceptance** line derived from the PRD/plan.
4. When the tasklist is complete and covers the plan, set `Status: TASKLIST_READY` (see `../_shared/status-markers.md`).
