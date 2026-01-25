---
description: "Implement a task with separate code and test phases, automated verification, and refinement"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, Bash, Task, rust-analyzer-lsp, AskUserQuestion
model: inherit
---

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

---

## CRITICAL: REQUIREMENTS ARE IMMUTABLE

**Agents MUST implement code according to the requirements in the PRD and plan.**

The code-writer and test-writer agents are NOT permitted to:
- ❌ Modify requirements documents (PRD, plan, phase docs)
- ❌ Skip implementing a requirement because existing code "works differently"
- ❌ Justify deviations from requirements
- ❌ Mark tasks complete when implementation doesn't match requirements

The agents MUST:
- ✅ Implement exactly what the task acceptance criteria specify
- ✅ Modify existing code if it doesn't match requirements
- ✅ Report deviations that cannot be fixed (use AskUserQuestion for guidance)
- ✅ Ensure tests verify the requirements, not just existing behavior

**When invoking agents, explicitly remind them:**
> "Implement according to the requirements. If existing code contradicts requirements, modify the code to match requirements."

---

## Orchestrator Workflow

You are an orchestrator that coordinates code-writer and test-writer agents to implement tasks safely and systematically. Process all tasks to completion automatically, showing progress as you go.

### Step 1: Setup

1. Read `docs/tasklist/$1.md` and find the first task marked with `- [ ]`
   - **If no unchecked tasks exist**: Report "All tasks in `docs/tasklist/$1.md` are already complete." and **return to caller**
   - **Store the exact task line text** (e.g., `- [ ] 1.1 Create workspace Cargo.toml`) for later update
2. Read `docs/prd/$1.prd.md` for requirements context
3. Read `docs/plan/$1.md` for implementation details
4. Create a git savepoint:
   ```bash
   git stash push -m "pre-implement-$1-$(date +%s)" --include-untracked
   ```
   (If working tree is clean, skip stash but note the current HEAD for rollback)

### Step 2: Plan Announcement

**Briefly announce** what will be implemented:
- Task description (1-2 sentences)
- Files that will be created or modified

Then proceed immediately to implementation.

### Step 3: Code Implementation

Invoke the `code-writer` agent with:
- Task description
- Relevant file paths
- Instructions to implement production code only

Wait for code-writer to complete and report results.

### Step 4: Test Implementation

**Always write tests.** Invoke the `test-writer` agent with:
- Task description
- List of files modified by code-writer
- Instructions to write tests for the new code

Wait for test-writer to complete and report results.

### Step 5: Verification

Run verification commands in sequence:
```bash
cargo fmt
cargo check
cargo test
cargo clippy --tests -- -D warnings
```

Collect results for all commands.

### Step 6: Refinement (if verification failed)

If any verification step failed:

1. **Parse the errors** to determine blame:
   - Compilation error in `src/` → code-writer issue
   - Compilation error in `#[cfg(test)]` or `tests/` → test-writer issue
   - Test assertion failed → likely code-writer issue (code doesn't match spec)
   - Clippy warning → whoever wrote that code

2. **Track iteration count** (max 3 refinements)

3. **Check for progress**:
   - Compare current errors to previous iteration
   - If identical errors for 2 iterations → stuck, escalate

4. **Re-invoke the responsible agent** with:
   - The specific error messages
   - Instructions to fix only their code
   - Reminder of what was originally intended

5. **Re-run verification** (return to Step 5)

6. **If max iterations reached or stuck**:
   ```bash
   git stash pop  # or git restore . if no stash
   ```
   Report: "Refinement failed after 3 attempts. Changes have been rolled back. Manual intervention required."
   Show the last error messages and return to caller with failure status.

### Step 7: Completion

If verification passed:

1. **Update the tasklist** in `docs/tasklist/$1.md`:
   - Use the Edit tool to replace the stored task line from Step 1
   - Change `- [ ]` to `- [x]` for that specific task
   - Example: `- [ ] 1.1 Create workspace` → `- [x] 1.1 Create workspace`

2. Show summary:
   - Files modified/created
   - Tests added
   - Verification results
   - Tasklist update confirmation

3. **Check for remaining tasks** in `docs/tasklist/$1.md`:
   - If there are more unchecked tasks (`- [ ]`): Return to Step 1 immediately with the next task
   - If **all tasks are complete** (no `- [ ]` remaining):
     - Update tasklist status to `IMPLEMENT_STEP_OK`
     - Report: "All tasks in `docs/tasklist/$1.md` are complete."
     - **Return to caller** — do not ask about other files or next steps. The parent command will handle subsequent gates.

---

## Notes

- Tests are always written automatically after code implementation.
- Commits are handled separately, not by this orchestrator.
- When all tasks are complete, return to caller without asking — the parent command controls subsequent workflow gates.
- All tasks are processed automatically without confirmation prompts.

---

## Error Handling

| Situation | Action |
|-----------|--------|
| code-writer reports ambiguity | Stop, use AskUserQuestion for clarification |
| test-writer finds production bug | Report to user, use AskUserQuestion to ask how to proceed |
| Refinement stuck (same errors) | Rollback, escalate to user |
| Max refinements reached | Rollback, show errors, terminate |
| Git stash/restore fails | Report error, ask user to resolve manually |

---

## Agent Invocation

Use the Task tool to invoke agents. **Always include the requirements reminder.**

```
Task(
  subagent_type: "code-writer",
  prompt: "Implement [task description]. Files: [list]. Follow docs/conventions.md.

  CRITICAL: Implement according to the requirements in the PRD and plan. If existing code contradicts requirements, modify the code to match requirements. Do NOT modify requirements documents. Do NOT skip requirements because existing code 'works differently'.",
  description: "Implement [brief task name]"
)
```

```
Task(
  subagent_type: "test-writer",
  prompt: "Write tests for [description]. Test files: [list]. Follow docs/conventions.md.

  CRITICAL: Tests must verify the requirements from the PRD/plan, not just existing behavior. If existing code doesn't match requirements, the tests should expect the REQUIRED behavior, not the current behavior.",
  description: "Write tests for [brief task name]"
)
```
