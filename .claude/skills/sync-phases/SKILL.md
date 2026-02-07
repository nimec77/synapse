---
description: "Sync phase completion status between tasklist.md and phase-*.md files"
allowed-tools: Read, Write, Glob, Grep
model: inherit
---

## Overview

This command synchronizes phase task completion between `docs/tasklist.md` and individual `docs/phase/phase-*.md` files.

## Steps

### Step 1: Read the global tasklist

Read `docs/tasklist.md` to understand the current state of all phases.

### Step 2: Find existing phase files

Use Glob to find all `docs/phase/phase-*.md` files (pattern: `docs/phase/phase-[0-9]*.md`).

### Step 3: Sync completed phases FROM phase files TO tasklist

For each existing `docs/phase/phase-N.md` file:

1. Read the phase file
2. Check if ALL tasks in that phase are marked complete (`- [x]`)
3. If the phase is complete in the phase file:
   - Update the corresponding tasks in `docs/tasklist.md` to `[x]`
   - Update the Feature Phases table: set status to `✅ Complete` and progress to `X/X`
4. If the phase is NOT complete but has some progress:
   - Sync individual task completion status to `docs/tasklist.md`
   - Update progress count in Feature Phases table (e.g., `2/4`)

### Step 4: Find the first incomplete phase

Scan `docs/tasklist.md` for the first phase where:
- Status is NOT `✅ Complete` in the Feature Phases table, OR
- Any task is marked `[ ]` (incomplete)

### Step 5: Extract incomplete phase to separate file

If a `docs/phase/phase-N.md` file does NOT exist for the first incomplete phase:

1. Extract from `docs/tasklist.md`:
   - Phase title (from `## Phase N: Title`)
   - Goal (from `**Goal:**` line)
   - All tasks for that phase (`- [ ] N.1 ...`, `- [ ] N.2 ...`, etc.)
   - Test/acceptance criteria (from `**Test:**` line)

2. Create `docs/phase/phase-N.md` with this structure:
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

   - Phase N-1 complete
   - [any other dependencies mentioned]

   ## Implementation Notes

   [Extract any implementation notes from tasklist.md for this phase, or leave placeholder]
   ```

### Step 6: Update Current Phase

Update the `**Current Phase:** N` line in `docs/tasklist.md` to reflect the first incomplete phase number.

### Step 7: Report

Output a summary:
- Which phases were synced
- Which phase file was created (if any)
- Current phase number
- Next actions needed

## Rules

- Phase files (`docs/phase/phase-N.md`) are the source of truth for task completion within that phase
- The tasklist.md Feature Phases table must stay in sync with actual task completion
- Never delete or overwrite existing implementation notes in phase files
- Preserve all formatting and extra sections in existing files
