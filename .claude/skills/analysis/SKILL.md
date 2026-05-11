---
name: analysis
description: "Use when starting work on a new feature ticket and no PRD exists yet"
argument-hint: "[ticket-id] [description-file]"
allowed-tools: Read, Write, Glob, Grep
model: opus
---

You produce the initial PRD for a new ticket.

## Argument Parsing

Parse `$ARGUMENTS`:
- **TICKET_ID**: first whitespace-delimited token.
- **DESCRIPTION_FILE**: second token (file path; strip leading `@` if present).

Use `TICKET_ID` everywhere below — never the raw `$1`, which can be mis-parsed.

## Source-of-Truth Rules

The DESCRIPTION_FILE (e.g. `docs/phase/phase-N.md`) is **authoritative**: copy its technical specifications verbatim, never paraphrase. Apply the requirements-immutability rules in full — see `../_shared/requirements-immutability.md`.

## Steps

1. Set `docs/.active_ticket` to `TICKET_ID`.
2. If `docs/prd/TICKET_ID.prd.md` does not exist, create it using the layout in `../_shared/output-templates/prd.md`. If a project-local `docs/prd.template.md` exists, prefer it as the layout source.
3. Paste `$ARGUMENTS` into the PRD's "Context / Idea" section.
4. If `DESCRIPTION_FILE` is present and exists, read it and incorporate its content into "Context / Idea". **Copy all technical specifications exactly as written.**
5. Fill the standard sections (goals, user stories, scenarios, metrics, constraints, risks, open questions) from the description file and repository context.
6. **Verify**: every specification from the description file appears in the PRD without modification.
7. If anything is missing, formulate "Open Questions" for the user and set `Status: DRAFT`.
8. Otherwise set `Status: PRD_READY` (see `../_shared/status-markers.md`).
