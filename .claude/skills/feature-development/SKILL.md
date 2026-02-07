---
description: "End-to-end AI-driven feature workflow: PRD -> plan -> tasks -> implementation -> review -> QA -> docs"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep, Skill, Task, AskUserQuestion
model: inherit
---

You are the orchestrator of feature `$1` ("$ARGUMENTS").

## Workflow

Execute the feature development workflow by running each gate in sequence until all gates pass. After every Skill or Task tool returns, immediately continue to the next numbered step — never stop between steps.

### Step 1: Check Current Status

Check which artifacts exist for ticket `$1`:
- PRD: `docs/prd/$1.prd.md`
- Plan: `docs/plan/$1.md`
- Tasklist: `docs/tasklist/$1.md`
- QA Report: `reports/qa/$1.md`
- Summary: `docs/summaries/$1-summary.md`

### Step 2: Execute Missing Gates

Execute each missing gate in order. **You MUST use the Skill or Task tool** as specified in each gate - do NOT perform the actions directly.

#### Gate 1: PRD_READY
- **Condition**: PRD file `docs/prd/$1.prd.md` does not exist
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Skill tool: `skill: "analysis"`, `args: "$1 $2 $3"`
  2. Invoke AskUserQuestion tool:
     - question: "PRD has been created. Ready to proceed to research and planning phase?"
     - header: "PRD Done"
     - options:
       - Label: "Continue to Planning", Description: "Proceed to research the codebase and create implementation plan"
       - Label: "Pause to review", Description: "Pause to review before continuing"
  3. If user selects "Pause to review" → invoke AskUserQuestion again:
     - question: "Take your time reviewing the PRD. Select 'Continue' when ready."
     - header: "Paused"
     - options: "Continue" / "Still reviewing"
     - Loop until user selects "Continue"
  4. Proceed to Gate 2

#### Gate 2: PLAN_APPROVED
- **Condition**: Plan file `docs/plan/$1.md` does not exist
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Skill tool: `skill: "research"`, `args: "$1"`
  2. Invoke Skill tool: `skill: "plan"`, `args: "$1"`
  3. Invoke AskUserQuestion tool:
     - question: "Implementation plan has been created. Ready to proceed to task breakdown and implementation?"
     - header: "Plan Done"
     - options:
       - Label: "Continue to Implementation", Description: "Break down plan into tasks and start implementation"
       - Label: "Pause to review", Description: "Pause to review before continuing"
  4. If user selects "Pause to review" → invoke AskUserQuestion again:
     - question: "Take your time reviewing the plan. Select 'Continue' when ready."
     - header: "Paused"
     - options: "Continue" / "Still reviewing"
     - Loop until user selects "Continue"
  5. Proceed to Gate 3

#### Gate 3: TASKLIST_READY
- **Condition**: Tasklist file `docs/tasklist/$1.md` does not exist
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Skill tool: `skill: "tasklist"`, `args: "$1"`
  2. Proceed to Gate 4

#### Gate 4: IMPLEMENT_STEP_OK
- **Condition**: Tasklist contains incomplete tasks (unchecked items `- [ ]`)
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Task tool:
     - `subagent_type`: "general-purpose"
     - `prompt`: "Execute the implement-orchestrated workflow for ticket $1. Read `.claude/skills/implement-orchestrated/SKILL.md` for instructions. Arguments: $1 --auto"
     - `description`: "Implement $1 tasks"
  2. Check context — are you in a review loop (came from Gate 5 with `REVIEW_NEEDS_FIXES`)?
     - **If review loop**: skip to step 5 (return to Gate 5)
     - **If initial implementation**: continue to step 3
  3. Invoke AskUserQuestion tool:
     - question: "Implementation is complete. Ready to proceed to code review and QA?"
     - header: "Code Done"
     - options:
       - Label: "Continue to Review", Description: "Proceed to code review, QA, and documentation"
       - Label: "Pause to review", Description: "Pause to review before continuing"
  4. If user selects "Pause to review" → invoke AskUserQuestion again:
     - question: "Take your time reviewing the implementation. Select 'Continue' when ready."
     - header: "Paused"
     - options: "Continue" / "Still reviewing"
     - Loop until user selects "Continue"
  5. Proceed to Gate 5

#### Gate 5: REVIEW_OK
- **Condition**: Implementation gate passed
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Task tool:
     - `subagent_type`: "reviewer"
     - `prompt`: "Review changes for ticket $1. Read `.claude/skills/run-reviewer/SKILL.md` for instructions. Arguments: $1"
     - `description`: "Review $1 changes"
  2. Parse the reviewer output for status:
     - **If `REVIEW_BLOCKED` or `REVIEW_NEEDS_FIXES`:**
       - Read tasklist to confirm new unchecked tasks exist
       - Mark that you are now in a **review loop**
       - Loop back to Gate 4 to implement review fixes
       - After Gate 4, return here for re-review
       - **Loop limit**: If review loop exceeds 3 iterations, terminate with: "Review loop exceeded 3 iterations. Manual intervention required."
     - **If `REVIEW_OK`:**
       - Clear the review loop marker
       - Proceed to Gate 6

#### Gate 6: RELEASE_READY
- **Condition**: QA Report file `reports/qa/$1.md` does not exist
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Task tool:
     - `subagent_type`: "qa"
     - `prompt`: "Generate QA plan and report for ticket $1. Read `.claude/skills/qa/SKILL.md` for instructions. Arguments: $1"
     - `description`: "QA for $1"
  2. Proceed to Gate 7

#### Gate 7: DOCS_UPDATED
- **Condition**: Summary file `docs/summaries/$1-summary.md` does not exist
- **Steps** (execute in order, do not stop between steps):
  1. Invoke Task tool:
     - `subagent_type`: "tech-writer"
     - `prompt`: "Update documentation for ticket $1. Read `.claude/skills/docs-update/SKILL.md` for instructions. Arguments: $1"
     - `description`: "Update docs for $1"
  2. Proceed to Step 3

### Step 3: Confirm Completion

After all gates pass, invoke Skill tool with `skill: "validate"` and `args: "$1"` to confirm the feature is complete.

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

### Step 5: Update CLAUDE.md

Invoke Skill tool with `skill: "init"` to regenerate the `CLAUDE.md` file with any new information from the completed iteration.

## Important

- Execute gates sequentially — each depends on the previous
- Every gate uses a numbered **Steps** list. Execute all steps in order. Do not stop after a Skill or Task tool returns — continue to the next numbered step.
- **Checkpoints**: Use AskUserQuestion at designated checkpoints (after PRD, Plan, Implementation) to let users pause or continue. Always use the structured AskUserQuestion tool — never just output text asking for confirmation.
- If user selects "Pause to review" at any checkpoint, re-prompt with AskUserQuestion until they select "Continue". Never terminate the workflow at intermediate checkpoints.
- If any gate fails, stop and report the issue
- Description file sync happens only after successful completion of all gates
- **Review loop**: Gate 5 can loop back to Gate 4 if fixes are requested. Track loop count to prevent infinite loops.
- **Task vs Skill**: Gates 4-7 use Task tool (which creates separate agent context) to ensure proper return to parent workflow. Gates 1-3 use Skill tool for simpler orchestration.

## Gate Flow Diagram

```
PRD -> Plan -> Tasklist -> Implementation -> Review
                              ^                |
                              |                v
                              +-- (if fixes) --+
                                               |
                                               v (if OK)
                                              QA -> Docs -> Validate
```
