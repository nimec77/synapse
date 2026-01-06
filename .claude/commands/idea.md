---
description: "Initialize feature: create a ticket and draft the PRD"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep
model: inherit
---

Use the `analyst` subagent.

You are starting the process of working on a feature with the identifier `$1`.

Steps:
1. Update `doc/.active_ticket` with the value `$1`.
2. If the file `doc/prd/$1.prd.md` does not exist, create it from the template `@doc/prd.template.md`.
3. Transfer `$ARGUMENTS` to the "Context / Idea" section.
4. If a third argument (description file path) is provided, read the file and incorporate its content into the PRD as additional context in the "Context / Idea" section.
5. Create the following sections: goals, user stories, scenarios, metrics, constraints, risks, open questions.
6. Fill in what can be derived from the repository context and the description file (if provided).
7. If there is insufficient data, formulate questions for the team and set `Status: DRAFT`.
8. If the PRD looks complete and without blocking questions, set `Status: PRD_READY`.
