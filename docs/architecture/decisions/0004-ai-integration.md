# ADR-0004: External AI APIs for Metadata Enrichment

**Status:** Accepted

**Date:** 2026-03-18

## Context

Populating a metadata management platform with high-quality definitions, classifications, and descriptions is labor-intensive. Common challenges:

- **Knowledge gaps:** Data stewards may not know the precise financial services definition of a term or the applicable regulatory classification.
- **Inconsistency:** Different authors describe similar concepts in different ways, leading to fragmented metadata.
- **Standards alignment:** Financial services metadata should align with industry standards (BCBS 239, DAMA DMBOK, ISO 8000), but few practitioners have memorized these frameworks.

AI language models can assist by generating draft metadata that is grounded in domain knowledge, but AI-generated content must not bypass governance controls.

## Decision

Integrate **Claude API** (primary) and **OpenAI API** (fallback) for metadata enrichment suggestions. The integration follows a **human-in-the-loop** pattern:

1. **User requests a suggestion** (e.g., "suggest a definition for this glossary term" or "classify this data element").
2. **The system calls the AI API** with the entity context and relevant prompt.
3. **The AI response is stored** in the `ai_suggestions` table, linked to the target entity.
4. **The user reviews the suggestion** and can accept, modify, or reject it.
5. **User feedback is recorded** in the `ai_feedback` table for quality tracking.

### Key constraint

**AI never auto-publishes.** All AI-generated suggestions require explicit human acceptance before they become part of the authoritative metadata. There is no automated path from AI output to published metadata.

### Feedback loop

The `ai_feedback` table tracks:

- Which suggestions were accepted, modified, or rejected
- What modifications were made (diff between suggestion and final content)
- User ratings of suggestion quality

This data enables quality tracking over time and can inform prompt improvements.

## Consequences

### Positive

- **Reduced knowledge gaps:** AI can generate reasonable first drafts for definitions, classifications, and descriptions, lowering the barrier for metadata population.
- **Improved standardization:** AI suggestions grounded in financial services terminology promote consistency across the metadata catalog.
- **Quality tracking:** The feedback loop provides measurable data on AI suggestion quality over time.
- **Governance preserved:** The human-in-the-loop requirement ensures AI output is always reviewed before publication.

### Negative

- **External API dependency:** The suggestion feature depends on external API availability. If both Claude and OpenAI APIs are unavailable, suggestions cannot be generated (but all other platform functionality continues normally).
- **API costs:** Each suggestion incurs API usage costs. Cost controls (rate limiting, token budgets) are needed.
- **Prompt maintenance:** Prompt quality directly affects suggestion quality. Prompts must be maintained and improved over time.

### Neutral

- **Dual-provider strategy:** Claude API is primary, OpenAI is fallback. This reduces single-provider risk but requires maintaining two integration paths.
