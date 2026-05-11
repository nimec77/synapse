# Research Template

Produced by `research` at `docs/research/$TICKET_ID.md`.

```markdown
# <TICKET_ID> Research

## Resolved Questions

<For each `Open Question` in the PRD, the answer the user provided via AskUserQuestion. Format: **Q:** <question> · **A:** <answer>.>

## Existing Code Surface

- **Modules touched:** <crate::module paths>
- **Public types involved:** <list>
- **Traits implemented:** <list>
- **Tests covering this area:** <list of test files / functions>

## Patterns in Use

<Patterns the implementation should follow: e.g. "providers extend `OpenAiCompatProvider` as thin wrappers", "config fields use serde defaults via free fns".>

## Limitations & Risks

- <limitation> — <impact on the planned implementation>

## New Technical Questions

<Questions that surfaced during research and weren't in the PRD. Flag for the user; the plan resolves them.>

## DEVIATIONS from Requirements

<Places where existing code contradicts PRD requirements. Each must be addressed in the plan's `Deviations to Fix` section. Delete this section if none.>
```

## Rules

- Research **never modifies** code — it only reads and reports.
- Research **never modifies** the PRD — see `_shared/requirements-immutability.md`.
- The "DEVIATIONS" section is the contract with the `plan` skill: every deviation listed here must become a task.
- Use `ast-index` for symbol/usage lookups before reading large files (see `.claude/rules/ast-index.md`).
