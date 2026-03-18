# ADR-0003: Generic Workflow Engine for Metadata Governance

**Status:** Accepted

**Date:** 2026-03-18

## Context

All metadata entities in the platform require governed approval workflows:

- **Glossary terms** must be reviewed and approved before becoming the authoritative definition.
- **Data elements** need sign-off from data owners before publication.
- **Quality rules** require validation and approval before enforcement.
- **Applications and processes** need governance review when registered or modified.

Each entity type may have slightly different workflow configurations (e.g., different approvers, different required reviews), but the fundamental lifecycle is the same. Building a separate workflow for each entity type would lead to duplicated logic and inconsistent governance.

## Decision

Implement a **generic workflow engine** that is entity-type-agnostic. The engine is driven by database-defined configuration, not hard-coded logic.

### Core tables

| Table | Purpose |
|-------|---------|
| `workflow_definitions` | Named workflow templates (e.g., "Standard Approval", "Expedited Review") |
| `workflow_entity_types` | Maps entity types to their applicable workflow definition |
| `workflow_states` | Defines the states within each workflow definition |
| `workflow_transitions` | Defines allowed state transitions and required roles/permissions |
| `workflow_instances` | Tracks a specific entity's progress through a workflow |
| `workflow_tasks` | Individual review/approval tasks assigned to users |

### Standard state machine

```
Draft --> Proposed --> Under Review --> Accepted --> Deprecated
                            |
                            +--> Revised (returns to Under Review)
                            |
                            +--> Rejected
```

- **Draft:** Initial creation, editable by the author.
- **Proposed:** Submitted for review, no longer editable without revision.
- **Under Review:** Assigned to reviewers, active review in progress.
- **Accepted:** Approved and published as authoritative metadata.
- **Revised:** Returned for changes, goes back to Under Review after edits.
- **Rejected:** Declined, with rationale recorded.
- **Deprecated:** Previously accepted, now superseded or retired.

### Notifications

Email notifications are triggered on state transitions. The notification content and recipients are determined by the transition definition (e.g., "on transition to Under Review, notify all assigned reviewers").

## Consequences

### Positive

- **Adding workflow to a new entity type requires only a database row** in `workflow_entity_types`, not code changes. The engine handles the rest.
- **Consistent governance:** All entity types follow the same approval patterns, making the process predictable for users.
- **Auditable:** Every state transition is recorded with timestamp, actor, and comment. Full history is preserved.
- **Configurable:** Different entity types can use different workflow definitions with different states or transition rules if needed.

### Negative

- **Generic complexity:** A generic engine is more complex to implement than a simple status field on each entity. The abstraction must be justified by the number of entity types using it.
- **Query complexity:** Fetching "all pending approvals for a user" requires joining through the workflow tables rather than a simple status filter on a single table.

### Neutral

- **State machine enforcement:** The engine enforces valid transitions. An entity cannot jump from Draft to Accepted without passing through the required intermediate states.
