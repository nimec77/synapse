---
description: "Create the architecture and implementation plan for the ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, rust-analyzer-lsp, AskUserQuestion
model: opus
---

Use the `planner` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

## CRITICAL: REQUIREMENTS ARE IMMUTABLE

**YOU MUST NEVER MODIFY, REINTERPRET, OR CONTRADICT REQUIREMENTS.**

The PRD, phase documentation, and vision.md contain the **authoritative requirements**. Your job is to create a plan that IMPLEMENTS those requirements exactly as specified.

### Forbidden Actions

- ❌ Creating a plan that deviates from stated requirements
- ❌ Justifying existing code that contradicts requirements
- ❌ Documenting "design decisions" that override requirements without explicit user approval
- ❌ Marking implementation as "complete" when it doesn't match requirements

### Required Actions

- ✅ If existing code contradicts requirements: Plan must include tasks to FIX the code
- ✅ If requirements conflict with each other: Use AskUserQuestion for clarification
- ✅ If a requirement seems technically infeasible: Use AskUserQuestion before planning alternatives
- ✅ Plan must deliver exactly what the requirements specify

### Example

**WRONG:**
> "Design Decision: Use UUID v4 instead of v8 because v4 is simpler."

**CORRECT:**
> "Requirement: UUID v8 (per docs/phase/phase-8.md). Plan: Update uuid crate to use v8 feature, modify Session::new() to use Uuid::new_v8()."

---

## Planner steps

1. Read:
- `docs/prd/$1.prd.md`,
- `docs/research/$1.md` (if available),
- `docs/conventions.md`.
2. **Verify alignment**: Check that research document flags any deviations from requirements
3. Create or update `docs/plan/$1.md` with the following structure:
- Components
- API contract
- Data flows
- NFR
- Risks
- **Deviations to fix** (if research identified any)
- Open questions (if any).
4. If there are architectural alternatives, create `docs/adr/$1.md` with the options and the chosen solution.
5. **The plan must implement ALL requirements exactly as specified.** If existing code deviates, the plan must include tasks to correct it.
6. If the plan is approved, set the line `Status: PLAN_APPROVED` in `docs/plan/$1.md`.
