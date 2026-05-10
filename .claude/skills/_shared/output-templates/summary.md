# Ticket Summary Template

Produced by `docs-update` at `docs/summaries/$TICKET_ID-summary.md`.

```markdown
# <TICKET_ID>: <feature title>

**Shipped:** <YYYY-MM-DD>
**Version:** <X.Y.Z> (if released as part of a version bump)

## What Shipped

<2-3 sentence summary of the user-facing change.>

## How It Works

<Brief technical description: which crates and modules changed, which traits/types were added or modified, any new public API.>

## Decisions

- **<decision>** — <reasoning>. <Link to ADR if one exists.>

## Files Changed

| File | Change |
|---|---|
| `<path>` | <add | modify | delete> |

## Follow-ups

<Anything intentionally deferred: open ADR questions, known limitations to address in a future ticket, etc. Empty if none.>
```

## Rules

- The summary is the durable record once the PRD/plan/tasklist age out. Make it self-contained.
- Always add a corresponding `## [Unreleased]` entry to `CHANGELOG.md` when the summary is written. Format: `- <one-line user-facing description> (<TICKET_ID>)`.
- Do not include implementation noise (commit count, lines changed) unless it affects users.
