---
description: "Quickly implement tasks from a phase file with automated coding and review"
argument-hint: "[phase-file-path]"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Task, AskUserQuestion
model: sonnet
---

## Input Validation

If `$1` is empty or the file does not exist:
- Display: "Error: Phase file path required. Usage: /quick-implement docs/phase/phase-16.md"
- Terminate immediately.

---

## CRITICAL: REQUIREMENTS ARE IMMUTABLE

Agents MUST implement code according to the requirements in the phase file and project docs.
- Implement exactly what the task specifies
- If existing code contradicts requirements, modify the code
- Report ambiguities via AskUserQuestion — never guess

---

## Orchestrator Workflow

You coordinate a `coder` agent (writes code + tests) and a `reviewer` agent (verifies builds and quality). Process all tasks to completion automatically.

### Step 1: Input & Context

1. Read the phase file (`$1`) — contains the task list
2. Read `docs/idea.md` — project description
3. Read `docs/vision.md` — architecture and phase ordering
4. Read `docs/conventions.md` — coding rules reference

### Step 2: Task Extraction

1. Parse the phase file for lines matching `- [ ]` (unchecked tasks)
2. If no unchecked tasks exist: Report "All tasks in `$1` are complete." and return
3. Pick the **first** unchecked task. Store its exact line text for later update

### Step 3: Announce

Briefly announce:
- Task description (1-2 sentences)
- Likely files affected

Then proceed immediately to implementation.

### Step 4: Code Implementation

Invoke the `coder` subagent via Task tool:

```
Task(
  subagent_type: "coder",
  model: "opus",
  prompt: "Implement the following task:
    <task description>

    Context:
    <summary from idea.md and vision.md>

    Read docs/conventions.md for coding rules.
    Relevant source files: <list of likely files>

    CRITICAL: Implement according to the requirements. If existing code contradicts requirements, modify the code to match. Do NOT modify requirements documents.",
  description: "Implement <brief task name>"
)
```

### Step 5: Review

Invoke the `reviewer` subagent via Task tool:

```
Task(
  subagent_type: "reviewer",
  model: "sonnet",
  prompt: "Review and verify the following task implementation:
    <task description>

    Files modified by coder: <list from coder output>

    Read docs/conventions.md for the conventions checklist.
    Run build verification and code quality review.
    Report PASS, FAIL_BUILD, or FAIL_REVIEW.",
  description: "Review <brief task name>"
)
```

**Branch on result:**
- `PASS` → go to Step 7
- `FAIL_BUILD` → go to Step 6 (build refinement)
- `FAIL_REVIEW` → go to Step 6 (review refinement)

### Step 6: Refinement Loop

**Build failures** (max 3 iterations):
1. Re-invoke `coder` with exact error output and instruction to fix
2. Re-invoke `reviewer` after fix
3. If 3 build-fix iterations exhausted → report failure, use AskUserQuestion:
   - Question: "Build refinement failed after 3 attempts. How should we proceed?"
   - Options:
     1. "Skip this task" — move to next task
     2. "Stop entirely" — terminate skill

**Review failures** (max 2 iterations, only after build passes):
1. Re-invoke `coder` with review feedback (specific file:line references and fix instructions)
2. Re-invoke `reviewer` after fix
3. If 2 review-fix iterations exhausted → report remaining review issues as warnings, treat as soft pass, proceed to Step 7

### Step 7: Completion

1. **Update phase file**: Use Edit tool to change `- [ ]` to `- [x]` for the completed task's exact line
2. **Show summary**:
   - Files modified/created
   - Tests added
   - Verification results
3. **Check for remaining tasks**:
   - If more `- [ ]` tasks remain → return to Step 2 with next task
   - If all complete → report "All tasks in `$1` are complete." and return

---

## Error Handling

| Situation | Action |
|-----------|--------|
| Coder reports ambiguity | Use AskUserQuestion for clarification |
| Build refinement stuck (3 iterations) | Report failure, ask user whether to continue or stop |
| Review refinement exhausted (2 iterations) | Warn and continue (soft pass) |
| Coder or reviewer agent fails | Report error, use AskUserQuestion to ask how to proceed |

---

## Notes

- All tasks are processed automatically without confirmation prompts
- Commits are handled separately, not by this orchestrator
- When all tasks are complete, return to caller without asking
- The coder writes both production code AND tests (unlike implement-orchestrated)
