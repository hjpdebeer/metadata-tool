# ADR-0001: Modular Monolith Architecture

**Status:** Accepted

**Date:** 2026-03-18

## Context

We need to choose an architectural style for the metadata management platform. The key factors are:

- **Purpose:** The tool is a metadata management platform for financial services, not a high-throughput transaction system. Request volumes are modest (governance workflows, metadata lookups, occasional bulk imports).
- **Team size:** The codebase is primarily AI-maintained with a small human team. Operational complexity must be minimized.
- **Deployment:** The tool should be simple to deploy and operate. Managing multiple services, service meshes, and distributed tracing adds overhead that is not justified by the workload.
- **Domain complexity:** The domain has multiple bounded contexts (glossary, data dictionary, data quality, lineage, applications, processes, workflow) that need clear separation, but they share a single database and frequently reference each other (e.g., lineage references data elements, workflow spans all entity types).

## Decision

Adopt a **modular monolith** architecture with clear domain boundaries enforced by Rust's module system. The system compiles into a **single deployable binary**.

Domain modules:

| Module | Responsibility |
|--------|---------------|
| `glossary` | Business term definitions, classifications, relationships |
| `data_dictionary` | Data elements, CDE designations, element-to-term mappings |
| `data_quality` | Quality rules, dimensions, rule-to-element bindings |
| `lineage` | Data flow mappings between systems and elements |
| `applications` | Application/system registry, technical metadata |
| `processes` | Business process registry, criticality, element linkage |
| `workflow` | Generic governance workflow engine |

Each module exposes a well-defined public interface (Rust `pub` items) and keeps internal implementation private. Cross-module dependencies go through these interfaces, not by reaching into internal types.

## Consequences

### Positive

- **Simple deployment:** One binary, one database, one configuration file. No container orchestration required.
- **Easier debugging:** A single process with a single log stream. No distributed tracing needed.
- **Compile-time enforcement:** Rust's module visibility rules enforce domain boundaries at compile time. Accidental coupling is a compiler error, not a runtime surprise.
- **Single database:** All modules share one PostgreSQL database. Foreign keys across domain tables are straightforward. No eventual consistency headaches.
- **Lower operational cost:** No service discovery, no inter-service auth, no network partitioning concerns.

### Negative

- **Scaling is all-or-nothing:** The entire binary scales together. If one module needs more resources, all modules get them.
- **Deployment couples all modules:** A change to one module requires redeploying the entire binary. In practice, this is acceptable given the low deployment frequency.

### Neutral

- **Decomposable later:** If scale demands it, individual modules can be extracted into separate services. The clean interfaces make this feasible without a rewrite.
