---
description: "Gather technical context and create a research document for the ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, AskUserQuestion, rust-analyzer-lsp
model: opus
---

Use the `researcher` subagent.

## Ticket Resolution

If the ticket ID is not provided as a parameter (`$1` is empty):
1. Read the file `docs/.active_ticket`
2. Use the first non-empty line as the ticket ID
3. If the file does not exist or contains no valid ticket ID, display an error message: "Error: No ticket specified. Provide a ticket ID as a parameter or set it in docs/.active_ticket" and terminate immediately.

---

## MANDATORY: INVOKE AskUserQuestion TOOL FIRST

**THIS IS A BLOCKING REQUIREMENT. YOU MUST NOT SKIP THIS STEP.**

Before doing ANY research or writing ANY document:

1. Read the PRD file `docs/prd/$TICKET.prd.md`
2. Find the "Open Questions" section
3. **INVOKE the AskUserQuestion tool** - DO NOT output questions as text!

### CRITICAL RULE

**NEVER output questions as plain text and wait. ALWAYS invoke the AskUserQuestion tool.**

- WRONG: Writing "Do you have any preferences?" as text output
- CORRECT: Calling AskUserQuestion tool with questions array

### What to Ask

**IF PRD has Open Questions:**
- Invoke AskUserQuestion for EVERY question listed
- DO NOT skip any questions
- DO NOT guess or assume answers

**IF PRD has NO Open Questions:**
- Still invoke AskUserQuestion with this question:
  - question: "Are there any implementation details, constraints, or preferences I should know before researching this ticket?"
  - header: "Preferences"
  - options: [{"label": "Use defaults", "description": "Proceed with documented requirements only"}, {"label": "I have specifics", "description": "I will provide additional details"}]

**WHY THIS IS CRITICAL:**
- Only the user knows the correct implementation approach
- Guessing leads to WRONG research and WRONG implementation
- The user MUST validate direction before work begins

**FAILURE TO INVOKE AskUserQuestion = INCORRECT WORK**

---

## CRITICAL: REQUIREMENTS ARE IMMUTABLE

**YOU MUST NEVER MODIFY, REINTERPRET, OR CONTRADICT REQUIREMENTS.**

The PRD and any referenced documentation (e.g., `docs/phase/*.md`, `docs/vision.md`) contain the **authoritative requirements**. Your job is to research how to IMPLEMENT those requirements, NOT to change them.

### Forbidden Actions

- ❌ Changing PRD requirements to match existing code
- ❌ Reporting "implementation uses X" as justification to ignore requirement Y
- ❌ Documenting existing code behavior as "resolved" when it contradicts requirements
- ❌ Suggesting alternatives that contradict stated requirements without explicit user approval

### Required Actions

- ✅ If existing code contradicts requirements: Flag as "DEVIATION FROM REQUIREMENTS" and list what needs to change
- ✅ If requirements seem infeasible: Use AskUserQuestion to get explicit approval before any deviation
- ✅ Always treat documented requirements as the source of truth
- ✅ Document gaps between current implementation and requirements

### Example

**WRONG:**
> "Requirements mention UUID v8, but implementation uses UUID v4. This is acceptable because v4 works."

**CORRECT:**
> "DEVIATION: Requirements specify UUID v8 (docs/phase/phase-8.md), but current implementation uses UUID v4. Implementation must be updated to use UUID v8."

---

## Research Steps (ONLY AFTER AskUserQuestion returns answers)

1. Read `docs/prd/$1.prd.md` and incorporate user answers
2. Scan key project directories (src, docs, configs) for entities and modules related to the ticket
3. Document the following in `docs/research/$1.md`:
   - existing endpoints and contracts,
   - layers and dependencies,
   - patterns used,
   - limitations and risks,
   - resolved questions (with user answers),
   - any NEW technical questions discovered during research,
   - **DEVIATIONS: any places where existing code contradicts requirements**
4. Do not change the code; only gather information
5. **Do not modify the PRD or any requirements documents**
