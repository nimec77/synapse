# Requirements Are Immutable

The PRD, plan, phase docs (`docs/phase/*.md`), and `docs/vision.md` are the **authoritative requirements**. A skill that creates, reviews, or implements work must implement those requirements as written — not adapt them to existing code.

## Forbidden

- Change a PRD/plan requirement to match what the code already does.
- Mark a task complete because existing code "works similarly".
- Document a "design decision" that overrides a stated requirement without explicit user approval.
- Reinterpret a technical specification (e.g. silently downgrading "UUID v8" to "UUID v4").
- Omit a requirement from a PRD, plan, tasklist, or implementation.

## Required

- If existing code contradicts a requirement: flag it as a **DEVIATION**, and create tasks to bring the code in line.
- If a requirement is ambiguous or seems infeasible: invoke `AskUserQuestion` before assuming an answer.
- Tests must verify the **required** behavior, not the current behavior.
- Acceptance criteria must restate the requirement, not paraphrase it loosely.

## Example

**Wrong:** "Requirements mention UUID v8, but implementation uses UUID v4. This is acceptable because v4 works."

**Right:** "DEVIATION: Requirements specify UUID v8 (`docs/phase/phase-8.md`), implementation uses v4. Plan adds task to switch the `uuid` crate feature and migrate `Session::new()` to `Uuid::new_v8()`."

## When the user provides a `DESCRIPTION_FILE`

A description file passed alongside the ticket ID (e.g. `phase-12.md`) is **also authoritative**. Copy its specifications verbatim into the PRD; do not paraphrase or reinterpret. If the description file conflicts with existing code, the description wins and the gap becomes work.
