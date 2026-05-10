---
name: implement-orchestrated
description: "Use when a task needs separated code and test phases with automated verification and refinement"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Agent, rust-analyzer-lsp, AskUserQuestion
model: sonnet
---

You orchestrate two `general-purpose` subagents — one for production code, one for tests — to drive a single tasklist to completion. Subagent invocation pattern: see `../_shared/subagent-registry.md`.

## Ticket Resolution

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line as `TICKET_ID`.
3. If the file is missing or empty: print `Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket` and stop.

Otherwise `TICKET_ID = $1`.

## Argument Flags

- `--auto` — process all unchecked tasks without pausing between them. Default when invoked from `dev-cycle` or `feature-development`. When absent, the orchestrator still processes all tasks but prints an explicit per-task announcement before each.

## Requirements

Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`. Both subagents must implement what the PRD/plan specifies; if existing code contradicts the spec, the code changes, not the requirements.

## Workflow

### Step 1 — Setup

1. Read `docs/tasklist/TICKET_ID.md`. Find the first task line matching `- [ ]`.
   - If none, print `All tasks in docs/tasklist/TICKET_ID.md are already complete.` and **return to caller**.
   - Store the **exact** task line text (e.g. `- [ ] 1.1 Create workspace Cargo.toml`) for the Step 7 update.
2. Read `docs/prd/TICKET_ID.prd.md` and `docs/plan/TICKET_ID.md` for context.
3. Create a savepoint:
   ```bash
   git stash push -m "pre-implement-TICKET_ID-$(date +%s)" --include-untracked
   ```
   If the working tree is clean, skip stash and record `git rev-parse HEAD` for rollback.

**Proceed to Step 2.**

### Step 2 — Announce

Briefly say (1-2 sentences) what the task is and which files are likely to change. Then proceed immediately.

**Proceed to Step 3.**

### Step 3 — Code implementation

```
Agent(
  subagent_type: "general-purpose",
  model: "opus",
  description: "Implement <brief task name>",
  prompt: "Implement task: <task description>.
           Files likely to change: <list>.
           Read docs/conventions.md and CLAUDE.md for project rules.
           Read docs/prd/TICKET_ID.prd.md and docs/plan/TICKET_ID.md for requirements.
           Apply '.claude/skills/_shared/requirements-immutability.md'.
           Write production code only — tests come in the next step."
)
```

Wait for the result.

**Proceed to Step 4.**

### Step 4 — Test implementation

```
Agent(
  subagent_type: "general-purpose",
  model: "sonnet",
  description: "Test <brief task name>",
  prompt: "Write tests for the implementation just produced for task: <task description>.
           Files modified by code phase: <list from Step 3 result>.
           Read docs/conventions.md for test placement rules.
           Tests must verify the requirements from docs/prd/TICKET_ID.prd.md, not just current behavior.
           Apply '.claude/skills/_shared/requirements-immutability.md'."
)
```

Wait for the result.

**Proceed to Step 5.**

### Step 5 — Verification

Run in sequence:

```bash
cargo fmt
cargo check
cargo test
cargo clippy --tests -- -D warnings
```

Collect every command's exit status. If all pass, **proceed to Step 6 (security audit)**. If any fail, **proceed to Step 7 (refinement)**.

### Step 6 — Security audit

If the project uses Cargo (it does, in synapse):

```bash
cargo audit
```

If `cargo audit` fails:
- Do NOT enter the refinement loop — security advisories are not auto-fixable.
- Invoke `AskUserQuestion`:
  - question: `Security vulnerability detected in dependencies. How should we proceed?`
  - header: `Security`
  - options:
    1. `Ignore advisory` — append the RUSTSEC ID to `.cargo/audit.toml` with a justification comment, then continue to Step 8.
    2. `Stop and review` — terminate with a message that manual intervention is required.

If `cargo audit` passes, **proceed to Step 8**.

### Step 7 — Refinement loop

If any verification step in Step 5 failed:

1. **Blame analysis:**
   - Compilation error in `src/` (non-test) → code phase issue.
   - Compilation error in `#[cfg(test)]` blocks or `*/tests.rs` → test phase issue.
   - Failing assertion → most likely code phase (code doesn't match spec).
   - Clippy warning → whoever owns the file the warning points at.
2. Track `iteration` (max 3 total refinements per task).
3. **Stuck check:** if the same errors appear two iterations in a row, treat as stuck.
4. Re-invoke the responsible subagent (use the Step 3 or Step 4 invocation form, but quote the exact errors and instruct: "fix only the errors above, don't add unrelated changes").
5. Return to **Step 5**.
6. **If max iterations reached or stuck:**
   ```bash
   git stash pop  # or git restore . && git checkout <recorded HEAD>
   ```
   Print `Refinement failed after 3 attempts. Changes have been rolled back. Manual intervention required.` and return to caller with a failure status.

### Step 8 — Completion

1. Use `Edit` to flip the stored task line in `docs/tasklist/TICKET_ID.md` from `- [ ]` to `- [x]`.
2. Print a one-block summary: files modified, tests added, verification results, tasklist update confirmed.
3. **Check for more `- [ ]` tasks in the tasklist:**
   - If any remain → return to **Step 1** with the next task.
   - If none remain → set `Status: IMPLEMENT_STEP_OK` in `docs/tasklist/TICKET_ID.md`, print `All tasks in docs/tasklist/TICKET_ID.md are complete.`, and **return to caller**. Do not invoke review or QA — the parent orchestrator owns those gates.

## Error Handling

| Situation | Action |
|---|---|
| Code subagent reports ambiguity | Stop, invoke `AskUserQuestion` for clarification before re-invoking. |
| Test subagent finds a production bug | Report to user via `AskUserQuestion`: fix in this task, or open a follow-up? |
| Refinement stuck (same errors twice) | Roll back, escalate to user. |
| Max refinements (3) reached | Roll back, show errors, terminate. |
| Git stash/restore fails | Print error, ask user to resolve manually. |

## Notes

- Tests are always written after code — never skipped.
- Commits are owned by the parent orchestrator, not this skill.
- Status markers (`IMPLEMENT_STEP_OK`, etc.) are listed in `../_shared/status-markers.md`.
