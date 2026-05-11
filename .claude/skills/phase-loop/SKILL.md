---
name: phase-loop
description: "Use when a tasklist with multiple phases needs to be executed end-to-end without manual checkpoints"
allowed-tools: Read, Write, Edit, Glob, Grep, Agent, Bash, Skill, AskUserQuestion
model: sonnet
---

You drive every incomplete phase in `docs/tasklist.md` to completion by repeatedly: syncing state, picking the next phase, dispatching `quick-implement`, and (optionally) committing. Subagent invocation pattern: see `../_shared/subagent-registry.md`. Status markers: see `../_shared/status-markers.md`.

## Argument Parsing

- `--no-commit` — disables the post-phase auto-commit. Default is to commit after each phase.

No other arguments — phases are discovered from `docs/tasklist.md`.

## EXECUTION CONTRACT

Execute every numbered step below in sequence. After each tool returns, immediately proceed to the next step. The only valid stopping points are **Step 6 (WORKFLOW COMPLETE)** or an explicit error. There are no user checkpoints inside this workflow — the user can interrupt manually if needed.

`MAX_ITERATIONS = 10` — safety cap on total phase iterations per run.

---

### Step 1 — Initialize

1. Parse `$ARGUMENTS`. Set `AUTO_COMMIT = true` unless `--no-commit` is present.
2. Initialize `iteration = 0`, `phases_completed = []`, `consecutive_failures = {}`.
3. Read `docs/tasklist.md`. Find the **Feature Phases** table.
4. Count rows where status is `Complete` (done) vs `In Progress`/`Pending` (open).
5. Print: `Starting phase loop. <done>/<total> phases complete.`

**Proceed to Step 2.**

---

### Step 2 — Sync state

Delegate to the `sync-phases` skill, which is the source of truth for phase ↔ tasklist synchronization:

```
Skill(skill: "sync-phases")
```

After it returns:

1. Re-read `docs/tasklist.md` (it may have been updated).
2. Scan the **Feature Phases** table for the **first** row whose status is not `Complete`. Extract its phase number → `CURRENT_PHASE`.
3. If every row is `Complete` → **jump to Step 6**.
4. Set `PHASE_FILE = docs/phase/phase-CURRENT_PHASE.md` (sync-phases will already have created it if it was missing — see `../_shared/phase-file-template.md`).
5. Read `PHASE_FILE` and verify it has at least one `- [ ]` task. If every task is `[x]`, the sync hadn't propagated yet — loop back to Step 2 once. If still inconsistent on the second pass, print error and stop.

**Proceed to Step 3.**

---

### Step 3 — Announce

Read `PHASE_FILE` to count remaining `- [ ]` tasks and extract the title.

Print: `Phase <CURRENT_PHASE>: <title> — <count> tasks remaining` (mention whether `PHASE_FILE` was just created vs already present).

**Proceed to Step 4.**

---

### Step 4 — Implement

```
Agent(
  subagent_type: "general-purpose",
  model: "sonnet",
  description: "Implement Phase <CURRENT_PHASE>",
  prompt: "Read '.claude/skills/quick-implement/SKILL.md' for full instructions.
           Execute it with argument: '<PHASE_FILE>'.
           Process ALL unchecked tasks in that file."
)
```

Wait for the result.

**Proceed to Step 5.**

---

### Step 5 — Post-implementation

1. Re-read `PHASE_FILE`. Count `[x]` and `[ ]`.
2. **If not all tasks are `[x]`:**
   - Increment `consecutive_failures[CURRENT_PHASE]`.
   - If `consecutive_failures[CURRENT_PHASE] >= 2`:
     - `AskUserQuestion`:
       - question: `Phase <CURRENT_PHASE> has failed 2 consecutive attempts. How should we proceed?`
       - header: `Phase stuck`
       - options:
         1. `Skip this phase` — mark as skipped in `phases_completed`, continue.
         2. `Stop the loop` — jump to Step 6 with a partial report.
3. **If `AUTO_COMMIT` is true and at least one task was newly completed:**
   - Run: `git add -A && git commit -m "feat: complete Phase <CURRENT_PHASE> (<title>)"`
   - On commit failure: warn and continue (best-effort).
4. **If all tasks are `[x]`:**
   - Append `<CURRENT_PHASE>` to `phases_completed`.
   - Reset `consecutive_failures[CURRENT_PHASE]`.
5. Increment `iteration`.
6. **If `iteration > MAX_ITERATIONS`:**
   - `AskUserQuestion`:
     - question: `Reached MAX_ITERATIONS (<MAX_ITERATIONS>). How should we proceed?`
     - header: `Iteration cap`
     - options:
       1. `Extend +5` — increase `MAX_ITERATIONS` by 5, continue.
       2. `Stop here` — jump to Step 6 with a partial report.
7. Loop back to **Step 2**.

---

### Step 6 — WORKFLOW COMPLETE

Print:
- Phases completed this run (numbers + titles).
- Total iterations used.
- Remaining incomplete phases (if any).
- Whether auto-commit was enabled.

This is the only valid stopping point.

---

## Error Handling

| Situation | Action |
|---|---|
| `quick-implement` partial completion | Loop to Step 2; sync picks up the same phase with remaining tasks. |
| `quick-implement` fails entirely | `AskUserQuestion`: Retry / Skip / Stop. |
| Same phase fails 2 consecutive iterations | Step 5 handles via `AskUserQuestion`. |
| `MAX_ITERATIONS` exceeded | Step 5 handles via `AskUserQuestion`. |
| Git commit fails | Warn and continue (best-effort). |
| `docs/tasklist.md` missing or unparseable | Print error and stop. |
| Phase file cannot be created (missing data in tasklist) | `sync-phases` reports the error; this skill stops. |

## Key References

| File | Role |
|---|---|
| `../sync-phases/SKILL.md` | Phase ↔ tasklist sync algorithm (called every iteration in Step 2) |
| `../quick-implement/SKILL.md` | Invoked in Step 4 |
| `../_shared/phase-file-template.md` | Layout `sync-phases` uses when creating a new phase file |
| `../_shared/status-markers.md` | Why phase status uses `Complete` text, not emoji |
| `docs/tasklist.md` | Feature Phases table + per-phase task sections |
| `docs/phase/phase-N.md` | Per-phase task checklists |
