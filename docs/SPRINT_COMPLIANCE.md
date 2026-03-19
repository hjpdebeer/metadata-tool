# Sprint: Codebase Compliance Refactoring

**Goal**: Bring all existing code into full compliance with METADATA_TOOL_PRINCIPLES.md, CODING_STANDARDS.md, and ADR-0006 (Standardised Data Access Patterns).

**Date**: 2026-03-19
**Estimated Effort**: 4-5 days

---

## Audit Summary

| Category | Violations | Severity |
|----------|-----------|----------|
| Data access pattern inconsistency (ADR-0006) | All domains | Must Fix |
| Safety (.unwrap/.expect in prod) | 3 | Must Fix |
| Error message casing | 11 variants | Must Fix |
| Missing crate/module docs | 15 files | Must Fix |
| Missing public type docs | ~40 types | Must Fix |
| Missing public function docs | ~80 functions | Must Fix |
| Missing derives (Clone/Deserialize) | ~15 types | Should Fix |
| Workflow constants undocumented | 12 constants | Should Fix |
| Dead code warning | 1 | Should Fix |

---

## Phase 1: Data Access Pattern Standardisation (ADR-0006)

### WI-C01: Establish Glossary as Reference Implementation
**Files**: `domain/glossary.rs`, `api/glossary.rs`, `api/ai.rs`
**What**: Refactor the glossary domain to implement ADR-0006 as the canonical pattern all other domains will follow.

**a) Detail Read Pattern** — Single flat struct, single JOIN query:
- Remove `#[serde(flatten)]` from `GlossaryTermDetailView`
- Replace with a single `GlossaryTermDetail` struct containing ALL 45 entity columns + ALL resolved lookup names + junction data as explicit fields (no nesting)
- Single SQL query with LEFT JOINs for all FK lookups (domain, category, term_type, unit_of_measure, classification, review_frequency, confidence_level, visibility, language, parent_term, owner, steward, domain_owner, approver)
- Separate queries only for junction arrays (regulatory_tags, subject_areas, tags, linked_processes)
- Use a `FromRow` struct for the joined query result, then combine with junction data

**b) Write Pattern** — Unified update accepting both IDs and display names:
- Create `resolve_lookup(pool, value, lookup_query) -> Option<Uuid>` helper: tries UUID parse first (UI dropdown sends ID), falls back to ILIKE name match (AI sends display name)
- Update the `update_term` handler to use `resolve_lookup` for all FK fields
- Glossary-specific lookup resolution queries:
  - term_type: `SELECT term_type_id FROM glossary_term_types WHERE type_name ILIKE $1`
  - unit_of_measure: `SELECT unit_id FROM glossary_units_of_measure WHERE unit_name ILIKE $1`
  - classification: `SELECT classification_id FROM data_classifications WHERE classification_name ILIKE $1`
  - review_frequency: `SELECT frequency_id FROM glossary_review_frequencies WHERE frequency_name ILIKE $1`
  - confidence_level: `SELECT confidence_id FROM glossary_confidence_levels WHERE level_name ILIKE $1`
  - visibility: `SELECT visibility_id FROM glossary_visibility_levels WHERE visibility_name ILIKE $1`
  - language: `SELECT language_id FROM glossary_languages WHERE language_name ILIKE $1`
  - parent_term: `SELECT term_id FROM glossary_terms WHERE term_name ILIKE $1 AND is_current_version = TRUE`

**c) AI Suggestion Application** — Route through unified write path:
- Refactor `apply_suggestion_to_entity` for glossary_term to use the same `resolve_lookup` and junction insert logic as the update handler
- Eliminate the separate code paths for text columns vs FK lookups vs junctions
- For junction fields (regulatory_tags, subject_areas, tags): parse comma-separated values, resolve each via lookup table, insert junction rows with ON CONFLICT DO NOTHING

**d) Frontend Alignment**:
- `GlossaryTermDetail` interface extends `GlossaryTerm` with resolved names (already done)
- Remove fallback logic in `GlossaryTermDetail.tsx` — only use `getTermDetail`
- Ensure create/update forms send UUIDs for dropdown fields

### WI-C02: Apply Pattern to Data Dictionary
**Files**: `domain/data_dictionary.rs`, `api/data_dictionary.rs`
**What**: Refactor `DataElementFullView` to follow the same flat struct + single JOIN query pattern.

### WI-C03: Apply Pattern to Data Quality
**Files**: `domain/data_quality.rs`, `api/data_quality.rs`

### WI-C04: Apply Pattern to Applications
**Files**: `domain/applications.rs`, `api/applications.rs`

### WI-C05: Apply Pattern to Processes
**Files**: `domain/processes.rs`, `api/processes.rs`

### WI-C06: Apply Pattern to Lineage
**Files**: `domain/lineage.rs`, `api/lineage.rs`

---

## Phase 2: Safety & Error Handling

### WI-C07: Fix Safety Violations
**Files**: `main.rs`, `config.rs`
**What**: Replace `.unwrap()` and `.expect()` in production code with proper `?` error propagation.
- `main.rs` — CORS `.unwrap()` → `?` with anyhow context
- `config.rs` — PORT `.expect()` → `.map_err()?`
- `config.rs` — JWT_EXPIRY_HOURS `.expect()` → `.map_err()?`

### WI-C08: Fix Error Message Casing
**File**: `error.rs`
**What**: Change all `#[error("...")]` messages to lowercase first word per CODING_STANDARDS Section 4.

---

## Phase 3: Documentation (Principle 13 — AI-Maintained Codebase)

### WI-C09: Add Crate-Level Documentation
**File**: `lib.rs`
**What**: Add `//!` crate doc comment explaining architecture, module responsibilities, and boundaries.

### WI-C10: Add Module-Level Documentation
**Files**: All `mod.rs` files (15 files)
**What**: Add `//!` module doc comments explaining responsibility, contents, and what is NOT in scope.

### WI-C11: Add Documentation to All Public Types
**Files**: All `domain/*.rs` files (11 files)
**What**: Add `///` doc comments to every public struct, enum, and field. ~40 types.

### WI-C12: Add Documentation to All Public Functions
**Files**: All `api/*.rs` files (12 files)
**What**: Add `///` doc comments to every public handler function. ~80 functions. Each doc includes:
- Purpose (one sentence)
- Principle references where applicable
- Auth requirements

### WI-C13: Document Workflow Constants
**File**: `workflow/mod.rs`
**What**: Add `///` doc comments to all 12 constants.

---

## Phase 4: Type Design & Cleanup

### WI-C14: Add Missing Derives to Response Types
**Files**: All `domain/*.rs` files
**What**: Add `Clone` and/or `Deserialize` to response types that are missing them.

### WI-C15: Fix Dead Code Warning
**File**: `notifications/mod.rs`
**What**: Fix the `display_name` field on `RecipientRow`.

### WI-C16: Verify Clippy Clean
**What**: Run `cargo clippy --all-targets -- -D warnings` and fix all warnings.

### WI-C17: Add Pre-Commit Script
**What**: Create `scripts/pre-commit.sh` with all verification commands.

---

## Acceptance Criteria

### Data Access Patterns (ADR-0006)
- [ ] Every detail endpoint uses a single JOIN query (no separate `resolve_name` calls)
- [ ] Every detail response is a flat struct (no `#[serde(flatten)]`)
- [ ] `resolve_lookup()` helper used for all FK field updates (accepts both UUID and display name)
- [ ] AI suggestion acceptance uses the same update path as UI edits
- [ ] Junction table writes use `ON CONFLICT DO NOTHING` for idempotency
- [ ] Frontend has one flat type per entity detail view

### Safety & Error Handling
- [ ] Zero `.unwrap()` or `.expect()` in production code paths
- [ ] All error messages start with lowercase

### Documentation (Principle 13)
- [ ] Every Rust source file has module-level documentation
- [ ] Every public type has a `///` doc comment
- [ ] Every public function has a `///` doc comment

### Type Design
- [ ] All response types derive `Debug, Clone, Serialize`
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo doc --workspace --no-deps` builds without warnings
- [ ] All 13+ tests still pass

---

## Execution Order

1. **WI-C01 first** — glossary as reference implementation (establishes the pattern)
2. **WI-C07-C08** — safety and error fixes (quick wins)
3. **WI-C02-C06** — apply pattern to remaining domains (parallel-able)
4. **WI-C09-C13** — documentation (can be done incrementally)
5. **WI-C14-C17** — cleanup and tooling (final pass)

## Out of Scope (Deferred)

- Adding new unit tests (separate sprint)
- Frontend TypeScript compliance review
- Newtype ID pattern (would require significant refactoring)
- Integration tests
