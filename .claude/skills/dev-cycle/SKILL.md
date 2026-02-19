---
description: "Run implementation and code review cycle for an existing tasklist"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep, Skill, Task, AskUserQuestion
model: inherit
---

## Argument Parsing

Parse `$ARGUMENTS` to extract positional arguments:
- **TICKET_ID**: first whitespace-delimited token from `$ARGUMENTS`
- **SHORT_TITLE**: quoted string (if present) from `$ARGUMENTS`
- **DESCRIPTION_FILE**: remaining token (file path, strip leading `@` if present) from `$ARGUMENTS`

Use TICKET_ID wherever the ticket identifier is needed below. Do NOT use the raw `$1` value — it may be incorrect due to argument parsing issues.

---

You are the dev-cycle orchestrator for ticket `TICKET_ID` ("$ARGUMENTS").

## EXECUTION CONTRACT

**You MUST execute every numbered step below in sequence. After every tool return (Task, Skill, Glob, Read), immediately proceed to the next step. The ONLY valid stopping point is the "WORKFLOW COMPLETE" marker at the end. Stopping before WORKFLOW COMPLETE is a contract violation. If any gate fails, stop and report the failure — that is the only other valid stop. There are NO user checkpoints — this workflow is fully automatic.**

---

### 1. Ticket resolution

If TICKET_ID is empty or not provided:
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

**After ticket ID is resolved, execute step 2.**

---

### 2. Gate: PRD

Skip if `docs/prd/TICKET_ID.prd.md` exists. Otherwise:

- Task: `subagent_type: "general-purpose"`, `description: "Create TICKET_ID PRD"`, `prompt: "Create a PRD for ticket TICKET_ID. Read '.claude/skills/analysis/SKILL.md' for instructions. Arguments: TICKET_ID SHORT_TITLE DESCRIPTION_FILE"`

**After this Task returns, execute step 3.**

---

### 3. Gate: Research & Plan

Skip if `docs/plan/TICKET_ID.md` exists. Otherwise:

- Task: `subagent_type: "general-purpose"`, `description: "Research TICKET_ID"`, `prompt: "Research the codebase for ticket TICKET_ID. Read '.claude/skills/research/SKILL.md' for instructions. Arguments: TICKET_ID"`

After research Task returns:

- Task: `subagent_type: "general-purpose"`, `description: "Plan TICKET_ID"`, `prompt: "Create an implementation plan for ticket TICKET_ID. Read '.claude/skills/plan/SKILL.md' for instructions. Arguments: TICKET_ID"`

**After this Task returns, execute step 4.**

---

### 4. Gate: Tasklist

Skip if `docs/tasklist/TICKET_ID.md` exists. Otherwise:

- Task: `subagent_type: "general-purpose"`, `description: "Create TICKET_ID tasklist"`, `prompt: "Create a tasklist for ticket TICKET_ID. Read '.claude/skills/tasklist/SKILL.md' for instructions. Arguments: TICKET_ID"`

Read `docs/tasklist/TICKET_ID.md` and verify it contains at least one unchecked task (`- [ ]`). If no unchecked tasks remain, stop with error: "Error: No unchecked tasks in `docs/tasklist/TICKET_ID.md`. Nothing to implement."

**After gate passes, initialize review loop counter to 0 and execute step 5.**

---

### 5. Gate: Implementation

Read `docs/tasklist/TICKET_ID.md` and check for unchecked tasks (`- [ ]`).

**If no unchecked tasks remain:** Skip to step 6.

Otherwise:

- Task: `subagent_type: "general-purpose"`, `description: "Implement TICKET_ID tasks"`, `prompt: "Execute the implement-orchestrated workflow for ticket TICKET_ID. Read '.claude/skills/implement-orchestrated/SKILL.md' for instructions. Arguments: TICKET_ID --auto"`

**After this Task returns, execute step 6.**

---

### 6. Gate: Review

- Task: `subagent_type: "general-purpose"`, `description: "Review TICKET_ID changes"`, `prompt: "Review changes for ticket TICKET_ID. Read '.claude/skills/run-reviewer/SKILL.md' for instructions. Arguments: TICKET_ID"`

**After this Task returns, parse the reviewer output and decide: loop or continue.**

**Decision after Task returns:**
- If `REVIEW_OK`: clear review loop state and **execute step 8**.
- If `REVIEW_NEEDS_FIXES` or `REVIEW_BLOCKED`: **execute step 7**.

---

### 7. Review loop

1. Increment review loop counter.
2. If loop counter exceeds 3, stop with error: "Review loop exceeded 3 iterations. Manual intervention required."
3. Read `docs/tasklist/TICKET_ID.md` and verify that new unchecked tasks (`- [ ]`) exist (the reviewer should have added them in step 6).
4. **Go back to step 5.**

---

### 8. Gate: Documentation

Skip if `docs/summaries/TICKET_ID-summary.md` exists. Otherwise:

- Task: `subagent_type: "general-purpose"`, `description: "Update docs for TICKET_ID"`, `prompt: "Update documentation for ticket TICKET_ID. Read '.claude/skills/docs-update/SKILL.md' for instructions. Arguments: TICKET_ID"`

**After this Task returns, execute step 9.**

---

### 9. Validate

- Skill: `skill: "validate"`, `args: "TICKET_ID"`

**After this Skill returns, execute step 10.**

---

### 10. Sync description file

Skip if DESCRIPTION_FILE is empty, not provided, or the file does not exist.

1. Read the description file at path DESCRIPTION_FILE.
2. Check if it contains checkbox tasks (`- [ ]` or `- [x]`).
3. If it contains tasks:
   - Read completed tasklist from `docs/tasklist/TICKET_ID.md`.
   - Compare tasks in description file against actually completed tasks.
   - If discrepancies found (tasks in description file NOT completed): list mismatches, display error, and stop.
   - If all tasks match: update description file changing all `- [ ]` to `- [x]`, report count.
4. Skip if file contains no checkbox tasks.

**After sync completes, execute step 11.**

---

### 11. WORKFLOW COMPLETE

All gates have passed. Report final status to the user:
- Feature `TICKET_ID` dev-cycle is complete.
- List which gates were executed: PRD, research & plan, tasklist, implementation, review (with iteration count), documentation, validate.
- Report any artifacts that were created during steps 2–4.
