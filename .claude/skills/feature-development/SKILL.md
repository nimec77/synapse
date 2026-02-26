---
description: "End-to-end AI-driven feature workflow: PRD -> plan -> tasks -> implementation -> review -> QA -> docs"
argument-hint: "[ticket-id] [description-file]"
allowed-tools: Read, Write, Glob, Grep, Skill, Task, AskUserQuestion
model: sonnet
---

## Argument Parsing

Parse `$ARGUMENTS` to extract positional arguments:
- **TICKET_ID**: first whitespace-delimited token from `$ARGUMENTS`
- **DESCRIPTION_FILE**: second token (file path, strip leading `@` if present) from `$ARGUMENTS`

Use TICKET_ID wherever the ticket identifier is needed below. Do NOT use the raw `$1` value — it may be incorrect due to argument parsing issues.

---

You are the orchestrator of feature `TICKET_ID` ("$ARGUMENTS").

## EXECUTION CONTRACT

**You MUST execute every numbered step below in sequence. After every tool return (Task, AskUserQuestion, Skill, Glob), immediately proceed to the next step. The ONLY valid stopping point is the "WORKFLOW COMPLETE" marker at the end. Stopping before WORKFLOW COMPLETE is a contract violation. If any gate fails, stop and report the failure — that is the only other valid stop.**

---

### 1. Check current status

Use Glob to check which artifacts already exist for ticket TICKET_ID:
- PRD: `docs/prd/TICKET_ID.prd.md`
- Plan: `docs/plan/TICKET_ID.md`
- Tasklist: `docs/tasklist/TICKET_ID.md`
- QA Report: `reports/qa/TICKET_ID.md`
- Summary: `docs/summaries/TICKET_ID-summary.md`

**After Glob returns, execute step 2.**

---

### 2. Gate: PRD

Skip if `docs/prd/TICKET_ID.prd.md` exists. Otherwise:

**After this Task returns, execute step 3.**

- Task: `subagent_type: "general-purpose"`, `model: "opus"`, `description: "Create TICKET_ID PRD"`, `prompt: "Create a PRD for ticket TICKET_ID. Read '.claude/skills/analysis/SKILL.md' for instructions. Arguments: TICKET_ID DESCRIPTION_FILE"`

---

### 3. Checkpoint: PRD review

- AskUserQuestion:
  - question: "PRD has been created. Ready to proceed to research and planning phase?"
  - header: "PRD Done"
  - options:
    - Label: "Continue to Planning", Description: "Proceed to research the codebase and create implementation plan"
    - Label: "Pause to review", Description: "Pause to review before continuing"
- If "Pause to review": loop AskUserQuestion (question: "Take your time reviewing the PRD. Select 'Continue' when ready.", header: "Paused", options: "Continue" / "Still reviewing") until user selects "Continue".

**Checkpoint resolved. Execute step 4.**

---

### 4. Gate: Research

Skip if `docs/plan/TICKET_ID.md` exists (research feeds into the plan). Otherwise:

**After this Task returns, execute step 5.**

- Task: `subagent_type: "general-purpose"`, `model: "opus"`, `description: "Research TICKET_ID"`, `prompt: "Research the codebase for ticket TICKET_ID. Read '.claude/skills/research/SKILL.md' for instructions. Arguments: TICKET_ID"`

---

### 5. Gate: Plan

Skip if `docs/plan/TICKET_ID.md` exists. Otherwise:

**After this Task returns, execute step 6.**

- Task: `subagent_type: "general-purpose"`, `model: "opus"`, `description: "Plan TICKET_ID"`, `prompt: "Create an implementation plan for ticket TICKET_ID. Read '.claude/skills/plan/SKILL.md' for instructions. Arguments: TICKET_ID"`

---

### 6. Checkpoint: Plan review

- AskUserQuestion:
  - question: "Implementation plan has been created. Ready to proceed to task breakdown and implementation?"
  - header: "Plan Done"
  - options:
    - Label: "Continue to Implementation", Description: "Break down plan into tasks and start implementation"
    - Label: "Pause to review", Description: "Pause to review before continuing"
- If "Pause to review": loop AskUserQuestion (question: "Take your time reviewing the plan. Select 'Continue' when ready.", header: "Paused", options: "Continue" / "Still reviewing") until user selects "Continue".

**Checkpoint resolved. Execute step 7.**

---

### 7. Gate: Tasklist

Skip if `docs/tasklist/TICKET_ID.md` exists. Otherwise:

**After this Task returns, execute step 8.**

- Task: `subagent_type: "general-purpose"`, `model: "sonnet"`, `description: "Create TICKET_ID tasklist"`, `prompt: "Create a tasklist for ticket TICKET_ID. Read '.claude/skills/tasklist/SKILL.md' for instructions. Arguments: TICKET_ID"`

---

### 8. Gate: Implementation

Skip if tasklist `docs/tasklist/TICKET_ID.md` contains no unchecked items (`- [ ]`). Otherwise:

**After this Task returns, execute step 9.**

- Task: `subagent_type: "general-purpose"`, `model: "sonnet"`, `description: "Implement TICKET_ID tasks"`, `prompt: "Execute the implement-orchestrated workflow for ticket TICKET_ID. Read '.claude/skills/implement-orchestrated/SKILL.md' for instructions. Arguments: TICKET_ID --auto"`

---

### 9. Checkpoint: Implementation review

Skip if currently in a review loop (returning from step 10). Otherwise:

- AskUserQuestion:
  - question: "Implementation is complete. Ready to proceed to code review and QA?"
  - header: "Code Done"
  - options:
    - Label: "Continue to Review", Description: "Proceed to code review, QA, and documentation"
    - Label: "Pause to review", Description: "Pause to review before continuing"
- If "Pause to review": loop AskUserQuestion (question: "Take your time reviewing the implementation. Select 'Continue' when ready.", header: "Paused", options: "Continue" / "Still reviewing") until user selects "Continue".

**Checkpoint resolved. Execute step 10.**

---

### 10. Gate: Review

**After this Task returns, parse the reviewer output and decide: loop or continue.**

- Task: `subagent_type: "general-purpose"`, `model: "opus"`, `description: "Review TICKET_ID changes"`, `prompt: "Review changes for ticket TICKET_ID. Read '.claude/skills/run-reviewer/SKILL.md' for instructions. Arguments: TICKET_ID"`

**Decision after Task returns:**
- If `REVIEW_BLOCKED` or `REVIEW_NEEDS_FIXES`: read tasklist to confirm new unchecked tasks exist, increment review loop counter, and **go back to step 8**. If loop count exceeds 3, stop with: "Review loop exceeded 3 iterations. Manual intervention required."
- If `REVIEW_OK`: clear review loop state and **execute step 11**.

---

### 11. Gate: QA

Skip if `reports/qa/TICKET_ID.md` exists. Otherwise:

**After this Task returns, execute step 12.**

- Task: `subagent_type: "general-purpose"`, `model: "sonnet"`, `description: "QA for TICKET_ID"`, `prompt: "Generate QA plan and report for ticket TICKET_ID. Read '.claude/skills/qa/SKILL.md' for instructions. Arguments: TICKET_ID"`

---

### 12. Gate: Documentation

Skip if `docs/summaries/TICKET_ID-summary.md` exists. Otherwise:

**After this Task returns, execute step 13.**

- Task: `subagent_type: "general-purpose"`, `model: "sonnet"`, `description: "Update docs for TICKET_ID"`, `prompt: "Update documentation for ticket TICKET_ID. Read '.claude/skills/docs-update/SKILL.md' for instructions. Arguments: TICKET_ID"`

---

### 13. Validate

**After this Skill returns, execute step 14.**

- Skill: `skill: "validate"`, `args: "TICKET_ID"`

---

### 14. Sync description file

Skip if DESCRIPTION_FILE is empty, not provided, or the file does not exist.

1. Read the description file at path DESCRIPTION_FILE.
2. Check if it's a phase file (name matches `phase-*.md` or `phase-[0-9]*.md`) or contains checkbox tasks (`- [ ]` or `- [x]`).
3. If it contains tasks:
   - Read completed tasklist from `docs/tasklist/TICKET_ID.md`.
   - Compare tasks in description file against actually completed tasks.
   - If discrepancies found (tasks in description file NOT completed): list mismatches, display error, and stop.
   - If all tasks match: update description file changing all `- [ ]` to `- [x]`, report count.
4. Skip if file contains no checkbox tasks.

**After sync completes, execute step 15.**

---

### 15. Update CLAUDE.md

**After this Skill returns, execute step 16.**

- Skill: `skill: "init"`

---

### 16. Commit pending changes

- Skill: `skill: "commit"`

**After this Skill returns, execute step 17.**

---

### 17. Release

- Skill: `skill: "release"`, `args: "minor"`

**After this Skill returns, you have reached the end.**

---

### WORKFLOW COMPLETE

All gates have passed. Report final status to the user: feature `TICKET_ID` is complete. List which gates were executed and which were skipped, including commit and release.
