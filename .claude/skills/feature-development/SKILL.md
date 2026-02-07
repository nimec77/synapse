---
description: "End-to-end AI-driven feature workflow: PRD -> plan -> tasks -> implementation -> review -> QA -> docs"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep, Skill, Task, AskUserQuestion
model: inherit
---

You are the orchestrator of feature `$1` ("$ARGUMENTS").

## EXECUTION CONTRACT

**You MUST execute every numbered step below in sequence. After every tool return (Task, AskUserQuestion, Skill, Glob), immediately proceed to the next step. The ONLY valid stopping point is the "WORKFLOW COMPLETE" marker at the end. Stopping before WORKFLOW COMPLETE is a contract violation. If any gate fails, stop and report the failure — that is the only other valid stop.**

---

### 1. Check current status

Use Glob to check which artifacts already exist for ticket `$1`:
- PRD: `docs/prd/$1.prd.md`
- Plan: `docs/plan/$1.md`
- Tasklist: `docs/tasklist/$1.md`
- QA Report: `reports/qa/$1.md`
- Summary: `docs/summaries/$1-summary.md`

**After Glob returns, execute step 2.**

---

### 2. Gate: PRD

Skip if `docs/prd/$1.prd.md` exists. Otherwise:

**After this Task returns, execute step 3.**

- Task: `subagent_type: "general-purpose"`, `description: "Create $1 PRD"`, `prompt: "Create a PRD for ticket $1. Read '.claude/skills/analysis/SKILL.md' for instructions. Arguments: $1 $2 $3"`

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

Skip if `docs/plan/$1.md` exists (research feeds into the plan). Otherwise:

**After this Task returns, execute step 5.**

- Task: `subagent_type: "general-purpose"`, `description: "Research $1"`, `prompt: "Research the codebase for ticket $1. Read '.claude/skills/research/SKILL.md' for instructions. Arguments: $1"`

---

### 5. Gate: Plan

Skip if `docs/plan/$1.md` exists. Otherwise:

**After this Task returns, execute step 6.**

- Task: `subagent_type: "general-purpose"`, `description: "Plan $1"`, `prompt: "Create an implementation plan for ticket $1. Read '.claude/skills/plan/SKILL.md' for instructions. Arguments: $1"`

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

Skip if `docs/tasklist/$1.md` exists. Otherwise:

**After this Task returns, execute step 8.**

- Task: `subagent_type: "general-purpose"`, `description: "Create $1 tasklist"`, `prompt: "Create a tasklist for ticket $1. Read '.claude/skills/tasklist/SKILL.md' for instructions. Arguments: $1"`

---

### 8. Gate: Implementation

Skip if tasklist `docs/tasklist/$1.md` contains no unchecked items (`- [ ]`). Otherwise:

**After this Task returns, execute step 9.**

- Task: `subagent_type: "general-purpose"`, `description: "Implement $1 tasks"`, `prompt: "Execute the implement-orchestrated workflow for ticket $1. Read '.claude/skills/implement-orchestrated/SKILL.md' for instructions. Arguments: $1 --auto"`

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

- Task: `subagent_type: "general-purpose"`, `description: "Review $1 changes"`, `prompt: "Review changes for ticket $1. Read '.claude/skills/run-reviewer/SKILL.md' for instructions. Arguments: $1"`

**Decision after Task returns:**
- If `REVIEW_BLOCKED` or `REVIEW_NEEDS_FIXES`: read tasklist to confirm new unchecked tasks exist, increment review loop counter, and **go back to step 8**. If loop count exceeds 3, stop with: "Review loop exceeded 3 iterations. Manual intervention required."
- If `REVIEW_OK`: clear review loop state and **execute step 11**.

---

### 11. Gate: QA

Skip if `reports/qa/$1.md` exists. Otherwise:

**After this Task returns, execute step 12.**

- Task: `subagent_type: "general-purpose"`, `description: "QA for $1"`, `prompt: "Generate QA plan and report for ticket $1. Read '.claude/skills/qa/SKILL.md' for instructions. Arguments: $1"`

---

### 12. Gate: Documentation

Skip if `docs/summaries/$1-summary.md` exists. Otherwise:

**After this Task returns, execute step 13.**

- Task: `subagent_type: "general-purpose"`, `description: "Update docs for $1"`, `prompt: "Update documentation for ticket $1. Read '.claude/skills/docs-update/SKILL.md' for instructions. Arguments: $1"`

---

### 13. Validate

**After this Skill returns, execute step 14.**

- Skill: `skill: "validate"`, `args: "$1"`

---

### 14. Sync description file

Skip if `$3` is empty, not provided, or the file does not exist.

1. Read the description file at path `$3`.
2. Check if it's a phase file (name matches `phase-*.md` or `phase-[0-9]*.md`) or contains checkbox tasks (`- [ ]` or `- [x]`).
3. If it contains tasks:
   - Read completed tasklist from `docs/tasklist/$1.md`.
   - Compare tasks in description file against actually completed tasks.
   - If discrepancies found (tasks in description file NOT completed): list mismatches, display error, and stop.
   - If all tasks match: update description file changing all `- [ ]` to `- [x]`, report count.
4. Skip if file contains no checkbox tasks.

**After sync completes, execute step 15.**

---

### 15. Update CLAUDE.md

**After this Skill returns, you have reached the end.**

- Skill: `skill: "init"`

---

### WORKFLOW COMPLETE

All gates have passed. Report final status to the user: feature `$1` is complete. List which gates were executed and which were skipped.
