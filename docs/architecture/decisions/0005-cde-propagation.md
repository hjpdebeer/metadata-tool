# ADR-0005: Automatic CDE Designation and Classification Propagation

**Status:** Accepted (amended 2026-03-21)

**Date:** 2026-03-18 (amended 2026-03-21)

## Context

BCBS 239 (Principles for Effective Risk Data Aggregation and Risk Reporting) requires that data supporting critical business processes is identified and managed as **Critical Data Elements (CDEs)**. CDEs receive heightened governance: stricter quality rules, mandatory lineage documentation, and priority remediation.

The metadata tool maintains a three-level hierarchy:
- **Business Glossary Terms** — business concepts, designated as **CBT** (Critical Business Term)
- **Data Elements** — logical data specifications, designated as **CDE** (Critical Data Element)
- **Technical Columns** — physical database columns linked to data elements

CBT and CDE designations, along with data classification levels, should propagate automatically through the linkage chain.

## Decision

Implement **database triggers** that automatically propagate CDE designation and classification based on glossary term and process linkage. This is **auto-acceptance via inheritance** — it intentionally bypasses the review/approval workflow because the Owner already approved the data element and its linkage to the glossary term, so the inherited properties are aligned with that governance decision.

### Propagation Rule 1: CBT → CDE

When a glossary term is a Critical Business Term (CBT), all data elements linked to it via `glossary_term_id` automatically become Critical Data Elements (CDEs):
- `is_cde` is set to `true`
- `cde_rationale` is populated with: "Auto-designated: inherited from Critical Business Term (ADR-0005)"
- `cde_designated_at` is set to the current timestamp

This fires in two scenarios:
- **Element links to a CBT:** When `data_elements.glossary_term_id` is set to a term where `is_cbt = true`
- **Term becomes a CBT:** When `glossary_terms.is_cbt` changes from false to true, all currently linked elements are updated

When a term loses CBT status, auto-designated CDEs (identified by rationale containing "Auto-designated") are automatically un-designated. Manually designated CDEs are not affected.

### Propagation Rule 2: Classification Inheritance

When a glossary term has a `classification_id` (e.g., Confidential, Restricted), linked data elements inherit it:
- If the element has no classification, it inherits the term's classification
- If the term's classification changes, elements that had the old classification are updated to the new one
- Elements with independently set classifications are not overwritten

### Propagation Rule 3: Critical Process → CDE (existing)

When a business process is marked as critical, all data elements linked via `process_data_elements` are designated as CDEs. This is the original ADR-0005 trigger and remains unchanged.

### Auto-Acceptance via Inheritance

These propagation rules bypass the workflow intentionally:
- The data element's linkage to a glossary term was approved through the standard Steward → Owner workflow
- The glossary term's CBT designation was approved through its own workflow
- Therefore, the inherited CDE status is a logical consequence of two already-approved governance decisions
- No additional approval is required for inherited properties

### Manual Override

Data elements can still be manually designated as CDEs or have classifications set independently of inheritance. Manual designations are not overwritten by the triggers. The triggers only act on auto-designated values (identified by the rationale text) and null classifications.

## Consequences

### Positive

- **Complete CDE coverage:** Every data element linked to a CBT is guaranteed to be a CDE
- **Classification consistency:** Elements inherit the classification of their business concept
- **Cannot be bypassed:** Database triggers execute regardless of which path modifies the data (UI, API, bulk upload, ingestion)
- **Traceable rationale:** Auto-generated `cde_rationale` documents the inheritance chain
- **Immediate propagation:** Changes propagate in the same transaction
- **Governance-aligned:** Auto-acceptance is justified by prior workflow approvals

### Negative

- **Trigger complexity:** Developers must be aware of the triggers when reasoning about state changes
- **Bulk impact:** Marking a term as CBT triggers updates across all linked elements in one transaction

### Neutral

- **Dual path:** CDEs can be designated both automatically (via CBT/process linkage) and manually (via direct designation). Both are valid.
