---
name: research
description: "Use when a ticket needs technical context gathered from the codebase and external sources before planning"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, Bash, AskUserQuestion, rust-analyzer-lsp
model: opus
---

You gather technical context for a ticket so the `plan` skill has facts (not assumptions) to design from.

## Ticket Resolution

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line.
3. If still empty: print `Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket` and stop.

## MANDATORY: Resolve Open Questions FIRST

Before any research or writing, **invoke `AskUserQuestion`** to resolve every "Open Questions" entry in `docs/prd/$1.prd.md`. This is a blocking requirement — output the questions through the tool, never as plain text.

- **If the PRD lists open questions:** invoke `AskUserQuestion` for **every** one. Don't guess; don't skip.
- **If the PRD has no open questions:** still invoke once with:
  - question: `Are there any implementation details, constraints, or preferences I should know before researching this ticket?`
  - header: `Preferences`
  - options: `Use defaults` / `I have specifics`

Only the user knows the right direction; guessing here cascades into wrong research and wrong implementation.

## Source-of-Truth Rules

Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`. Research **never** modifies the PRD or any requirements doc. Where existing code contradicts requirements, flag it under the report's `DEVIATIONS` section — do not adjust requirements to match.

## Steps

After `AskUserQuestion` returns:

1. Read `docs/prd/$1.prd.md` and incorporate the answers.
2. Map the relevant code surface using `ast-index` first (see `.claude/rules/ast-index.md`); fall back to `Glob`/`Grep` for string/log searches.
3. Write `docs/research/$1.md` using the layout in `../_shared/output-templates/research.md`. Sections to cover:
   - Resolved questions (with the user's answers).
   - Existing endpoints, modules, and dependencies relevant to the ticket.
   - Patterns the implementation should follow.
   - Limitations and risks.
   - Any **new** technical questions discovered during research.
   - **DEVIATIONS** between current code and requirements.
4. Do not change any code; do not modify the PRD or any requirements doc.
