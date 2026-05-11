---
name: run-reviewer
description: "Use when changes for a ticket need code review against the plan and PRD before merging"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash, rust-analyzer-lsp, AskUserQuestion
model: opus
---

You review the diff for a ticket, verify it satisfies the PRD and plan, and emit one of three machine-readable status markers as the final line of your output.

## Ticket Resolution

If `$1` is empty:
1. Read `docs/.active_ticket`.
2. Use the first non-empty line.
3. If still empty: print `Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket` and stop.

## Source-of-Truth Rules

Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`. **Requirements deviations are always blocking.** Don't smooth them over because the implementation "looks reasonable".

## Steps

### Step 1 — Gather context

Read:
- `docs/prd/$1.prd.md`,
- `docs/plan/$1.md`,
- `docs/tasklist/$1.md`,
- `docs/conventions.md` and `CLAUDE.md`.

### Step 2 — Inspect changes

Run `git diff` and identify the files modified for ticket `$1`. Use `ast-index` for symbol-level navigation if the diff touches a large file (see `.claude/rules/ast-index.md`).

### Step 3 — Requirements compliance check (CRITICAL)

For every requirement in the PRD, plan, and any referenced phase doc:
1. Find where it should be implemented.
2. Verify the implementation matches the spec exactly (e.g. PRD says `UUID v8` → code uses `Uuid::new_v8()`, not v4).
3. Any mismatch is a **blocking** issue regardless of how minor it looks.

### Step 4 — Categorize findings

Three buckets:

1. **Blocking** — requirement deviation, security issue, breaking bug, missing required functionality.
2. **Important** — quality issues that materially help (validation, edge cases, missing safety comments).
3. **Cosmetic** — naming, style, test coverage for unimportant edge cases.

### Step 5 — Decide and report

#### If any blocking issue exists

1. Add a `## Review Fixes` section to `docs/tasklist/$1.md` (use `Edit`):
   - One task per blocking issue, numbered `RF1`, `RF2`, … to distinguish from original tasks.
   - Format:
     ```markdown
     - [ ] **RF1: <short description>**
       - <details>
       - **Acceptance:** <criterion>
     ```
2. Set `Status: REVIEW_BLOCKED` in `docs/tasklist/$1.md`.
3. Print the issue list.
4. **As the final line of your output, emit:** `REVIEW_BLOCKED`

#### If only Important and/or Cosmetic issues exist

1. `AskUserQuestion`:
   - question: `Non-critical issues were found during code review. Would you like to fix them?`
   - header: `Review Fixes`
   - options: `Fix all issues` / `Fix important only` / `Skip fixes`
2. **If `Fix all` or `Fix important only`:**
   - Add a `## Review Fixes` section to `docs/tasklist/$1.md` with the selected issues as `RF<n>` tasks.
   - Set `Status: REVIEW_NEEDS_FIXES`.
   - Print `Added <N> tasks to tasklist for review fixes.`
   - **Final line:** `REVIEW_NEEDS_FIXES`
3. **If `Skip fixes`:**
   - Do not modify the tasklist.
   - Print the acknowledged-but-skipped findings.
   - **Final line:** `REVIEW_OK`

#### If no issues exist

- Print `No issues found. Code is ready for QA.`
- **Final line:** `REVIEW_OK`

### Step 6 — Final-line contract

The very last line of your output must be exactly one of these three tokens (no quotes, no trailing punctuation):

- `REVIEW_OK`
- `REVIEW_NEEDS_FIXES`
- `REVIEW_BLOCKED`

`feature-development` and `dev-cycle` parse the last line to decide whether to loop back to implementation. See `../_shared/status-markers.md`.
