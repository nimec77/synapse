# `_shared/` — Cross-skill References

Shared doctrine, templates, and registries that multiple skills under `.claude/skills/` link to. The leading underscore is intentional: it sorts first in directory listings and makes clear this is **not** an invocable skill.

## Files

| File | Purpose | Linked from |
|---|---|---|
| `requirements-immutability.md` | The "PRD is the source of truth" doctrine | `analysis`, `research`, `plan`, `tasklist`, `implement-orchestrated`, `quick-implement`, `run-reviewer` |
| `status-markers.md` | Canonical list of every workflow status string | `validate`, `feature-development`, `dev-cycle`, `phase-loop`, every artifact-emitting skill |
| `subagent-registry.md` | Available subagents and the modern invocation pattern | every orchestrator that delegates work |
| `phase-file-template.md` | Layout of `docs/phase/phase-N.md` | `sync-phases`, `phase-loop` |
| `tasklist-template.md` | Layout of `docs/tasklist/$TICKET_ID.md` | `tasklist`, `implement-orchestrated`, `dev-cycle`, `phase-loop` |
| `output-templates/prd.md` | Layout of `docs/prd/$TICKET_ID.prd.md` | `analysis` |
| `output-templates/plan.md` | Layout of `docs/plan/$TICKET_ID.md` | `plan` |
| `output-templates/research.md` | Layout of `docs/research/$TICKET_ID.md` | `research` |
| `output-templates/qa-report.md` | Layout of `reports/qa/$TICKET_ID.md` | `qa` |
| `output-templates/summary.md` | Layout of `docs/summaries/$TICKET_ID-summary.md` | `docs-update` |

## How skills reference these

In a SKILL.md, link rather than re-inline:

```markdown
Apply the requirements-immutability rules — see `../_shared/requirements-immutability.md`.
```

When a template's structure changes, update the file here and every skill picks it up automatically.
