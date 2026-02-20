---
description: "Continuous phase loop: sync status, implement next phase, commit, repeat"
allowed-tools: Read, Write, Edit, Glob, Grep, Task, Bash, AskUserQuestion
model: sonnet
---

## Argument Parsing

- `--no-commit`: disables auto-commit after each phase (default: commit after each phase)
- No other arguments needed — phases are discovered automatically from `docs/tasklist.md`

---

## EXECUTION CONTRACT

**You MUST execute every numbered step below in sequence. After every tool return, immediately proceed to the next step. The ONLY valid stopping points are Step 6 (WORKFLOW COMPLETE) or an explicit error. Stopping before WORKFLOW COMPLETE is a contract violation. There are NO user checkpoints — this workflow is fully automatic.**

**`MAX_ITERATIONS = 10`** — safety cap on total phase iterations per run.

---

### Step 1: Initialize

1. Parse `$ARGUMENTS` for `--no-commit` flag. Set `AUTO_COMMIT = true` unless `--no-commit` is present.
2. Set `iteration = 0`, `phases_completed = []`, `consecutive_failures = {}`
3. Read `docs/tasklist.md`
4. Count phases: `:green_circle:` = done, `:white_circle:` = pending
5. Report: **"Starting phase loop. X/Y phases complete."**

**After initialization, execute Step 2.**

---

### Step 2: Sync Phase Status

> **Note:** This logic mirrors `.claude/skills/sync-phases/SKILL.md` — inlined here for autonomous execution.

**2a.** Read `docs/tasklist.md` (refresh on each iteration)

**2b.** Glob `docs/phase/phase-[0-9]*.md` to find all existing phase files

**2c.** For each existing phase file:
1. Read the phase file
2. Count `[x]` (complete) and `[ ]` (incomplete) tasks
3. If ALL tasks are `[x]`:
   - Update the corresponding row in the Feature Phases table: status to `:green_circle:`, progress to `X/X`
   - Sync individual task checkboxes in the phase section of `docs/tasklist.md` to `[x]`
4. If some tasks are complete but not all:
   - Update progress count in Feature Phases table (e.g., `3/5`)
   - Sync individual task completion status to `docs/tasklist.md`

**2d.** Scan the Feature Phases table in `docs/tasklist.md` for the **first** row where status is NOT `:green_circle:` → this is `CURRENT_PHASE` (extract the phase number)

**2e.** If no non-`:green_circle:` row found → jump to **Step 6** (ALL COMPLETE)

**2f.** Set `PHASE_FILE = docs/phase/phase-{CURRENT_PHASE}.md`

If `PHASE_FILE` does NOT exist, create it by extracting from `docs/tasklist.md`:
- Phase title from `## Phase N: Title`
- Goal from `**Goal:**` line (if present) or derive from description
- All tasks for that phase (`- [ ] N.x ...`)
- Test/acceptance criteria from `**Verify:**` line
- Dependencies from the Feature Phases table `Depends on` column

Use this structure:
```markdown
# Phase N: Title

**Goal:** [extracted goal]

## Tasks

- [ ] N.1 [task description]
- [ ] N.2 [task description]
...

## Acceptance Criteria

**Test:** [extracted test criteria]

## Dependencies

- [extracted dependencies or "None"]
```

**2g.** Update the `**Current Phase:**` line in `docs/tasklist.md` to reflect `CURRENT_PHASE`

**2h.** Read `PHASE_FILE` and verify it has at least one `- [ ]` task. If ALL tasks are `[x]`:
- This phase is already complete — loop back to **2d** to find the next incomplete phase

**After sync completes with a valid PHASE_FILE, execute Step 3.**

---

### Step 3: Announce Phase

Read `PHASE_FILE` to count remaining `- [ ]` tasks.

Report: **"Phase {N}: {title} — {count} tasks remaining"**

Include whether the phase file was just created or already existed.

**After announcement, execute Step 4.**

---

### Step 4: Implement Phase

Invoke the quick-implement workflow via Task subagent:

```
Task(
  subagent_type: "general-purpose",
  model: "sonnet",
  prompt: "Read `.claude/skills/quick-implement/SKILL.md` for full instructions.
           Execute it with argument: `{PHASE_FILE}`
           (e.g., `docs/phase/phase-17.md`).
           Process ALL unchecked tasks in that file.",
  description: "Implement Phase {N} tasks"
)
```

**Data flow:** Step 2 produces `PHASE_FILE` → Step 4 passes it as the quick-implement `$1` argument → quick-implement reads the file and processes all `- [ ]` tasks.

**After Task returns, execute Step 5.**

---

### Step 5: Post-Implementation

**5a.** Read `PHASE_FILE` and check task completion:
- Count `[x]` and `[ ]` tasks
- If NOT all tasks are `[x]`: record a failure for this phase in `consecutive_failures`

**5b.** Check consecutive failure count for this phase:
- If same phase has failed **2 consecutive iterations** → use AskUserQuestion:
  - Question: "Phase {N} has failed 2 consecutive attempts. How should we proceed?"
  - Options:
    1. "Skip this phase" — mark phase as skipped, continue to next
    2. "Stop the loop" — jump to Step 6 with partial report

**5c.** If `AUTO_COMMIT` is true and at least one task was completed:
- Run: `git add -A && git commit -m "feat: complete Phase {N} ({title})"`
- If git commit fails: warn and continue (best-effort)

**5d.** If all tasks in `PHASE_FILE` are `[x]`:
- Append phase number to `phases_completed`
- Reset consecutive failure count for this phase

**5e.** Increment `iteration`

**5f.** If `iteration > MAX_ITERATIONS`:
- Use AskUserQuestion:
  - Question: "Reached MAX_ITERATIONS ({MAX_ITERATIONS}). How should we proceed?"
  - Options:
    1. "Extend +5 iterations" — increase MAX_ITERATIONS by 5, continue
    2. "Stop here" — jump to Step 6 with partial report

**5g.** Go back to **Step 2**

---

### Step 6: WORKFLOW COMPLETE

Report final summary:
- Phases completed this run: `phases_completed` (list phase numbers and titles)
- Total iterations used: `iteration`
- Remaining incomplete phases (if any)
- Whether auto-commit was enabled

**This is the ONLY valid stopping point.**

---

## Error Handling

| Situation | Action |
|-----------|--------|
| quick-implement partial completion | Loop back to Step 2 (sync picks up same phase with remaining tasks) |
| quick-implement fails entirely | AskUserQuestion: "Retry" / "Skip this phase" / "Stop the loop" |
| Same phase fails 2 consecutive iterations | AskUserQuestion: "Skip" or "Stop?" (Step 5b) |
| MAX_ITERATIONS exceeded | AskUserQuestion: "Extend +5" or "Stop?" (Step 5f) |
| Git commit fails | Warn in output, continue to next iteration (best-effort) |
| Phase file cannot be created (missing data in tasklist) | Report error and stop |
| `docs/tasklist.md` missing or unparseable | Report error and stop |

---

## Key References

| File | Role |
|------|------|
| `.claude/skills/sync-phases/SKILL.md` | Source of truth for sync algorithm (inlined into Step 2) |
| `.claude/skills/quick-implement/SKILL.md` | Invoked via Task in Step 4 |
| `.claude/skills/dev-cycle/SKILL.md` | Pattern reference for execution contract and Task invocation |
| `docs/tasklist.md` | Data structure: Feature Phases table, phase sections, checkboxes |
| `docs/phase/phase-*.md` | Individual phase files with task checklists |
