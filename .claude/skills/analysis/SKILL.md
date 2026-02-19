---
description: "Initialize feature: create a ticket and draft the PRD"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep
model: inherit
---

## Argument Parsing

Parse `$ARGUMENTS` to extract positional arguments:
- **TICKET_ID**: first whitespace-delimited token from `$ARGUMENTS`
- **SHORT_TITLE**: quoted string (if present) from `$ARGUMENTS`
- **DESCRIPTION_FILE**: remaining token (file path, strip leading `@` if present) from `$ARGUMENTS`

Use TICKET_ID wherever the ticket identifier is needed below. Do NOT use the raw `$1` value — it may be incorrect due to argument parsing issues.

---

Use the `analyst` subagent.

You are starting the process of working on a feature with the identifier `TICKET_ID`.

## CRITICAL: DESCRIPTION FILES ARE AUTHORITATIVE

**The description file (e.g., `docs/phase/phase-*.md`) contains the authoritative requirements.**

When a description file is provided:
- Its specifications are the SOURCE OF TRUTH
- Do NOT modify requirements based on existing code
- Do NOT contradict the description file
- Copy technical specifications EXACTLY as written

### Forbidden Actions

- Changing specifications from the description file based on existing code
- Saying "implementation uses X" to override specified requirement Y
- Omitting requirements from the description file
- Reinterpreting technical specifications (e.g., changing "UUID v8" to "UUID v4")

### Required Actions

- Copy all technical specifications from description file verbatim
- If existing code differs from requirements, note this as a gap to be fixed
- All requirements from the description file must appear in the PRD

---

## Steps

1. Update `docs/.active_ticket` with the value `TICKET_ID`.
2. If the file `docs/prd/TICKET_ID.prd.md` does not exist, create it from the template `@docs/prd.template.md`.
3. Transfer `$ARGUMENTS` to the "Context / Idea" section.
4. If a third argument (description file path) is provided, read the file and incorporate its content into the PRD as additional context in the "Context / Idea" section. **Copy all technical specifications exactly as written.**
5. Create the following sections: goals, user stories, scenarios, metrics, constraints, risks, open questions.
6. Fill in what can be derived from the repository context and the description file (if provided).
7. **Verify**: All specifications from the description file are present in the PRD without modification.
8. If there is insufficient data, formulate questions for the team and set `Status: DRAFT`.
9. If the PRD looks complete and without blocking questions, set `Status: PRD_READY`.
