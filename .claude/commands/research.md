---
description: "Gather technical context and create a research document for the ticket"
argument-hint: "[ticket-id]"
allowed-tools: Read, Write, Glob, Grep, AskUserQuestion, rust-analyzer-lsp
model: inherit
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

## Research Steps (ONLY AFTER AskUserQuestion returns answers)

1. Read `docs/prd/$1.prd.md` and incorporate user answers
2. Scan key project directories (src, docs, configs) for entities and modules related to the ticket
3. Document the following in `docs/research/$1.md`:
   - existing endpoints and contracts,
   - layers and dependencies,
   - patterns used,
   - limitations and risks,
   - resolved questions (with user answers),
   - any NEW technical questions discovered during research
4. Do not change the code; only gather information
