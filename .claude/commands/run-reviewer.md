---
description: "Review changes for a ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, rust-analyzer-lsp, AskUserQuestion
model: inherit
---

Use the `reviewer` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## Run-reviewer Steps

### Step 1: Gather Context

Read:
- `docs/prd/$1.prd.md`
- `docs/plan/$1.md`
- `docs/tasklist/$1.md`
- `docs/conventions.md`

### Step 2: Analyze Changes

Analyze the diff for changes related to ticket `$1` (use `git diff` or check modified files).

### Step 3: Generate Review

Generate a structured review with three categories:

1. **Blocking** - Issues that MUST be fixed before merging (security issues, breaking bugs, missing required functionality)
2. **Important** - Recommended fixes that improve quality (safety comments, validation, edge cases)
3. **Cosmetic** - Minor improvements (naming, style, test coverage for edge cases)

### Step 4: Handle Review Results

#### If Blocking Issues Found

1. Report: "REVIEW_BLOCKED: The following issues must be fixed before merging:"
2. List all blocking issues
3. Add tasks to `docs/tasklist/$1.md` under a new section `## Review Fixes`:
   - Each blocking issue becomes a task with `- [ ]`
   - Include acceptance criteria for each task
4. Update the tasklist status from `IMPLEMENT_STEP_OK` to `REVIEW_BLOCKED`
5. Terminate with message: "Review blocked. New tasks added to tasklist for required fixes."

#### If Only Non-Blocking Issues Found (Important and/or Cosmetic)

1. Report the review findings
2. Use AskUserQuestion to ask the user:
   - Question: "Non-critical issues were found during code review. Would you like to fix them?"
   - Header: "Review Fixes"
   - Options:
     - Label: "Fix all issues", Description: "Add tasks for all important and cosmetic issues"
     - Label: "Fix important only", Description: "Add tasks for important issues, skip cosmetic"
     - Label: "Skip fixes", Description: "Proceed without fixing non-critical issues"

3. Based on user response:

   **If "Fix all issues" or "Fix important only":**
   - Add tasks to `docs/tasklist/$1.md` under a new section `## Review Fixes`:
     - Each selected issue becomes a task with `- [ ]`
     - Include acceptance criteria for each task
   - Update the tasklist status from `IMPLEMENT_STEP_OK` to `REVIEW_NEEDS_FIXES`
   - Report: "REVIEW_NEEDS_FIXES: Added N tasks to tasklist for review fixes."

   **If "Skip fixes":**
   - Report: "REVIEW_OK: Non-critical issues acknowledged but skipped."
   - Do not modify the tasklist

#### If No Issues Found

1. Report: "REVIEW_OK: No issues found. Code is ready for QA."

### Step 5: Return Status

The command must clearly output one of these statuses at the end:
- `REVIEW_OK` - No fixes needed, proceed to QA
- `REVIEW_NEEDS_FIXES` - Non-critical fixes requested, new tasks added
- `REVIEW_BLOCKED` - Critical fixes required, new tasks added

## Task Format

When adding tasks to the tasklist, use this format:

```markdown
## Review Fixes

- [ ] **RF1: [Short description of the fix]**
  - [Details of what needs to be changed]
  - **Acceptance:** [Criteria for completion]
```

Number tasks sequentially (RF1, RF2, etc.) to distinguish them from original implementation tasks.
