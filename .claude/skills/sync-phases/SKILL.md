---
name: sync-phases
description: "Use when phase completion checkboxes in tasklist.md and individual phase-*.md files have drifted out of sync"
allowed-tools: Read, Write, Edit, Glob, Grep
model: sonnet
---

You synchronize task-completion state between `docs/tasklist.md` (the master view) and the individual `docs/phase/phase-*.md` files (the per-phase source of truth). Phase files win when they disagree with the tasklist.

## Status Vocabulary

The Feature Phases table in `docs/tasklist.md` uses **plain-text** statuses — never emoji. Permitted values: `Complete`, `In Progress`, `Pending`. Progress is `<done>/<total>` (e.g. `3/5`). See `../_shared/status-markers.md` for the full registry.

## Steps

### Step 1 — Read the global tasklist

Read `docs/tasklist.md`. Locate the **Feature Phases** table.

### Step 2 — Find existing phase files

Glob `docs/phase/phase-[0-9]*.md`.

### Step 3 — Sync phase files → tasklist

For each existing `docs/phase/phase-N.md`:

1. Read it; count `[x]` and `[ ]` tasks.
2. **If all `[x]`:**
   - In `docs/tasklist.md`, set the row's status to `Complete` and progress to `<total>/<total>`.
   - Sync each task checkbox in the per-phase section of `docs/tasklist.md` to `[x]`.
3. **If partially complete:**
   - Set status to `In Progress` and progress to `<done>/<total>`.
   - Sync each task checkbox in `docs/tasklist.md` to mirror the phase file.

### Step 4 — Find the first incomplete phase

Scan the Feature Phases table top-to-bottom. The **first** row whose status is not `Complete` is `CURRENT_PHASE`.

If every row is `Complete`, set `CURRENT_PHASE = none` and proceed to Step 7.

### Step 5 — Materialize the phase file if missing

If `docs/phase/phase-CURRENT_PHASE.md` does **not** exist, create it from `docs/tasklist.md` using the layout in `../_shared/phase-file-template.md`. Extract:

- Title from the `## Phase N: <title>` heading.
- Goal from `**Goal:**` (or derive a one-line goal from the phase description if missing).
- Tasks from the `- [ ] N.x …` lines in that phase's section.
- Acceptance criteria from `**Test:**` or `**Verify:**` lines.
- Dependencies from the Feature Phases table's `Depends on` column.

### Step 6 — Update the Current Phase pointer

Update the `**Current Phase:** <N>` line at the top of `docs/tasklist.md` to `CURRENT_PHASE`.

### Step 7 — Report

Print:
- Phases whose tasklist rows were updated.
- Phase file created (if any).
- Current phase number (or `all complete`).
- Next action: which phase to work on next, or "all phases complete".

## Rules

- Phase files are the source of truth for task completion within their phase. The tasklist mirrors them.
- Never delete or overwrite an existing `## Implementation Notes` section in a phase file.
- Preserve any extra sections an earlier author added (`## Open Questions`, `## Risks`, etc.).
- Multi-phase drift: if more than one phase is out of sync, this skill processes them in numeric order in a single pass.
