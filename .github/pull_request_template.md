## Summary

-

## Design Workflow

Which phase(s) does this PR address?

- [ ] **Phase 1: Business Process** -- Business process modelled/updated before any data or code changes
- [ ] **Phase 2: Data Architecture** -- Data elements, schemas, or metadata definitions created/updated
- [ ] **Phase 3: Technology/Code** -- Implementation of approved business process and data architecture
- [ ] **N/A** -- Infrastructure, CI/CD, documentation-only, or other non-workflow change

## Data Architecture

- [ ] All new data elements have complete metadata (description, type, constraints, lineage, owner)
- [ ] Naming standards followed (see `NAMING_STANDARDS.md`)
- [ ] No duplication of existing data elements -- reuse or extend instead
- [ ] Critical Data Element (CDE) impact assessed -- propagation rules reviewed if CDEs are added or modified

## Principles Compliance

Confirm this PR adheres to the project's 14 governing principles (`METADATA_TOOL_PRINCIPLES.md`):

- [ ] **P1 API-First** -- All functionality exposed via versioned API; no UI-only paths
- [ ] **P2 Metadata-Described Everything** -- Every data element, process, and artefact has machine-readable metadata
- [ ] **P3 Data Quality-By-Design** -- Validation rules defined at the metadata layer, not bolted on after the fact
- [ ] **P4 Design-First Workflow** -- Business process designed before data architecture, data architecture before code
- [ ] **P5 Workflow-Governed Metadata** -- Metadata changes go through approval workflows with full audit trail
- [ ] **P6 AI-Assisted, Human-Governed** -- AI may draft or suggest; humans approve all changes to production metadata
- [ ] **P7 Rust Only (Backend)** -- Backend code is Rust; no other backend languages introduced
- [ ] **P8 Naming Standards Enforcement** -- Identifiers conform to the naming standard and are validated programmatically
- [ ] **P9 Audit Everything** -- All state changes produce immutable audit records (who, what, when, why)
- [ ] **P10 Secure-by-Design** -- Authentication, authorisation, encryption, and least-privilege enforced by default
- [ ] **P11 Single Source of Truth** -- Each data element has exactly one authoritative source; no shadow copies
- [ ] **P12 Critical Data Element Propagation** -- CDE changes propagate to all dependent systems with lineage tracking
- [ ] **P13 AI-Maintained Codebase** -- Code is structured for AI readability: clear naming, small modules, rich doc comments
- [ ] **P14 Bank-Agnostic Design** -- No institution-specific logic in core; all bank-specific behaviour is configuration

## Security Compliance

- [ ] No raw SQL interpolation -- all queries use parameterised statements or the query builder
- [ ] No PII or sensitive data written to logs
- [ ] All new HTTP endpoints require authentication and authorisation
- [ ] Cryptographic comparisons use constant-time functions
- [ ] Error responses do not leak internal state, stack traces, or sensitive data

## Coding Standards Compliance

- [ ] **Naming** -- Types are `PascalCase`, functions/variables are `snake_case`, constants are `SCREAMING_SNAKE_CASE`
- [ ] **Module structure** -- New modules follow the established directory layout and re-export conventions
- [ ] **Type design** -- Domain concepts use newtypes or enums; no stringly-typed interfaces
- [ ] **Error handling** -- Errors use typed variants (`thiserror`); no `.unwrap()` in library/production code
- [ ] **Documentation** -- Public items have `///` doc comments explaining purpose, parameters, and examples
- [ ] **Tests** -- Unit tests for logic, integration tests for API endpoints; coverage does not regress

## Automated Checks

All of the following must pass before merge:

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo doc --workspace --no-deps`
- [ ] Frontend: `npm run build`

## Test Plan

-
