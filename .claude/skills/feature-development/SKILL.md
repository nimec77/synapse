---
description: "End-to-end AI-driven feature workflow: PRD -> plan -> tasks -> implementation -> review -> QA -> docs"
argument-hint: "[ticket-id] [short-title] [description-file]"
allowed-tools: Read, Write, Glob, Grep, Skill, Task, AskUserQuestion
model: inherit
---

You are the orchestrator of feature `$1` ("$ARGUMENTS").

## Workflow

Execute the feature development workflow by running each gate in sequence until all gates pass.

**CRITICAL: CONTINUOUS EXECUTION REQUIREMENT**
- After EVERY Skill or Task tool invocation completes, you MUST immediately continue to the next instruction (checkpoint or next gate)
- NEVER pause or wait for user input after a Skill/Task returns - the ONLY acceptable pause is via AskUserQuestion at designated checkpoints
- When a Skill/Task outputs a summary, acknowledge it briefly and IMMEDIATELY proceed to the checkpoint
- Do NOT treat Skill/Task completion as a stopping point - treat it as a trigger to continue

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
- **Action**: Invoke Skill tool with `skill: "analysis"` and `args: "$1 $2 $3"`
- **On Skill Return**: When the skill completes, IMMEDIATELY proceed to the checkpoint below.
- **Checkpoint**: IMMEDIATELY use AskUserQuestion:
  - Question: "PRD has been created. Ready to proceed to research and planning phase?"
  - Header: "PRD Done"
  - Options:
    - Label: "Continue to Planning", Description: "Proceed to research the codebase and create implementation plan"
    - Label: "Pause to review", Description: "Pause to review before continuing"
  - If user selects "Pause to review": Use AskUserQuestion again:
    - Question: "Take your time reviewing the PRD. Select 'Continue' when ready."
    - Header: "Paused"
    - Options: "Continue" (proceed to planning) / "Still reviewing" (re-prompt this question)
    - Loop until user selects "Continue", then proceed to Gate 2.

#### Gate 2: PLAN_APPROVED
- **Condition**: Plan file `docs/plan/$1.md` does not exist
- **Action**:
  1. Invoke Skill tool with `skill: "research"` and `args: "$1"`
  2. When research completes, IMMEDIATELY invoke Skill tool with `skill: "plan"` and `args: "$1"`
- **On Skill Return**: When the plan skill completes, IMMEDIATELY proceed to the checkpoint below.
- **Checkpoint**: IMMEDIATELY use AskUserQuestion:
  - Question: "Implementation plan has been created. Ready to proceed to task breakdown and implementation?"
  - Header: "Plan Done"
  - Options:
    - Label: "Continue to Implementation", Description: "Break down plan into tasks and start implementation"
    - Label: "Pause to review", Description: "Pause to review before continuing"
  - If user selects "Pause to review": Use AskUserQuestion again:
    - Question: "Take your time reviewing the plan. Select 'Continue' when ready."
    - Header: "Paused"
    - Options: "Continue" (proceed to tasklist and implementation) / "Still reviewing" (re-prompt this question)
    - Loop until user selects "Continue", then proceed to Gate 3.

#### Gate 3: TASKLIST_READY
- **Condition**: Tasklist file `docs/tasklist/$1.md` does not exist
- **Action**: Invoke Skill tool with `skill: "tasklist"` and `args: "$1"`
- **On Skill Return**: When the skill completes, IMMEDIATELY proceed to Gate 4.

#### Gate 4: IMPLEMENT_STEP_OK
- **Condition**: Tasklist contains incomplete tasks (unchecked items `- [ ]`)
- **Action**: Invoke Task tool with:
  - `subagent_type`: "general-purpose"
  - `prompt`: "Execute the implement-orchestrated workflow for ticket $1. Read `.claude/skills/implement-orchestrated/SKILL.md` for instructions. Arguments: $1 --auto"
  - `description`: "Implement $1 tasks"
- **On Task Return**: When the task agent completes, IMMEDIATELY proceed based on context:
  - **If this is initial implementation** (not in a review loop): proceed to the checkpoint below
  - **If this is a review loop** (came from Gate 5 with `REVIEW_NEEDS_FIXES`): skip checkpoint, return directly to Gate 5 for re-review
- **Checkpoint** (initial implementation only): IMMEDIATELY use AskUserQuestion:
  - Question: "Implementation is complete. Ready to proceed to code review and QA?"
  - Header: "Code Done"
  - Options:
    - Label: "Continue to Review", Description: "Proceed to code review, QA, and documentation"
    - Label: "Pause to review", Description: "Pause to review before continuing"
  - If user selects "Pause to review": Use AskUserQuestion again:
    - Question: "Take your time reviewing the implementation. Select 'Continue' when ready."
    - Header: "Paused"
    - Options: "Continue" (proceed to review and QA) / "Still reviewing" (re-prompt this question)
    - Loop until user selects "Continue", then proceed to Gate 5.

#### Gate 5: REVIEW_OK
- **Condition**: Implementation gate passed
- **Action**: Invoke Task tool with:
  - `subagent_type`: "reviewer"
  - `prompt`: "Review changes for ticket $1. Read `.claude/skills/run-reviewer/SKILL.md` for instructions. Arguments: $1"
  - `description`: "Review $1 changes"
- **On Task Return**: When the reviewer task completes, parse the output for status and IMMEDIATELY proceed:

  1. **If `REVIEW_BLOCKED` or `REVIEW_NEEDS_FIXES`:**
     - The reviewer has added new tasks to `docs/tasklist/$1.md`
     - Read the tasklist to confirm new unchecked tasks exist (`- [ ]`)
     - Mark that you are now in a **review loop**
     - **Loop back to Gate 4** to implement the review fixes
     - After Gate 4 Task returns, return here (Gate 5) for re-review
     - This loop continues until review returns `REVIEW_OK`

  2. **If `REVIEW_OK`:**
     - Clear the review loop marker
     - Proceed to Gate 6

  **Loop limit**: If the review loop runs more than 3 times, terminate with message: "Review loop exceeded 3 iterations. Manual intervention required."

#### Gate 6: RELEASE_READY
- **Condition**: QA Report file `reports/qa/$1.md` does not exist
- **Action**: Invoke Task tool with:
  - `subagent_type`: "qa"
  - `prompt`: "Generate QA plan and report for ticket $1. Read `.claude/skills/qa/SKILL.md` for instructions. Arguments: $1"
  - `description`: "QA for $1"
- **On Task Return**: When the QA task completes, IMMEDIATELY proceed to Gate 7.

#### Gate 7: DOCS_UPDATED
- **Condition**: Summary file `docs/summaries/$1-summary.md` does not exist
- **Action**: Invoke Task tool with:
  - `subagent_type`: "tech-writer"
  - `prompt`: "Update documentation for ticket $1. Read `.claude/skills/docs-update/SKILL.md` for instructions. Arguments: $1"
  - `description`: "Update docs for $1"
- **On Task Return**: When the docs task completes, IMMEDIATELY proceed to Step 3 (Confirm Completion).

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

- **NEVER PAUSE AFTER TOOL RETURNS**: When a Skill or Task tool completes and returns output, you MUST immediately continue to the next instruction. The ONLY way to pause is via AskUserQuestion at designated checkpoints.
- Execute gates sequentially - each depends on the previous
- **Checkpoints**: Use AskUserQuestion at designated checkpoints (after PRD, Plan, Implementation) to let users pause or continue. Always use the structured AskUserQuestion tool - never just output text asking for confirmation.
- If user selects "Pause to review" at any checkpoint, re-prompt with AskUserQuestion until they select "Continue". Never terminate the workflow at intermediate checkpoints.
- If any gate fails, stop and report the issue
- Description file sync happens only after successful completion of all gates
- **Review loop**: Gate 5 can loop back to Gate 4 if fixes are requested. Track loop count to prevent infinite loops.
- **Task vs Skill**: Gates 4-7 use Task tool (which creates separate agent context) to ensure proper return to parent workflow. Gates 1-3 use Skill tool for simpler orchestration.

## Review Loop Handling

When in a review loop (Gate 5 returned `REVIEW_NEEDS_FIXES`):

1. **Track review loop state internally** - remember that you are in a review loop
2. **After Gate 4 Task completes during review loop:**
   - Do NOT show the "Code Done" checkpoint
   - IMMEDIATELY return to Gate 5 for re-review
3. **Reset review loop tracking** when Gate 5 returns `REVIEW_OK`
4. **Why this works:** The Task tool creates a separate agent context. When the sub-agent completes, control automatically returns here, and you retain knowledge of being in a review loop.

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
