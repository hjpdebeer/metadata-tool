# ADR-0005: Automatic CDE Designation from Critical Business Processes

**Status:** Accepted

**Date:** 2026-03-18

## Context

BCBS 239 (Principles for Effective Risk Data Aggregation and Risk Reporting) requires that data supporting critical business processes is identified and managed as **Critical Data Elements (CDEs)**. CDEs receive heightened governance: stricter quality rules, mandatory lineage documentation, and priority remediation.

In practice, manual CDE designation is:

- **Error-prone:** Data stewards forget to designate elements when a process becomes critical.
- **Incomplete:** New data elements linked to an already-critical process are not automatically flagged.
- **Inconsistent:** Different teams apply different thresholds for CDE designation.

The relationship between critical processes and CDEs is deterministic: if a business process is critical, then all data elements supporting that process are CDEs by definition (per BCBS 239). This relationship should be enforced automatically.

## Decision

Implement **database triggers** that automatically propagate CDE designation based on critical business process linkage. Two triggers handle the two scenarios:

### Trigger 1: Process becomes critical

When a business process is marked as critical (`is_critical = true`), all data elements currently linked to that process via `process_data_elements` are automatically designated as CDEs:

- `is_cde` is set to `true`
- `cde_rationale` is populated with: "Auto-designated: linked to critical business process '{process_name}'"
- `cde_designated_at` is set to the current timestamp
- `cde_designated_by` records the system as the actor

### Trigger 2: Element linked to a critical process

When a data element is linked to a business process (via `process_data_elements`) and that process is already critical, the element is automatically designated as a CDE with the same fields populated.

### Manual designation

Data elements can also be manually designated as CDEs independent of process linkage. Manual designation sets the same fields but with a user-provided rationale. The triggers do not overwrite manual designations.

## Consequences

### Positive

- **Complete CDE coverage:** Every data element linked to a critical process is guaranteed to be designated as a CDE. No elements fall through the cracks.
- **Cannot be bypassed:** Database triggers execute regardless of which application path creates or modifies the data. There is no code path that can skip CDE propagation.
- **Traceable rationale:** The auto-generated `cde_rationale` documents exactly why an element was designated, linking it back to the specific critical process.
- **Immediate propagation:** CDE designation happens in the same transaction as the triggering change. There is no eventual consistency delay.

### Negative

- **Trigger complexity:** Database triggers are less visible than application code. Developers must be aware of the triggers when reasoning about data element state changes.
- **Bulk operations:** Marking a process with many linked elements as critical triggers CDE designation for all of them in a single transaction, which could be slow for very large linkage sets.
- **No automatic un-designation:** If a process is later marked as non-critical, the triggers do not automatically remove CDE designation from linked elements. Un-designation is a deliberate governance decision that requires human review.

### Neutral

- **Dual path:** CDEs can be designated both automatically (via process linkage) and manually (via direct designation). Both paths are valid and produce the same result in the data model.
