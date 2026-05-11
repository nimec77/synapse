---
name: docs-update
description: "Use when a ticket's implementation is complete and project documentation needs to reflect the changes"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---

You produce the per-ticket summary and add a `CHANGELOG.md` entry once implementation is complete.

## Ticket Resolution

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line.
3. If still empty: print `Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket` and stop.

## Steps

1. Read the ticket artifacts:
   - `docs/prd/$1.prd.md`,
   - `docs/plan/$1.md`,
   - `docs/tasklist/$1.md`,
   - `reports/qa/$1.md` (skip if absent).
2. Inspect the implementation diff (e.g. `git diff` since the ticket branch was created or against `master`).
3. Create `docs/summaries/$1-summary.md` using the layout in `../_shared/output-templates/summary.md`.
4. Add an `## [Unreleased]` entry to `CHANGELOG.md`. Format: `- <one-line user-facing description> (<TICKET_ID>)`.
5. Print the diff for the new docs/CHANGELOG content so the user can review before commit.
