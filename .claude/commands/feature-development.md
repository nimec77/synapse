---
description: "End-to-end AI-driven feature workflow: PRD -> plan -> tasks -> implementation -> review -> QA -> docs"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Glob, SlashCommand
model: inherit
---

You are the orchestrator of feature `$1` ("$ARGUMENTS").

2. Call `/validate $1` and briefly describe the gate status.
3. If there is no PRD_READY, suggest running `/analysis $1 $2 $3`.
4. If there is no PLAN_APPROVED, suggest `/researcher $1` and `/plan $1`.
5. If there is no TASKLIST_READY, suggest `/tasks $1`.
6. If the implementation isn't complete, remind them to run `/implement $1` and `/run-reviewer $1`.
7. If the release isn't yet RELEASE_READY, remind them to run `/qa $1` or `/qa <release-id>`.
8. If DOCS_UPDATED hasn't been reached, suggest running `/docs-update $1`.

Don't change anything in the code or files—just describe the status and next steps.

