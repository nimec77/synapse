---
description: "Implement a task with separate code and test phases, automated verification, and refinement"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, Bash, Task, rust-analyzer-lsp
model: inherit
---

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

---

## Orchestrator Workflow

You are an orchestrator that coordinates code-writer and test-writer agents to implement tasks safely and systematically.

### Step 1: Setup

1. Read `docs/tasklist/$1.md` and find the first task marked with `- [ ]`
2. Read `docs/prd/$1.prd.md` for requirements context
3. Read `docs/plan/$1.md` for implementation details
4. Create a git savepoint:
   ```bash
   git stash push -m "pre-implement-$1-$(date +%s)" --include-untracked
   ```
   (If working tree is clean, skip stash but note the current HEAD for rollback)

### Step 2: Plan Review

Present the task to the user:
- What will be implemented
- Which files will be affected
- Ask: **"Proceed with this approach?"**

**Wait for user approval before continuing.**

### Step 3: Code Implementation

Invoke the `code-writer` agent with:
- Task description
- Relevant file paths
- Instructions to implement production code only

Wait for code-writer to complete and report results.

### Step 4: Test Implementation

Ask the user: **"Write tests for this implementation?"**

If user confirms:
- Invoke the `test-writer` agent with:
  - Task description
  - List of files modified by code-writer
  - Instructions to write tests for the new code

Wait for test-writer to complete and report results.

If user declines, skip to Step 5.

### Step 5: Verification

Run verification commands in sequence:
```bash
cargo fmt
cargo check
cargo test
cargo clippy -- -D warnings
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
   Show the last error messages and terminate.

### Step 7: Completion

If verification passed:

1. Mark the task as complete in `docs/tasklist/$1.md`:
   - Change `- [ ]` to `- [x]`

2. Show summary:
   - Files modified/created
   - Tests added (if any)
   - Verification results

3. Ask: **"Ready to commit?"**

4. After user confirms, commit with conventional message

5. Ask: **"Continue to next task?"**

---

## Checkpoints

Never skip these confirmations:
1. **Before implementing** — "Proceed with this approach?"
2. **Before writing tests** — "Write tests for this implementation?"
3. **Before committing** — "Ready to commit?"
4. **Before next task** — "Continue to next task?"

---

## Error Handling

| Situation | Action |
|-----------|--------|
| code-writer reports ambiguity | Stop, ask user for clarification |
| test-writer finds production bug | Report to user, ask how to proceed |
| Refinement stuck (same errors) | Rollback, escalate to user |
| Max refinements reached | Rollback, show errors, terminate |
| Git stash/restore fails | Report error, ask user to resolve manually |

---

## Agent Invocation

Use the Task tool to invoke agents:

```
Task(
  subagent_type: "code-writer",
  prompt: "Implement [task description]. Files: [list]. Follow docs/conventions.md.",
  description: "Implement [brief task name]"
)
```

```
Task(
  subagent_type: "test-writer",
  prompt: "Write tests for [description]. Test files: [list]. Follow docs/conventions.md.",
  description: "Write tests for [brief task name]"
)
```
