---
name: quick-implement
description: "Use when a phase file lists ready tasks that need automated coding plus review without the full orchestrated cycle"
argument-hint: "[phase-file-path]"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, Agent, AskUserQuestion
model: sonnet
---

You orchestrate a single `general-purpose` coder subagent (writes code **and** tests in one pass) and a `general-purpose` reviewer subagent (verifies build + quality) to drive every unchecked task in a phase file to completion. Subagent invocation pattern: see `../_shared/subagent-registry.md`.

## Input Validation

If `$1` is empty or the file does not exist:
- Print `Error: Phase file path required. Usage: /quick-implement docs/phase/phase-N.md` and stop.

## Requirements

Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`. The coder must implement what the phase file and project docs specify; if existing code contradicts the spec, the code changes.

## Workflow

### Step 1 — Context

Read in order:
1. The phase file (`$1`) — the task list.
2. `docs/idea.md` — project description (if present, skip if missing).
3. `docs/vision.md` — architecture and phase ordering.
4. `docs/conventions.md` — coding rules.

### Step 2 — Pick a task

Parse `$1` for the **first** line matching `- [ ]`. Store the exact line text for the Step 6 update.

If no unchecked task exists: print `All tasks in $1 are complete.` and **return to caller**.

### Step 3 — Announce

In 1-2 sentences: task description and likely files affected. Then proceed.

### Step 4 — Implement

```
Agent(
  subagent_type: "general-purpose",
  model: "opus",
  description: "Implement <brief task name>",
  prompt: "Implement the following task and write its tests in the same pass:
           <task description>

           Context: <2-3 sentence summary from idea.md and vision.md>
           Project rules: read docs/conventions.md and CLAUDE.md.
           Apply '.claude/skills/_shared/requirements-immutability.md'.
           Likely files: <list>.

           Output: list every file you created or modified, separated by newlines."
)
```

Wait for result. Keep the file list for Step 5.

### Step 5 — Review

```
Agent(
  subagent_type: "general-purpose",
  model: "opus",
  description: "Review <brief task name>",
  prompt: "Review the implementation just produced for task: <task description>.
           Files modified by coder: <list from Step 4>.

           Run verification:
             cargo fmt --check
             cargo check
             cargo test
             cargo clippy --tests -- -D warnings
           Then assess code quality against docs/conventions.md.

           Report exactly one of these statuses on the LAST line of your output:
             PASS              — verification clean and quality acceptable
             FAIL_BUILD        — verification failed (fmt, check, test, or clippy)
             FAIL_REVIEW       — verification clean but quality issues need fixing"
)
```

Branch on the last-line status:

- `PASS` → **Step 6**.
- `FAIL_BUILD` → **Step 5a (build refinement)**.
- `FAIL_REVIEW` → **Step 5b (review refinement)**.

### Step 5a — Build refinement (max 3 iterations)

1. Re-invoke the coder (Step 4 form) with the exact error output and instruction to fix only those errors.
2. Re-invoke the reviewer (Step 5 form).
3. If three build-fix iterations exhaust without `PASS`:
   - `AskUserQuestion`:
     - question: `Build refinement failed after 3 attempts. How should we proceed?`
     - options: `Skip this task` (move to next) / `Stop entirely` (terminate skill).

### Step 5b — Review refinement (max 3 iterations)

1. Re-invoke the coder with the reviewer's specific file:line feedback and fix instructions.
2. Re-invoke the reviewer.
3. If three review-fix iterations exhaust without `PASS`: report remaining issues as warnings, treat as soft pass, **proceed to Step 6**.

> Iteration count is **3 for both build and review refinement** — matches `implement-orchestrated`. Earlier versions had a 2/3 split; the asymmetry caused confusion and one extra build attempt rarely changed the outcome.

### Step 6 — Completion

1. Use `Edit` to flip the stored task line from `- [ ]` to `- [x]` in `$1`.
2. Print a one-block summary: files modified, tests added, verification results.
3. **Check for more `- [ ]` tasks in `$1`:**
   - If any remain → return to **Step 2** with the next task.
   - If none remain → print `All tasks in $1 are complete.` and **return to caller**.

## Error Handling

| Situation | Action |
|---|---|
| Coder reports ambiguity | `AskUserQuestion` for clarification. |
| Build refinement exhausted (3 iterations) | `AskUserQuestion`: skip task or stop. |
| Review refinement exhausted (3 iterations) | Soft pass with warnings, continue. |
| Coder or reviewer subagent fails | Report error, `AskUserQuestion` to ask how to proceed. |

## Notes

- Tasks are processed automatically without confirmation prompts.
- Commits are owned by the caller (typically `phase-loop`), not this skill.
- The coder writes both production code AND tests (this is the difference from `implement-orchestrated`, which separates them).
