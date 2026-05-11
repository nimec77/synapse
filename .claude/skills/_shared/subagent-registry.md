# Subagent Registry

Only **`general-purpose`** is available in this project. There is no project-local `.claude/agents/` directory and no user-level `~/.claude/agents/` definitions for `coder`, `reviewer`, `code-writer`, `test-writer`, `analyst`, `planner`, `researcher`, `task-planner`, `qa`, `validator`, or `tech-writer`. References to those names in older skills were aspirational and resolve to nothing — silent breakage.

## The pattern

When a skill needs to delegate work to a subagent, use this exact form:

```
Agent(
  subagent_type: "general-purpose",
  model: "<opus|sonnet|haiku>",
  description: "<3-5 word task title>",
  prompt: "<self-contained brief>. Read '.claude/skills/<skill-name>/SKILL.md' for instructions. Arguments: <args>"
)
```

The subagent loads the named SKILL.md and follows it. The orchestrator gets a single result back.

## Model selection

Default mapping when migrating from named subagents:

| Old subagent name | Model | Used by |
|---|---|---|
| `analyst`, `planner`, `researcher`, `reviewer`, `code-writer`, `coder` | `opus` | analysis, plan, research, run-reviewer, implement-orchestrated, quick-implement |
| `task-planner`, `test-writer`, `qa`, `validator`, `tech-writer` | `sonnet` | tasklist, implement-orchestrated (test phase), qa, validate, docs-update |

If unsure: `opus` for design/review/critical-thinking work, `sonnet` for structured implementation/extraction work.

## Why this matters

A typo or a name from another project causes the harness to fall back silently — work appears to run, but with no specialization or instructions, and the result is unpredictable. Always use `general-purpose` plus a skill reference.

## Background tasks

Add `run_in_background: true` only when the orchestrator has independent work to do while the subagent runs. For sequential workflows (most of ours), omit it.
