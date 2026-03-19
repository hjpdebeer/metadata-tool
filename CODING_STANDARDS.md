# Coding Standards

Coding standards for the metadata-tool project. Every contributor -- human or AI -- must follow these rules. When in doubt, this document is authoritative.

This is an enterprise metadata lifecycle management platform for financial institutions: a Rust/Axum backend, a React/TypeScript/Ant Design frontend, and a PostgreSQL database. The platform manages business glossary terms, data dictionaries, data quality rules, data lineage, business applications, business processes, workflow approvals, and AI-assisted metadata enrichment.

---

## Table of Contents

1. [Naming Conventions](#1-naming-conventions)
2. [Rust Module Organisation](#2-rust-module-organisation)
3. [Type Design](#3-type-design)
4. [Error Handling](#4-error-handling)
5. [Documentation Depth](#5-documentation-depth)
6. [Testing Standards](#6-testing-standards)
7. [Dependency Management](#7-dependency-management)
8. [Formatting and Linting](#8-formatting-and-linting)
9. [Safety and Robustness](#9-safety-and-robustness)
10. [Database Conventions](#10-database-conventions)
11. [Frontend Conventions](#11-frontend-conventions)
12. [API Design Conventions](#12-api-design-conventions)
13. [Git Conventions](#13-git-conventions)
14. [Pre-Commit Verification Checklist](#14-pre-commit-verification-checklist)
15. [AI Integration Standards](#15-ai-integration-standards)

---

## 1. Naming Conventions

### 1.1 Rust

Follow RFC 430. No exceptions.

| Item | Case | Example |
|------|------|---------|
| Crate | kebab-case | `metadata-tool` |
| Module | snake_case | `data_dictionary`, `glossary` |
| Type (struct, enum, trait) | PascalCase | `GlossaryTerm`, `AppError` |
| Function / method | snake_case | `list_terms`, `create_element` |
| Constant / static | SCREAMING_SNAKE_CASE | `STATE_DRAFT`, `ENTITY_GLOSSARY_TERM` |
| Type parameter | single uppercase letter or short PascalCase | `T`, `Res` |
| Lifetime | short lowercase | `'a`, `'conn` |

#### 1.1.1 Acronym Handling

Acronyms are treated as single words in PascalCase. They are capitalised only at the first letter, not as full uppercase. This prevents visual clutter and keeps compound names readable.

| Acronym | In PascalCase | NOT |
|---------|---------------|-----|
| CDE | `Cde` | ~~`CDE`~~ |
| API | `Api` | ~~`API`~~ |
| SSO | `Sso` | ~~`SSO`~~ |
| UUID | `Uuid` | ~~`UUID`~~ |
| SQL | `Sql` | ~~`SQL`~~ |
| JWT | `Jwt` | ~~`JWT`~~ |
| RBAC | `Rbac` | ~~`RBAC`~~ |
| JSON | `Json` | ~~`JSON`~~ |
| HTTP | `Http` | ~~`HTTP`~~ |
| URL | `Url` | ~~`URL`~~ |
| ID | `Id` | ~~`ID`~~ |
| AI | `Ai` | ~~`AI`~~ |

The sole exception is SCREAMING_SNAKE_CASE constants, where acronyms remain fully capitalised because every letter is uppercase anyway:

```rust
const JWT_SECRET: &str = "...";        // correct
const DATABASE_URL: &str = "...";      // correct
const AI_SERVICE_TIMEOUT: u64 = 30;    // correct
```

#### 1.1.2 Type Name Patterns

| Pattern | When | Example |
|---------|------|---------|
| `{Domain}Error` | Error enum for a domain module | `GlossaryError`, `WorkflowError` |
| `{Domain}Service` | Service trait for a domain module | `GlossaryService`, `LineageService` |
| `{Action}{Entity}Request` | Inbound DTO | `CreateGlossaryTermRequest`, `UpdateDataElementRequest` |
| `{Action}{Entity}Response` | Outbound DTO (when it differs from the entity) | `SearchGlossaryTermsResponse` |
| `{Entity}Id(Uuid)` | Newtype wrapper for a domain entity ID | `TermId(Uuid)`, `ElementId(Uuid)` |
| `{Entity}` | Core domain struct | `GlossaryTerm`, `DataElement`, `QualityRule` |
| `App{Thing}` | Application-wide shared type | `AppState`, `AppConfig`, `AppError` |

#### 1.1.3 Function Naming

| Prefix | Meaning | Return type |
|--------|---------|-------------|
| `get_` | Fallible lookup, may fail | `Result<T, E>` |
| bare (no prefix) | Infallible accessor | `T` or `&T` |
| `is_` / `has_` | Boolean predicate | `bool` |
| `new()` | Primary constructor | `Self` or `Result<Self, E>` |
| `try_` | Alternative fallible constructor | `Result<Self, E>` |
| `list_` | Returns a collection (may be empty) | `Result<Vec<T>, E>` or `Vec<T>` |
| `create_` | Inserts a new resource | `Result<T, E>` |
| `update_` | Modifies an existing resource | `Result<T, E>` |
| `delete_` | Removes a resource (soft or hard) | `Result<(), E>` |

#### 1.1.4 Module Naming

- snake_case, always.
- Name after the concept, not the role: `glossary` not `glossary_module`, `lineage` not `lineage_utils`.
- Never use `utils`, `helpers`, `common`, or `misc` as module names. If a function has no clear home, the module structure needs rethinking.

### 1.2 Frontend (TypeScript / React)

| Item | Case | Example |
|------|------|---------|
| Component | PascalCase | `GlossaryPage`, `AppLayout` |
| Component file | PascalCase `.tsx` | `GlossaryPage.tsx`, `Dashboard.tsx` |
| Non-component file | kebab-case `.ts` | `theme-config.ts`, `api.ts` |
| Function / variable | camelCase | `listTerms`, `pageSize` |
| Constant | SCREAMING_SNAKE_CASE | `API_BASE_URL`, `DEFAULT_PAGE_SIZE` |
| CSS class | kebab-case | `term-card`, `sidebar-nav` |
| Interface / type | PascalCase | `GlossaryTerm`, `PaginatedResponse<T>` |
| Enum value | PascalCase | `EntityStatus.UnderReview` |
| Directory | kebab-case | `pages/`, `services/`, `theme/` |

### 1.3 Database

Everything in the database is snake_case. No exceptions.

| Object | Convention | Example |
|--------|-----------|---------|
| Table | Plural noun, snake_case | `glossary_terms`, `data_elements` |
| Column | snake_case | `term_name`, `is_current_version` |
| Primary key | `{singular_table}_id` | `term_id`, `element_id` |
| Foreign key column | `{referenced_table_singular}_id` | `domain_id`, `status_id` |
| Boolean column | `is_` or `has_` prefix | `is_current_version`, `has_pii` |
| Timestamp column | `_at` suffix | `created_at`, `updated_at`, `deleted_at` |
| Index | `idx_{table}_{columns}` | `idx_glossary_terms_domain` |
| Primary key constraint | `pk_{table}` | `pk_glossary_terms` (or PostgreSQL default) |
| Foreign key constraint | `fk_{table}_{referenced_table}` | `fk_glossary_terms_domains` |
| Check constraint | `ck_{table}_{description}` | `ck_term_relationships_no_self_ref` |
| Unique constraint | `uq_{table}_{columns}` | `uq_glossary_terms_name_domain` |
| Trigger | `trg_{table}_{action}` | `trg_glossary_terms_updated_at` |
| Sequence | `seq_{table}_{column}` | `seq_audit_log_id` |

### 1.4 API Paths

All paths are kebab-case, nested under `/api/v1/`:

```
/api/v1/glossary/terms
/api/v1/glossary/terms/{term_id}
/api/v1/glossary/terms/{term_id}/ai-enrich
/api/v1/data-dictionary/elements
/api/v1/data-quality/rules
/api/v1/workflow/instances/{instance_id}/transition
```

Path parameters use snake_case: `{term_id}`, `{element_id}`, `{instance_id}`.

### 1.5 Cross-Layer Naming Alignment

Frontend UI labels, API field names, and database column names MUST be traceable and consistent. A mismatch between any layer is a bug.

**Rule**: For every data field, there is ONE concept name used across all layers:

| Layer | Convention | Example |
|-------|-----------|---------|
| Database column | `snake_case` with `_id` suffix for FKs | `classification_id` |
| API response (detail) | includes both `{concept}_id` and `{concept}_name` | `classification_id` + `classification_name` |
| API response (AI suggestion) | uses concept name without `_id` | `data_classification` |
| Frontend label | Title Case of concept name | "Data Classification" |

**Lookup fields** (FK to a lookup table):
- The DB column stores the UUID: `classification_id`
- The API detail response includes the resolved display name: `classification_name`
- The AI prompt uses the concept name: `data_classification`
- The frontend label matches the concept: "Data Classification"

**Never**: Use different names for the same concept across layers (e.g., "Sensitivity Classification" in frontend but "data_classifications" in DB). See `docs/NAMING_ALIGNMENT.md` for the complete field mapping.

---

## 2. Rust Module Organisation

### 2.1 lib.rs

`lib.rs` contains exactly three things:

1. A crate-level doc comment explaining what the crate does.
2. `mod` declarations for top-level modules.
3. `pub use` re-exports for the crate's public API.

No logic, no function definitions, no type definitions.

```rust
//! Metadata management tool -- enterprise metadata lifecycle management
//! for financial institutions.

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod ai;
pub mod naming;
pub mod notifications;
pub mod workflow;
```

### 2.2 Domain Module Structure

Each domain (glossary, data_dictionary, data_quality, lineage, applications, processes, workflow) follows this structure:

```
domain/
  mod.rs          -- re-exports, module-level doc comment
  glossary.rs     -- types: entity structs, request/response DTOs
```

As modules grow, they may be split into directories:

```
domain/
  glossary/
    mod.rs        -- re-exports only
    types.rs      -- entity structs, DTOs
    service.rs    -- service trait definition
    error.rs      -- GlossaryError enum
```

Each API module mirrors its domain:

```
api/
  mod.rs          -- re-exports
  glossary.rs     -- Axum route handlers for glossary endpoints
```

### 2.3 Visibility Rules

- Use `mod` (not `pub mod`) for internal submodules. Only expose what consumers need.
- Use `pub use` in `mod.rs` to flatten the public API. Callers should not reach into submodule paths.
- Handler functions in `api/` modules are `pub` because they are referenced by the router in `main.rs`.
- Domain structs in `domain/` modules are `pub` because they are used by both API handlers and service logic.
- Internal helpers within a module are `pub(crate)` or private.

### 2.4 When to Split

Split a module into submodules when:

- It exceeds approximately 300 lines.
- It contains two or more clearly distinct concerns (e.g., types vs. service logic vs. error handling).
- Reading the file requires scrolling past unrelated code to find what you need.

Do not split prematurely. A 200-line module with a single concern is fine as one file.

---

## 3. Type Design

### 3.1 Required Derives

Every public type must derive the following, at minimum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
```

Additional derives based on usage:

| Context | Additional derive | Example |
|---------|------------------|---------|
| Read from database | `FromRow` (sqlx) | `GlossaryTerm` |
| Exposed in OpenAPI | `ToSchema` (utoipa) | All request/response types |
| Used as query params | `Deserialize` + `IntoParams` (utoipa) | `SearchGlossaryTermsRequest` |
| Used as hash map key | `Hash`, `Eq`, `PartialEq` | ID newtypes |

### 3.2 Newtype ID Pattern

Every domain entity should have a newtype wrapper for its ID. This prevents mixing up IDs from different entities at compile time:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct TermId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct ElementId(pub Uuid);
```

Until the codebase is migrated to newtypes, raw `Uuid` fields remain acceptable in existing code. New code should prefer newtypes.

### 3.3 Financial Types

Never use `f32` or `f64` for monetary amounts or any value where floating-point imprecision is unacceptable. This platform manages metadata for financial institutions; if monetary values appear (e.g., data quality cost-of-poor-quality scores), use `rust_decimal::Decimal`.

```rust
// WRONG
pub cost_impact: f64,

// CORRECT
pub cost_impact: rust_decimal::Decimal,
```

### 3.4 Time Types

All timestamps use `chrono::DateTime<Utc>`. No naive datetimes, no local time, no Unix timestamps as integers:

```rust
pub created_at: DateTime<Utc>,
pub updated_at: DateTime<Utc>,
pub deleted_at: Option<DateTime<Utc>>,
```

### 3.5 Option Semantics

`Option<T>` means "not provided" or "not applicable". It is semantically distinct from zero, empty string, or a default value:

- `Option<String>`: `None` means the field was not provided. `Some("")` should be treated as a validation error if the field requires content when present.
- `Option<Uuid>`: `None` means no association. This is not the same as a nil UUID.
- `Option<DateTime<Utc>>`: `None` means the event has not occurred (e.g., `deleted_at: None` means not deleted).

Never use `Option<T>` to mean "I was too lazy to figure out the right default." Every `Option<T>` field must have a documented reason for being optional.

---

## 4. Error Handling

### 4.1 Error Crate Strategy

| Context | Crate | Rationale |
|---------|-------|-----------|
| Library / domain code | `thiserror` | Structured, typed error enums |
| Binary (`main.rs`) | `anyhow` | Convenient for top-level error propagation |
| API handlers | `AppError` + `thiserror` | Maps to HTTP status codes via `IntoResponse` |

### 4.2 Error Enum per Domain

Each domain module that can fail should define its own error enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GlossaryError {
    #[error("glossary term {0} not found")]
    TermNotFound(Uuid),

    #[error("duplicate term name in domain {domain_id}: {term_name}")]
    DuplicateTermName { term_name: String, domain_id: Uuid },

    #[error("term {0} cannot be modified in current workflow state")]
    InvalidWorkflowState(Uuid),
}
```

Domain errors convert into `AppError` via `From` implementations, which map them to the appropriate HTTP status code.

### 4.3 Error Message Rules

- **Lowercase**: Error messages start with a lowercase letter.
- **No trailing period**: `"term not found"` not `"Term not found."`.
- **Include context**: Include IDs, field names, or state values that help diagnose the issue. `"glossary term 550e8400-... not found"` not `"not found"`.
- **Never include PII**: No user names, email addresses, or personal data in error messages. Use user IDs only.
- **No stack traces in responses**: Stack traces go to logs (via `tracing`), never to the API response.

### 4.4 Unwrap and Expect

- **Never** use `.unwrap()` in production code. Use `.expect("reason")` only when the invariant is truly guaranteed and you can explain why in the message.
- `.unwrap()` is permitted in `#[cfg(test)]` blocks and in test helper functions.
- Prefer `?` for error propagation. Prefer `.ok_or_else(|| ...)` or `.map_err(|e| ...)` for conversions.

### 4.5 Silent Error Discarding

Never use `let _ = fallible_operation();` without logging or handling the error. If you intentionally discard a result, add a comment explaining why and log at `debug` or `warn` level:

```rust
// Connection cleanup is best-effort; failure here does not affect the response.
if let Err(err) = pool.close().await {
    tracing::warn!(?err, "connection pool close failed during shutdown");
}
```

---

## 5. Documentation Depth

### 5.1 Every Public Item Gets a Doc Comment

No public type, function, trait, constant, or module may exist without a `///` doc comment. If you cannot explain what it does in one sentence, reconsider the design.

### 5.2 Crate and Module Level

The crate-level doc comment (`//!` in `lib.rs`) explains:

- What this crate is responsible for.
- What it is NOT responsible for (boundary with other crates/modules).
- Key design decisions.

Module-level doc comments (`//!` in `mod.rs`) explain:

- The domain concept this module owns.
- Relationships to other modules.
- Data flow (who calls this module and what it calls).

```rust
//! Business glossary domain.
//!
//! Owns the definition and governance of business terms, domains, categories,
//! and term relationships. Terms flow through the shared workflow engine
//! (see `crate::workflow`) for approval. AI enrichment (see `crate::ai`)
//! generates definition suggestions for new terms.
//!
//! This module does NOT own:
//! - Workflow state transitions (owned by `crate::workflow`)
//! - User/role management (owned by `crate::auth`)
//! - Full-text search indexing (handled by PostgreSQL triggers)
```

### 5.3 Structs

Document the struct's purpose, any invariants, and each field:

```rust
/// A business glossary term with its governance metadata.
///
/// Terms are versioned: when a term is updated after acceptance, a new version
/// is created with `version_number` incremented and `is_current_version = true`.
/// The previous version's `is_current_version` is set to `false`.
///
/// Soft-deleted terms have `deleted_at` set and are excluded from all queries
/// unless explicitly requested.
pub struct GlossaryTerm {
    /// Unique identifier for this term version.
    pub term_id: Uuid,
    /// Human-readable term name. Unique within a domain for current versions.
    pub term_name: String,
    /// Business definition of the term. Required; must be non-empty.
    pub definition: String,
    // ... etc.
}
```

### 5.4 Traits

Document the contract, who implements the trait, and who consumes it:

```rust
/// Service interface for glossary operations.
///
/// Implemented by `GlossaryServiceImpl` (backed by PostgreSQL).
/// Consumed by API handlers in `crate::api::glossary`.
///
/// All methods that modify data require a valid `Claims` (authenticated user).
/// Read methods may be called without authentication for public terms.
pub trait GlossaryService: Send + Sync {
    // ...
}
```

### 5.5 Functions

Document purpose, parameters, return value, and error conditions:

```rust
/// Retrieves a glossary term by its ID.
///
/// Returns the current version of the term. Soft-deleted terms are not returned.
///
/// # Errors
///
/// - `GlossaryError::TermNotFound` if no active term exists with the given ID.
/// - `AppError::Database` on connection or query failure.
pub async fn get_term(pool: &PgPool, term_id: Uuid) -> AppResult<GlossaryTerm> {
    // ...
}
```

### 5.6 Specificity

Documentation must be specific to this project. Do not write generic doc comments like "Creates a new instance" -- explain *what* is being created, *why*, and what side effects occur (workflow initiation, audit logging, notification triggers).

---

## 6. Testing Standards

### 6.1 Test Naming

Tests are named `{subject}_{behaviour}`:

```rust
#[test]
fn term_id_equality_matches_inner_uuid() { ... }

#[test]
fn create_term_request_rejects_empty_definition() { ... }

#[tokio::test]
async fn list_terms_returns_empty_for_new_domain() { ... }

#[tokio::test]
async fn get_term_returns_not_found_for_deleted_term() { ... }
```

Do not use `test_` prefix -- the `#[test]` attribute already indicates it is a test.

### 6.2 Test Organisation

- **Unit tests**: In the same file as the code under test, inside a `#[cfg(test)] mod tests { ... }` block.
- **Integration tests**: In the `tests/` directory at the crate root. These test the API through HTTP requests against a real or test database.
- **Frontend tests**: Colocated with the component (e.g., `GlossaryPage.test.tsx` next to `GlossaryPage.tsx`).

### 6.3 Mandatory Test Scenarios

Every function or handler must be tested for:

1. **Happy path**: The expected successful case.
2. **Every error variant**: If the function can return `TermNotFound`, `DuplicateTermName`, and `InvalidWorkflowState`, each variant gets at least one test.
3. **Invalid input**: Empty strings, null-like values, negative numbers, strings exceeding length limits.
4. **Boundary values**: Page 0 vs. page 1, empty result sets, maximum page sizes.

### 6.4 Regression Tests

Every bug fix must include a test that reproduces the bug. The test must fail without the fix and pass with it. Name it descriptively:

```rust
#[tokio::test]
async fn create_term_does_not_panic_on_unicode_term_name() { ... }
```

### 6.5 Test Helpers

Shared test utilities (fixture builders, database setup) live in a `tests/common/` module or a `test_utils` module within the crate. Never duplicate setup logic across test files.

### 6.6 Frontend Testing

- Use React Testing Library for component tests. Test behaviour, not implementation details.
- Mock API calls using MSW (Mock Service Worker) or jest mocks on the axios instance.
- Every page component must have at least: renders without crashing, displays loading state, displays data, handles error state.

---

## 7. Dependency Management

### 7.1 Workspace Dependencies (Rust)

All external dependencies are declared in the root `Cargo.toml` under `[workspace.dependencies]`. Individual crates reference them with `workspace = true`:

```toml
# Root Cargo.toml
[workspace.dependencies]
axum = { version = "0.8", features = ["macros", "json"] }
serde = { version = "1", features = ["derive"] }

# backend/Cargo.toml
[dependencies]
axum = { workspace = true }
serde = { workspace = true }
```

### 7.2 Adding a New Dependency

Before adding any dependency:

1. **Check if an existing dependency already provides the functionality.** Do not add `rand` if `uuid` already covers your needs.
2. **Verify the license** is compatible with MIT OR Apache-2.0.
3. **Check maintenance status**: last commit date, open issues, download count on crates.io.
4. **Add to `[workspace.dependencies]` first**, then reference with `workspace = true` in the crate.
5. **Document why** the dependency is needed in the PR description.

### 7.3 Frontend Dependencies

All frontend dependencies are managed in `frontend/package.json`. The same diligence applies: check for existing solutions, verify license compatibility, prefer well-maintained packages.

Current approved frontend dependencies:
- `react`, `react-dom` -- UI framework
- `react-router-dom` -- client-side routing
- `antd`, `@ant-design/icons`, `@ant-design/pro-components` -- UI component library
- `@xyflow/react` -- lineage visualization (React Flow)
- `axios` -- HTTP client
- `dayjs` -- date manipulation (Ant Design's date library)

### 7.4 Version Pinning

- Rust: Use exact major.minor in `Cargo.toml` (e.g., `version = "0.8"`, not `version = "*"`).
- Frontend: Use caret ranges (`^`) as npm default. Lock file (`package-lock.json`) must be committed.

---

## 8. Formatting and Linting

### 8.1 Rust Formatting

All Rust code is formatted with `cargo fmt`. The project uses a `rustfmt.toml` at the workspace root:

```toml
max_width = 100
edition = "2024"
```

### 8.2 Rust Linting

All code must pass clippy with warnings treated as errors:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Do not add `#[allow(clippy::...)]` without a comment explaining why the lint does not apply. Prefer fixing the code to suppressing the lint.

### 8.3 Frontend Formatting and Linting

- **Prettier** for formatting (to be configured in `.prettierrc`).
- **ESLint** for linting (to be configured in `eslint.config.js`).
- TypeScript strict mode is mandatory (`"strict": true` in `tsconfig.json`).

### 8.4 CI Enforcement

The following commands must all succeed before a PR can be merged:

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps

# Frontend
cd frontend && npm ci && npm run build
```

---

## 9. Safety and Robustness

### 9.1 No Unsafe Code

`unsafe` is not permitted in domain code, API handlers, or service logic. If a dependency requires `unsafe` internally, that is acceptable -- but this project's code must not contain any `unsafe` blocks.

If an edge case genuinely requires `unsafe` (which is unlikely for a web application), it must be:

1. Isolated in its own function with a `# Safety` doc comment.
2. Reviewed and approved explicitly.
3. Covered by tests that exercise the unsafe invariant.

### 9.2 No Panics in Request Paths

No code in the request-handling path (API handler, middleware, service, database query) may panic. This means:

- No `.unwrap()` or `.expect()` on values that come from user input, database queries, or external services.
- No `panic!()`, `unreachable!()`, or `todo!()` in production code paths. (`todo!()` is acceptable during initial development of stub handlers, but must be replaced before the feature is considered complete.)
- Array/slice indexing (`vec[i]`) must be replaced with `.get(i)` when the index is not statically guaranteed.

### 9.3 Input Validation

Validate all input at the trust boundary -- the API layer. Use the `validator` crate with derive macros on request types:

```rust
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateGlossaryTermRequest {
    #[validate(length(min = 1, max = 512))]
    pub term_name: String,
    #[validate(length(min = 1))]
    pub definition: String,
}
```

The API handler must call `.validate()` and convert validation errors into `AppError::Validation` before processing the request.

Inner layers (service, database) may assume input has been validated. They should still return errors for business rule violations (duplicate names, invalid state transitions), but should not re-validate string lengths.

### 9.4 SQL Injection Prevention

All database queries must use parameterized queries via the `sqlx::query!` or `sqlx::query_as!` macros. Never construct SQL strings with format strings or string concatenation:

```rust
// CORRECT
let term = sqlx::query_as!(
    GlossaryTerm,
    "SELECT * FROM glossary_terms WHERE term_id = $1 AND deleted_at IS NULL",
    term_id
)
.fetch_optional(&pool)
.await?;

// WRONG -- SQL injection risk
let query = format!("SELECT * FROM glossary_terms WHERE term_id = '{}'", term_id);
```

For dynamic queries (optional filters, dynamic sorting), use `sqlx::QueryBuilder` or conditional query construction with bound parameters.

### 9.5 No PII in Logs

Structured logging via `tracing` is mandatory. Never log:

- User email addresses
- User display names
- Authentication tokens or secrets
- Request bodies that may contain personal data

Log user IDs (UUIDs) for traceability. Use `tracing::instrument` on functions to automatically capture function arguments, but `#[instrument(skip(password, token, body))]` to exclude sensitive fields:

```rust
#[tracing::instrument(skip(state, body), fields(user_id = %claims.sub))]
pub async fn create_term(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateGlossaryTermRequest>,
) -> AppResult<Json<GlossaryTerm>> {
    // ...
}
```

---

## 10. Database Conventions

### 10.1 General

- Single PostgreSQL 17 database, `public` schema.
- All object names in snake_case.
- Third normal form (3NF) as baseline. Denormalization is permitted only with documented justification (performance, materialised views for read-heavy paths).

### 10.2 Tables

- Plural nouns: `glossary_terms`, `data_elements`, `quality_rules`.
- Lookup/reference tables: descriptive name, may be singular when representing a type: `entity_statuses`, `glossary_term_relationship_types`.
- Junction tables: `{table1}_{table2}` or a descriptive name: `process_data_elements`, `application_data_elements`.

### 10.3 Columns

| Column type | Convention | Example |
|-------------|-----------|---------|
| Primary key | `{singular_entity}_id UUID DEFAULT gen_random_uuid()` | `term_id`, `element_id` |
| Foreign key | `{referenced_singular}_id` | `domain_id`, `status_id`, `owner_user_id` |
| Boolean | `is_` or `has_` prefix | `is_current_version`, `has_pii`, `is_symmetric` |
| Timestamp | `_at` suffix, `TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP` | `created_at`, `updated_at` |
| Soft delete | `deleted_at TIMESTAMPTZ` (nullable, NULL = not deleted) | `deleted_at` |
| Audit | `created_by UUID NOT NULL REFERENCES users(user_id)`, `updated_by UUID REFERENCES users(user_id)` | -- |
| Text search | `search_vector TSVECTOR` | `search_vector` on `glossary_terms` |
| Display ordering | `display_order INT NOT NULL DEFAULT 0` | On lookup tables |

### 10.4 Standard Columns

Every domain table must include:

```sql
created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
created_by   UUID NOT NULL REFERENCES users(user_id),
updated_by   UUID REFERENCES users(user_id)
```

Tables that support soft delete also include:

```sql
deleted_at   TIMESTAMPTZ
```

### 10.5 Primary Keys

All primary keys are UUIDs generated by `gen_random_uuid()`. No serial integers. UUIDs are generated by PostgreSQL, not by the application.

### 10.6 Indexes

- Every foreign key column gets an index (PostgreSQL does not auto-index FK columns).
- Full-text search columns get a GIN index: `CREATE INDEX idx_{table}_search ON {table} USING GIN(search_vector)`.
- Partial indexes for soft-deleted tables: `WHERE deleted_at IS NULL`.
- Composite indexes for common query patterns (e.g., `(term_name, domain_id) WHERE is_current_version = TRUE AND deleted_at IS NULL`).

### 10.7 Constraints

- Use `CHECK` constraints for business rules that can be expressed as simple predicates: `CHECK(source_term_id != target_term_id)`.
- Use `UNIQUE` constraints rather than relying on application-level uniqueness checks.
- Name all constraints explicitly (see naming conventions in section 1.3).

### 10.8 Migrations

- Migrations live in `backend/migrations/`.
- File naming: `{NNN}_{description}.sql` where NNN is a zero-padded sequential number.
- Each migration is idempotent where possible (use `IF NOT EXISTS`, `CREATE OR REPLACE`).
- Never modify an existing migration that has been applied. Create a new migration instead.
- Destructive migrations (DROP TABLE, DROP COLUMN) require explicit approval and a data preservation plan.

### 10.9 Triggers

- `updated_at` is maintained by a trigger, not by application code. The trigger fires on UPDATE and sets `updated_at = CURRENT_TIMESTAMP`.
- Naming standards validation triggers enforce column/table naming rules at the database level (see migration 012).
- Business logic triggers (e.g., critical business process auto-CDE designation) are documented in the migration that creates them.

---

## 11. Frontend Conventions

### 11.1 Component Structure

- One component per file.
- File name matches the component name in PascalCase: `GlossaryPage.tsx` exports `GlossaryPage`.
- Co-locate styles, tests, and types with the component when they are specific to it.

```
pages/
  GlossaryPage.tsx
  GlossaryPage.test.tsx     (when tests are added)
  Dashboard.tsx
```

### 11.2 Component Patterns

- Functional components only. No class components.
- Use `React.FC` for components with no children; explicit props interface otherwise.
- Props interface is named `{Component}Props` and defined in the same file, directly above the component.

```tsx
interface TermCardProps {
  term: GlossaryTerm;
  onEdit: (termId: string) => void;
}

const TermCard: React.FC<TermCardProps> = ({ term, onEdit }) => {
  // ...
};

export default TermCard;
```

### 11.3 State Management

- **Local state**: `useState` for component-specific state.
- **Shared state**: React Context + `useReducer` for state shared across a subtree (e.g., authentication context, current user).
- **Server state**: Fetch data in the component or a custom hook. Consider React Query / TanStack Query if caching and revalidation needs grow.
- **Avoid prop drilling**: If a prop passes through more than two intermediate components, lift it to context or restructure the component tree.

### 11.4 API Calls

All API calls go through the centralised axios client in `src/services/api.ts`. Service functions are organised by domain:

```
services/
  api.ts                -- axios instance with JWT interceptor
  glossary-service.ts   -- listTerms(), getTerm(), createTerm(), etc.
  workflow-service.ts   -- getMyTasks(), transitionInstance(), etc.
```

Service functions return typed responses:

```typescript
export async function listTerms(params: SearchTermsParams): Promise<PaginatedResponse<GlossaryTerm>> {
  const response = await api.get('/glossary/terms', { params });
  return response.data;
}
```

### 11.5 Ant Design Theme

- Theme configuration lives in `src/theme/themeConfig.ts`.
- Primary colour: deep navy (#1B3A5C).
- Customise via Ant Design's token system. Override component tokens in `themeConfig.ts`.
- Never override Ant Design styles with raw CSS unless the token system cannot achieve the desired result. Document the reason in a comment if raw CSS is necessary.

### 11.6 Routing

- React Router v7 (`react-router-dom`).
- All routes defined in `App.tsx`.
- Page components are lazy-loaded for code splitting (once the app grows beyond initial pages):

```tsx
const GlossaryPage = React.lazy(() => import('./pages/GlossaryPage'));
```

- Use `<Navigate>` for redirects, never `window.location`.

### 11.7 TypeScript Discipline

- **Strict mode**: `"strict": true` in `tsconfig.json`. Non-negotiable.
- **No `any`**: Never use `any`. Use `unknown` and narrow with type guards if the type is truly unknown.
- **Interfaces over type aliases** for object shapes: `interface GlossaryTerm { ... }` not `type GlossaryTerm = { ... }`. Use `type` for unions, intersections, and mapped types.
- **Explicit return types** on exported functions and service functions. Inferred return types are acceptable for internal/private helpers and event handlers.

---

## 12. API Design Conventions

### 12.1 RESTful Resource Orientation

Endpoints represent resources, not actions. Use HTTP methods to express operations:

| Operation | Method | Path | Status |
|-----------|--------|------|--------|
| List | GET | `/api/v1/glossary/terms` | 200 |
| Get one | GET | `/api/v1/glossary/terms/{term_id}` | 200 |
| Create | POST | `/api/v1/glossary/terms` | 201 |
| Update | PUT | `/api/v1/glossary/terms/{term_id}` | 200 |
| Delete | DELETE | `/api/v1/glossary/terms/{term_id}` | 204 |
| Custom action | POST | `/api/v1/glossary/terms/{term_id}/ai-enrich` | 200 |

### 12.2 Versioning

All API paths are prefixed with `/api/v1/`. When breaking changes are needed, a `/api/v2/` prefix is introduced. Old versions are maintained until all consumers migrate.

### 12.3 Pagination

List endpoints support pagination via query parameters:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | integer | 1 | Page number (1-indexed) |
| `page_size` | integer | 20 | Items per page (max 100) |

Response includes pagination metadata:

```json
{
  "data": [...],
  "total_count": 142,
  "page": 1,
  "page_size": 20
}
```

### 12.4 Filtering

Filter by query parameters matching field names:

```
GET /api/v1/glossary/terms?domain_id=...&status=ACCEPTED&query=customer
```

- `query` is reserved for full-text search.
- Other parameters filter by exact match on the corresponding field.

### 12.5 Sorting

Sort via query parameters:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `sort_by` | string | `created_at` | Column to sort by |
| `sort_order` | string | `desc` | `asc` or `desc` |

Only allow sorting on indexed columns. Return 400 for unsupported sort fields.

### 12.6 Error Responses

All error responses follow a consistent shape:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "glossary term 550e8400-e29b-41d4-a716-446655440000 not found"
  }
}
```

The `code` field is a machine-readable SCREAMING_SNAKE_CASE string. The `message` field is a human-readable description. Error codes are defined in `AppError` (see `backend/src/error.rs`).

Standard error codes:

| HTTP Status | Code | When |
|-------------|------|------|
| 400 | `BAD_REQUEST` | Malformed request |
| 401 | `UNAUTHORIZED` | Missing or invalid JWT |
| 403 | `FORBIDDEN` | Authenticated but insufficient role |
| 404 | `NOT_FOUND` | Resource does not exist |
| 409 | `CONFLICT` | Duplicate resource or state conflict |
| 422 | `VALIDATION_ERROR` | Field validation failure |
| 422 | `NAMING_VIOLATION` | Naming standard violation |
| 422 | `WORKFLOW_ERROR` | Invalid workflow transition |
| 502 | `AI_SERVICE_ERROR` | Claude/OpenAI API failure |
| 500 | `DATABASE_ERROR` | Database query failure |
| 500 | `INTERNAL_ERROR` | Unexpected server error |

### 12.7 OpenAPI Annotations

Every endpoint must have a `#[utoipa::path(...)]` annotation with:

- HTTP method and path
- Path and query parameters with types and descriptions
- Request body type (for POST/PUT)
- All possible response status codes with descriptions and body types
- `security(("bearer_auth" = []))` for authenticated endpoints
- `tag` matching the domain

### 12.8 Authentication

- All mutating endpoints (POST, PUT, DELETE) require a valid JWT Bearer token.
- Read endpoints may be protected or public depending on the resource.
- JWT tokens are issued after Microsoft Entra ID SSO authentication.
- The `Claims` struct (see `backend/src/auth/mod.rs`) is extracted by middleware and passed to handlers.

### 12.9 CORS

CORS is configured to allow only the frontend origin (`frontend_url` from `AppConfig`). No wildcard origins in production. The development configuration allows `http://localhost:5173`.

---

## 13. Git Conventions

### 13.1 Branch Naming

Branches are named with a category prefix and a kebab-case description:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feature/` | New functionality | `feature/glossary-search` |
| `fix/` | Bug fix | `fix/term-duplicate-on-update` |
| `docs/` | Documentation only | `docs/api-pagination-guide` |
| `refactor/` | Code restructuring without behaviour change | `refactor/extract-glossary-service` |
| `chore/` | Build, CI, dependency updates | `chore/upgrade-axum-0.9` |
| `test/` | Adding or fixing tests | `test/workflow-transition-coverage` |

### 13.2 Commit Messages

Follow conventional commits:

```
<type>: <description>

[optional body]

[optional footer]
```

Types:

| Type | When |
|------|------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `refactor` | Code restructuring without behaviour change |
| `chore` | Build, CI, deps, tooling |
| `test` | Adding or fixing tests |
| `style` | Formatting changes (no logic change) |
| `perf` | Performance improvement |

Examples:

```
feat: add full-text search to glossary term listing

fix: prevent duplicate term creation when domain_id is null

refactor: extract workflow transition logic into WorkflowService trait

chore: upgrade sqlx to 0.8.3 for PostgreSQL 17 compatibility

docs: document AI enrichment request/response schema
```

Rules:

- Description starts with a lowercase verb in imperative mood: "add", "fix", "extract", not "Added", "Fixes", "Extracting".
- First line is 72 characters or less.
- Body (if present) is separated by a blank line and wraps at 72 characters.
- Reference issue numbers in the footer: `Closes #42`.

### 13.3 Branch Workflow

- **No direct commits to `main`.** All changes go through feature branches and pull requests.
- Branch from `main`, merge back to `main` via PR.
- PRs require all automated checks to pass before merge.
- Squash-merge is preferred for feature branches to keep `main` history clean.
- Delete the branch after merge.

---

## 14. Pre-Commit Verification Checklist

Before committing any change, verify every applicable item. This checklist applies to both human developers and AI-assisted development.

### 14.1 Rust Backend

- [ ] `cargo fmt --all -- --check` passes (no formatting violations)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes (no lint warnings)
- [ ] `cargo test --workspace` passes (all tests green)
- [ ] `cargo doc --workspace --no-deps` passes (no doc build errors or broken links)

### 14.2 Frontend

- [ ] `cd frontend && npm run build` succeeds (TypeScript type check + Vite build)

### 14.3 Security

- [ ] No hardcoded credentials, API keys, secrets, or tokens in the code
- [ ] No `.env` files or credential files added to the commit
- [ ] No PII (names, emails, addresses) in log statements or error messages
- [ ] SQL queries use parameterized bindings, never string interpolation

### 14.4 Code Quality

- [ ] Every new public item (type, function, trait, constant) has a doc comment
- [ ] Every new API endpoint has a `#[utoipa::path(...)]` annotation
- [ ] No `.unwrap()` in production code paths
- [ ] No `unsafe` blocks in domain code
- [ ] No `any` type in TypeScript code
- [ ] Error types include relevant context (IDs, field names) without PII

### 14.5 Database

- [ ] Database schema changes have a corresponding new migration file in `backend/migrations/`
- [ ] Migration file is sequentially numbered (`{NNN}_{description}.sql`)
- [ ] Existing migrations have not been modified
- [ ] New tables include `created_at`, `updated_at`, `created_by`, `updated_by` columns
- [ ] Foreign key columns have indexes

### 14.6 Naming

- [ ] Rust names follow RFC 430 (section 1.1)
- [ ] Acronyms are treated as single words in PascalCase (section 1.1.1)
- [ ] Database names follow snake_case conventions (section 1.3)
- [ ] API paths follow kebab-case under `/api/v1/` (section 1.4)
- [ ] Frontend names follow the TypeScript/React conventions (section 1.2)

### 14.7 Testing

- [ ] New functionality has corresponding tests
- [ ] Bug fixes include a regression test
- [ ] All existing tests still pass

---

## 15. AI Integration Standards

### 15.1 Structured Response Schema

All AI prompts MUST request responses in a strict JSON schema with defined data types and length constraints. Never rely on AI to infer the correct format — specify it explicitly.

Every AI prompt that expects structured data MUST include:
1. The exact JSON schema with field names, types, and constraints
2. Maximum character lengths matching the target database column types
3. Allowed values for enum/dropdown fields
4. Explicit instructions on what NOT to include

Example prompt schema block:
```
Respond with a JSON array. Each object must conform to this schema:
{
  "field_name": "string — exact field name from the list above",
  "suggested_value": "string — max 2000 characters for TEXT fields, max 50 for abbreviation",
  "confidence": "number — between 0.0 and 1.0 inclusive",
  "rationale": "string — max 500 characters, cite standards where applicable"
}
```

### 15.2 Backend Validation of AI Responses

AI responses MUST be validated before storage. The backend MUST:

1. **Schema validation**: Verify the response is valid JSON matching the expected structure. Reject malformed responses with `AppError::AiService`.
2. **Field allow-list**: Only accept suggestions for fields explicitly listed in the allow-list. Silently drop suggestions for unlisted fields (e.g., `_id`, `_at`, `_by` fields).
3. **Length enforcement**: Truncate or reject `suggested_value` entries that exceed the target column's maximum length. Never let a database `VARCHAR` overflow error reach the user.
4. **Type coercion**: Handle AI returning `null` where a string is expected (use `#[serde(default)]` or custom deserializers). Handle confidence values outside 0.0–1.0 (clamp, don't reject).
5. **Content filtering**: Strip control characters, excessive whitespace, and markdown formatting artifacts from suggested values before storage.

### 15.3 Prompt Design Rules

1. **Never expose internal IDs**: AI prompts must only contain human-readable text fields for the entity being enriched. Never send UUID primary keys, foreign keys, status IDs, or user IDs as entity data. Exception: lookup table UUIDs are included deliberately — see Section 15.6.
2. **Never request ID suggestions**: The prompt must explicitly instruct the AI to never suggest values for fields ending in `_id`, `_at`, or `_by`, or for ownership fields (owner, steward, approver).
3. **Lookup fields**: When a field maps to a lookup table (dropdown), the prompt MUST include the complete lookup table with `{id, name}` pairs. The AI returns the UUID directly — the backend parses it with `Uuid::parse_str()`. See Section 15.6 for the full pattern.
4. **Idempotent enrichment**: Calling enrichment multiple times on the same entity must not create duplicate suggestions. Check for existing PENDING suggestions before creating new ones, or replace them.
5. **Financial services context**: All prompts must reference industry standards (DAMA DMBOK, BCBS 239, ISO 8000) to ground suggestions in authoritative sources rather than generic knowledge.

### 15.4 AI Suggestion Lifecycle (Principle 6)

AI suggestions MUST follow this lifecycle — no exceptions:

```
AI generates → Stored as PENDING → User reviews → ACCEPTED / MODIFIED / REJECTED
```

1. AI NEVER auto-publishes metadata (Principle 6: AI-Assisted, Human-Governed).
2. All suggestions are stored in `ai_suggestions` table with PENDING status for audit trail (Principle 9: Audit Everything).
3. Accepted suggestions are applied to the entity and the suggestion status is updated to ACCEPTED (or MODIFIED if the user edited the value).
4. Rejected suggestions are marked REJECTED but never deleted — they remain in the audit trail.
5. Users may optionally provide feedback (1–5 rating) on suggestions for quality tracking.

### 15.5 Error Handling for AI Calls

1. **Timeout**: AI API calls must have a connect timeout (10s) and a total timeout (90s). Log timeouts as warnings, not errors.
2. **Fallback**: If the primary provider (Claude) fails, fall back to the secondary (OpenAI). If both fail, return a clear error message to the user — never silently fail.
3. **Rate limiting**: Respect provider rate limits. If rate-limited, return a user-friendly message suggesting they retry in a few minutes.
4. **Cost awareness**: Log the provider and model used for each enrichment call. Monitor usage to prevent unexpected costs.
5. **Network**: Use `native-tls` (OS certificate store) for AI API calls to avoid DNS/IPv6 issues with `rustls`. Force IPv4 via `local_address(Ipv4Addr::UNSPECIFIED)` if the network does not support IPv6.

### 15.6 Lookup Fields in AI Prompts

When an AI prompt needs to suggest a value for a field that maps to a lookup table (FK to a predefined list), the prompt MUST include the complete lookup table with UUIDs. The AI returns the UUID directly — no fuzzy matching.

**Standard pattern**:
1. Before building the prompt, fetch all lookup values from DB as `{id, name}` pairs (see `fetch_glossary_lookups` in `api/ai.rs`)
2. Include them in the prompt: "Pick the best match from this list. Return the UUID."
3. AI returns the UUID in `suggested_value`
4. Acceptance handler uses `resolve_lookup()` which tries `Uuid::parse_str()` first, falling back to ILIKE name match for backward compatibility
5. If the UUID does not parse and no ILIKE match is found, the suggestion is silently dropped

**Why**: Eliminates reliance on fuzzy matching (ILIKE), prevents mismatches when display names are ambiguous, and ensures AI can only pick from valid values. The prompt is self-contained — the AI does not need external knowledge of what lookup values exist.

**Applies to**: domain, category, data_classification, term_type, unit_of_measure — any field backed by a lookup table. When adding new lookup fields, follow this same pattern: fetch the lookup table, embed it in the prompt, accept the UUID in the handler.
