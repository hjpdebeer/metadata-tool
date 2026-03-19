# Sprint: Codebase Compliance Refactoring

**Goal**: Bring all existing code into full compliance with METADATA_TOOL_PRINCIPLES.md and CODING_STANDARDS.md.

**Date**: 2026-03-19
**Estimated Effort**: 2-3 days

---

## Audit Summary

| Category | Violations | Severity |
|----------|-----------|----------|
| Safety (.unwrap/.expect in prod) | 3 | Must Fix |
| Error message casing | 11 variants | Must Fix |
| Missing crate/module docs | 15 files | Must Fix |
| Missing public type docs | ~40 types | Must Fix |
| Missing public function docs | ~80 functions | Must Fix |
| Missing derives (Clone/Deserialize) | ~15 types | Should Fix |
| Workflow constants undocumented | 12 constants | Should Fix |
| Dead code warning | 1 | Should Fix |

---

## Work Items

### WI-C01: Fix Safety Violations (Must Fix)
**Files**: `main.rs`, `config.rs`
**What**: Replace `.unwrap()` and `.expect()` in production code with proper `?` error propagation.
- `main.rs:212` — CORS `.unwrap()` → `?` with anyhow context
- `config.rs:49` — PORT `.expect()` → `.map_err()?`
- `config.rs:54` — JWT_EXPIRY_HOURS `.expect()` → `.map_err()?`

### WI-C02: Fix Error Message Casing (Must Fix)
**File**: `error.rs`
**What**: Change all `#[error("...")]` messages to lowercase first word per CODING_STANDARDS Section 4.
- "Not found" → "not found"
- "Bad request" → "bad request"
- "Unauthorized" → "unauthorized"
- All 11 AppError variants

### WI-C03: Add Crate-Level Documentation (Must Fix — Principle 13)
**File**: `lib.rs`
**What**: Add `//!` crate doc comment explaining architecture, module responsibilities, and boundaries.

### WI-C04: Add Module-Level Documentation (Must Fix — Principle 13)
**Files**: All `mod.rs` files (15 files)
**What**: Add `//!` module doc comments to:
- `api/mod.rs` — API handlers overview
- `domain/mod.rs` — Domain models overview
- `auth/mod.rs` — Authentication and JWT
- `workflow/mod.rs` — Workflow engine
- `ai/mod.rs` — AI integration
- `notifications/mod.rs` — Notification system
- `naming/mod.rs` — Naming standards
- `db/mod.rs` — Database connectivity
- All domain submodules

### WI-C05: Add Documentation to All Public Types (Must Fix — Principle 13)
**Files**: All `domain/*.rs` files (11 files)
**What**: Add `///` doc comments to every public struct, enum, and field that lacks one. ~40 types across:
- `domain/glossary.rs` — GlossaryTerm (45 fields), all lookup types, request/response types
- `domain/data_dictionary.rs` — DataElement, DataElementFullView, all request types
- `domain/data_quality.rs` — QualityRule, QualityDimension, assessment types
- `domain/lineage.rs` — LineageGraph, LineageNode, ImpactAnalysis
- `domain/applications.rs` — Application, ApplicationFullView
- `domain/processes.rs` — BusinessProcess, ProcessStep, CDE types
- `domain/workflow.rs` — WorkflowInstance, WorkflowTask, PendingTaskView
- `domain/users.rs` — User, Role, UserWithRoles
- `domain/notifications.rs` — InAppNotification, notification types
- `domain/ai.rs` — AiSuggestion, AiEnrichResponse

### WI-C06: Add Documentation to All Public Functions (Must Fix — Principle 13)
**Files**: All `api/*.rs` files (12 files)
**What**: Add `///` doc comments to every public handler function. ~80 functions across all API modules. Each doc should include:
- Purpose (one sentence)
- Principle references where applicable
- Note about auth requirements

### WI-C07: Add Missing Derives to Response Types (Should Fix)
**Files**: All `domain/*.rs` files
**What**: Add `Clone` and/or `Deserialize` to response types that are missing them:
- `PaginatedResponse<T>` — add Clone where T: Clone
- `PaginatedGlossaryTerms`, `PaginatedDataElements`, etc. — add Clone, Deserialize
- `GlossaryTermDetailView`, `DataElementFullView`, `ApplicationFullView`, etc.
- `DashboardStats`, `AiEnrichResponse`, `AiSuggestionResponse`, `FeedbackResponse`

### WI-C08: Document Workflow Constants (Should Fix)
**File**: `workflow/mod.rs`
**What**: Add `///` doc comments to all 12 constants explaining their purpose and usage.

### WI-C09: Fix Dead Code Warning (Should Fix)
**File**: `notifications/mod.rs`
**What**: Fix the `display_name` field on `RecipientRow` — either use it or remove it.

### WI-C10: Verify Clippy Clean (Should Fix)
**What**: Run `cargo clippy --all-targets -- -D warnings` and fix all warnings. Currently 1 dead_code warning.

### WI-C11: Add Pre-Commit Checklist Verification
**What**: Document the exact commands in a `scripts/pre-commit.sh`:
```bash
#!/bin/bash
set -e
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
cd frontend && npm run build
```

---

## Acceptance Criteria

- [ ] Zero `.unwrap()` or `.expect()` in production code paths
- [ ] All error messages start with lowercase
- [ ] Every Rust source file has module-level documentation
- [ ] Every public type has a `///` doc comment
- [ ] Every public function has a `///` doc comment
- [ ] All response types derive `Debug, Clone, Serialize`
- [ ] `cargo clippy --all-targets -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo doc --workspace --no-deps` builds without warnings
- [ ] All 13+ tests still pass

---

## Out of Scope (Deferred)

- Adding new unit tests (separate sprint)
- Frontend TypeScript compliance review
- Newtype ID pattern (would require significant refactoring)
- Integration tests
