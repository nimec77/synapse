# Workflow Status Markers

Skills coordinate by writing/reading **status strings** in artifact files (`docs/prd/*.prd.md`, `docs/plan/*.md`, `docs/tasklist/*.md`) and by emitting them as the **last line** of an orchestrated subagent's output. Every marker below is part of a stable contract — do not rename, alias, or paraphrase. Orchestrators (`feature-development`, `dev-cycle`, `phase-loop`, `validate`) gate on these exact strings.

## Artifact statuses

Written as `Status: <MARKER>` inside the artifact file.

| Marker | Where it lives | Emitted by | Gates |
|---|---|---|---|
| `DRAFT` | `docs/prd/$1.prd.md` | `analysis` (when blocking questions remain) | none — informational |
| `PRD_READY` | `docs/prd/$1.prd.md` | `analysis` (when PRD is complete) | `research`, `plan` may proceed |
| `PLAN_APPROVED` | `docs/plan/$1.md` | `plan` | `tasklist` may proceed |
| `TASKLIST_READY` | `docs/tasklist/$1.md` | `tasklist` | `implement-orchestrated`, `dev-cycle` may proceed |
| `IMPLEMENT_STEP_OK` | `docs/tasklist/$1.md` | `implement-orchestrated` (when all tasks done) | `run-reviewer` may proceed |
| `REVIEW_BLOCKED` | `docs/tasklist/$1.md` | `run-reviewer` | implementation must run again |
| `REVIEW_NEEDS_FIXES` | `docs/tasklist/$1.md` | `run-reviewer` | implementation must run again |
| `RELEASE_READY` | `docs/releases/R-*.md` | release coordinator | `release` may proceed |

## Orchestrated-output statuses

Emitted by review/QA/release subagents as the **last line of their output** so the calling orchestrator can branch.

| Marker | Emitted by | Calling skill branches on |
|---|---|---|
| `REVIEW_OK` | `run-reviewer` | continue to docs/QA |
| `REVIEW_NEEDS_FIXES` | `run-reviewer` | go back to implementation gate |
| `REVIEW_BLOCKED` | `run-reviewer` | go back to implementation gate |
| `RELEASE_PUBLISHED` | `release` | downstream automation can push |

## Phase-file checkbox markers

Inside `docs/phase/phase-N.md` and `docs/tasklist/$1.md`:

- `- [ ]` — task pending
- `- [x]` — task complete

These are the **only** in-file task markers. Do not use emoji (`:green_circle:`, `:white_circle:`) or YAML frontmatter for task state — they are fragile to encoding and editor rewriting. The Feature Phases table in `docs/tasklist.md` uses plain text statuses: `Complete` / `In Progress` / `Pending` with progress fractions `X/Y`.

## Adding a new marker

1. Pick a SCREAMING_SNAKE_CASE name that includes the verb noun pair (`THING_STATE`).
2. Add a row to one of the tables above.
3. Update every emitting skill and every gating skill in the same change.
