# Development Workflow

Rules for AI-assisted development of Synapse. Follow this process for every task.

---

## Core Principle

**One task at a time. Plan → Agree → Implement → Test → Commit → Next.**

---

## Workflow Steps

### 1. Read Context

Before starting any work:
- [ ] Read `tasklist.md` — find current phase and next unchecked task
- [ ] Read `vision.md` — understand architecture relevant to task
- [ ] Read `conventions.md` — review rules to follow

### 2. Propose Solution

Before writing any code:
- [ ] Explain what you will do (1-3 sentences)
- [ ] Show key code snippets or file structure
- [ ] List files to create/modify
- [ ] Ask: **"Proceed with this approach?"**

**Wait for user approval before implementing.**

### 3. Implement

After approval:
- [ ] Write code following `conventions.md`
- [ ] Keep changes minimal and focused
- [ ] Run `cargo check` to verify compilation

### 4. Verify

After implementation:
- [ ] Run `cargo test` — all tests must pass
- [ ] Run `cargo clippy -- -D warnings` — no warnings
- [ ] Demonstrate the feature works (show command + output)
- [ ] Ask: **"Ready to commit?"**

**Wait for user confirmation.**

### 5. Commit & Update

After confirmation:
- [ ] Commit with conventional message: `<type>: <description>`
- [ ] Update `tasklist.md`:
  - Mark task checkbox: `- [x]`
  - Update phase progress: `2/4` → `3/4`
  - If phase complete: change status to ✅
- [ ] Report: "Task X.Y complete. Next: Task X.Z"

### 6. Next Task

- [ ] Ask: **"Continue to next task?"**
- [ ] On approval, return to Step 1

---

## Decision Points

| Situation | Action |
|-----------|--------|
| Unclear requirement | Ask for clarification before proposing |
| Multiple valid approaches | Present options, recommend one, wait for choice |
| Task seems too large | Propose breaking into subtasks |
| Tests fail | Fix before asking to commit |
| Blocked by missing dependency | Report blocker, ask how to proceed |

---

## Commit Message Format

```
<type>: <short description>

Types: feat, fix, refactor, test, docs, chore
```

Examples:
- `feat: add Config struct with TOML parsing`
- `test: add unit tests for message serialization`
- `fix: handle missing API key gracefully`

---

## Checkpoints

Never skip these confirmations:

1. **Before implementing** — "Proceed with this approach?"
2. **Before committing** — "Ready to commit?"
3. **Before next task** — "Continue to next task?"

---

## Quick Reference

```
┌─────────────────────────────────────────────────────┐
│  READ → PROPOSE → [wait] → IMPLEMENT → VERIFY →    │
│  [wait] → COMMIT → UPDATE → [wait] → NEXT          │
└─────────────────────────────────────────────────────┘
```

**Three waits. Three confirmations. No skipping.**
