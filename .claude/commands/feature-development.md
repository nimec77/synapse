---
description: "End-to-end AI-driven feature workflow: PRD -> plan -> tasks -> implementation -> review -> QA -> docs"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep, Skill
model: inherit
---

You are the orchestrator of feature `$1` ("$ARGUMENTS").

## Workflow

Execute the feature development workflow by running each gate in sequence until all gates pass.

### Step 1: Check Current Status

Check which artifacts exist for ticket `$1`:
- PRD: `docs/prd/$1.prd.md`
- Plan: `docs/plan/$1.md`
- Tasklist: `docs/tasklist/$1.md`
- QA Report: `reports/qa/$1.md`
- Release: `docs/releases/$1.md`

### Step 2: Execute Missing Gates

Run each missing gate in order. After each gate completes, proceed to the next.

1. **PRD_READY**: If no PRD exists, run `/analysis $1 $2 $3`
2. **PLAN_APPROVED**: If no plan exists, run `/researcher $1`, then `/planner $1`
3. **TASKLIST_READY**: If no tasklist exists, run `/tasks $1`
4. **IMPLEMENT_STEP_OK**: If tasks are incomplete, run `/implement-orchestrated $1`
5. **REVIEW_OK**: After implementation, run `/run-reviewer $1`
6. **RELEASE_READY**: Run `/qa $1` to generate QA report
7. **DOCS_UPDATED**: Run `/docs-update $1` to finalize documentation

### Step 3: Confirm Completion

After all gates pass, run `/validate $1` to confirm the feature is complete.

### Step 4: Sync Description File

If a description file was provided (`$3` is not empty):

1. **Read the description file** at the path specified by `$3`

2. **Check if it's a phase file or contains tasks:**
   - File name matches pattern `phase-*.md` OR `phase-[0-9]*.md`
   - OR file contains checkbox tasks (`- [ ]` or `- [x]`)

3. **If it contains tasks, verify and sync:**
   - Read the completed tasklist from `docs/tasklist/$1.md`
   - Compare tasks in the description file against actually completed tasks
   - **If discrepancies found** (tasks in description file that were NOT completed):
     - List the mismatched tasks
     - Display error: "Error: The following tasks from the description file were not completed: [list]"
     - Terminate execution immediately
   - **If all tasks match:**
     - Update the description file: change all `- [ ]` to `- [x]`
     - Report: "Updated [filename]: marked N tasks as complete"

4. **Skip this step** if:
   - `$3` is empty or not provided
   - The file doesn't exist
   - The file contains no checkbox tasks

## Important

- Execute gates sequentially - each depends on the previous
- Wait for user confirmation between major phases (PRD, Plan, Implementation)
- If any gate fails, stop and report the issue
- Description file sync happens only after successful completion of all gates

