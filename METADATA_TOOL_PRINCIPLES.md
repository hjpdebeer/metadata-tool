# Metadata Tool — Foundational Principles

These principles are the immutable foundation of the Metadata Management Tool. Every design decision, architecture choice, and code contribution must honour them. They are non-negotiable.

---

## Principle 1: API-First

All functionality is designed, specified, and exposed as REST APIs before any UI is built. APIs are the primary interface — the frontend and any external integrations are consumers. OpenAPI 3.1 specs are generated from Rust types at compile time via utoipa. API contracts are always in sync with the implementation.

**Why:** The tool must support ingesting metadata via API from discovery tools and allow design tools to read from it. The UI is one of many consumers.

**Enforcement:**

- Every API handler in `backend/src/api/*.rs` carries `#[utoipa::path(...)]` annotations that generate the OpenAPI spec directly from the Rust function signatures, request types, and response types. The spec is never hand-written.
- The `ApiDoc` struct in `backend/src/main.rs` registers all paths and schema components via the `#[derive(OpenApi)]` macro. Adding a route without an OpenAPI annotation is a compile-time omission that is caught during code review — the route will be absent from `/swagger-ui/`.
- All domain types in `backend/src/domain/*.rs` derive `utoipa::ToSchema` alongside `serde::Serialize`/`Deserialize`, ensuring that the OpenAPI schema and the wire format are generated from the same Rust struct.
- The Swagger UI is served at `/swagger-ui/` and the raw spec at `/api-docs/openapi.json`, making the API contract browsable and machine-readable at all times.
- All routes are mounted under `/api/v1/` in `backend/src/main.rs`. No functionality exists only in the frontend — every user-facing feature has a corresponding API endpoint.
- The frontend in `frontend/` consumes the API via Axios (`frontend/src/services/api.ts`) and never accesses the database directly.

---

## Principle 2: Metadata-Described Everything

All data elements, structures, and integrations in the tool must be fully described with metadata covering three dimensions:

- **Business metadata:** owner, domain, classification, regulatory tags, glossary terms
- **Technical metadata:** data type, format, precision, length, nullability, allowed values
- **Data quality metadata:** validation rules, completeness thresholds, freshness requirements

The metadata registry is the single source of truth. The tool's own database schema is self-describing (`meta_tables`, `meta_columns`).

**Why:** A metadata management tool that doesn't manage its own metadata is a contradiction.

**Enforcement:**

- Migration `backend/migrations/012_meta_and_triggers.sql` creates `meta_tables` and `meta_columns` — the tool's own self-describing metadata registry. Every table in the schema has a corresponding entry in `meta_tables` with its display name, description, domain, and change frequency. Every column has an entry in `meta_columns` with its description, data type, whether it is required, and example values.
- Business metadata is captured on every domain entity: `owner_user_id`, `steward_user_id`, `domain_id`, `classification_id`, `regulatory_reference`, and `glossary_term_id` fields appear on `glossary_terms`, `data_elements`, `quality_rules`, `applications`, and `business_processes` (see `backend/src/domain/*.rs`).
- Technical metadata is captured in the `source_systems`, `technical_schemas`, `technical_tables`, and `technical_columns` tables (migration `backend/migrations/004_data_dictionary.sql`), recording data types, precision, lengths, nullability, ordinal positions, and foreign key relationships.
- Data quality metadata is captured in `quality_dimensions`, `quality_rules`, `quality_assessments`, and `quality_scores` (migration `backend/migrations/005_data_quality.sql`), including threshold percentages, severity levels, and assessment results.
- When adding a new table to any migration, a corresponding `INSERT INTO meta_tables` and `INSERT INTO meta_columns` block must be added to keep the self-describing registry current.

---

## Principle 3: Data Quality-By-Design (Quality-in-Depth)

Data quality is enforced at every layer independently:

1. **Application layer** — API input validation before processing
2. **Service layer** — business rule validation, naming standards, workflow state checks
3. **Database layer** — constraints, triggers, check functions

No layer trusts another. Each validates independently.

**Why:** Following defence-in-depth, a defect in one layer's validation does not propagate.

**Enforcement:**

- **Application layer:** The `validator` crate (version 0.19 with `derive` feature in `backend/Cargo.toml`) provides declarative validation on request types in `backend/src/domain/*.rs`. API handlers in `backend/src/api/*.rs` validate input before any database interaction. The `AppError::Validation` variant in `backend/src/error.rs` returns HTTP 422 for validation failures.
- **Service layer:** The `naming` module (`backend/src/naming/mod.rs`) validates technical names against configurable patterns loaded from the `naming_standards` table. The `AppError::NamingViolation` variant provides a distinct error for naming failures. Workflow state transitions are validated against the `workflow_transitions` table to ensure only permitted transitions occur — the `AppError::Workflow` variant catches illegal transitions.
- **Database layer:** PostgreSQL enforces constraints independently of the application:
  - `CHECK` constraints on `quality_rules.severity`, `workflow_tasks.status`, `process_data_elements.usage_type`, and `ai_suggestions.status` (migrations 005, 009, 008, 011).
  - `NOT NULL`, `UNIQUE`, and `FOREIGN KEY` constraints on all tables prevent orphaned records and duplicates.
  - The `validate_column_naming_standards()` trigger in migration 012 fires on every `INSERT` or `UPDATE` of `technical_columns.column_name` and sets `naming_standard_compliant` and `naming_standard_violation` directly in the row.
  - The `audit_trigger_function()` in migration 011 independently logs all changes regardless of whether the application layer recorded them.

---

## Principle 4: Design-First Workflow (Business -> Data -> Technology)

Every feature follows a three-phase design workflow:

- **Phase 1: Business Process** — define the process, actors, inputs, outputs
- **Phase 2: Data Architecture** — identify data elements, register metadata, assign ownership
- **Phase 3: Technology / Code** — implementation follows from Phases 1-2

Technology decisions are NEVER made before understanding the business process and data architecture.

**Why:** Most failed IT projects start with technology. The metadata tool enforces this discipline both internally (how we build it) and externally (the workflow it provides to users).

**Enforcement:**

- The database schema itself follows this sequence: `business_processes` and `process_steps` (migration 008) model the business process with steps, actors (`responsible_role`), and inputs/outputs (`input_data_elements`, `output_data_elements`). `data_elements` (migration 004) and `process_data_elements` (migration 008) model the data architecture — linking elements to processes with usage types (INPUT, OUTPUT, BOTH). Only then do `technical_schemas`, `technical_tables`, and `technical_columns` (migration 004) capture the technology layer.
- The API design reflects this hierarchy: `/api/v1/processes` endpoints allow creating business processes and linking them to data elements before any technical column mapping exists. Data elements can be fully defined at the business level (`element_name`, `business_definition`, `business_rules`) before any technical column is linked via `element_id`.
- The `process_steps` table requires `step_name` and `description` but makes `application_id` optional — a process can be fully documented without any technology decisions.
- Internal development follows the same discipline: new features start with a process description in a design document, then data model changes in migrations, then Rust implementation. This is documented in `CLAUDE.md` under "Key Architecture Decisions" and enforced through code review.

---

## Principle 5: Workflow-Governed Metadata

All metadata entities follow a governed lifecycle: Draft -> Proposed -> Under Review -> Accepted/Revised/Rejected -> Deprecated. No metadata is published without going through its appropriate approval workflow. Workflows are configurable per entity type.

**Why:** Ungoverned metadata is unreliable metadata. Every business term, data element, quality rule, application, and business process must be reviewed and approved by the appropriate steward.

**Enforcement:**

- Migration `backend/migrations/009_workflow.sql` implements the complete workflow engine:
  - `workflow_states` defines the state machine with seven states (DRAFT, PROPOSED, UNDER_REVIEW, REVISED, ACCEPTED, REJECTED, DEPRECATED), each marked as `is_initial` or `is_terminal`.
  - `workflow_entity_types` registers five governed entity types: GLOSSARY_TERM, DATA_ELEMENT, QUALITY_RULE, APPLICATION, and BUSINESS_PROCESS.
  - `workflow_definitions` allows configurable workflow definitions per entity type, including `review_sla_hours`.
  - `workflow_transitions` defines permitted state transitions with `from_state_id`, `to_state_id`, `action_code`, and an optional `required_role_id` restricting who can perform each transition.
  - `workflow_approvers` supports multi-level approval chains with `approval_order` and `is_mandatory` flags.
- Every domain entity has a `status_id` foreign key referencing `entity_statuses`, tying it to the workflow state machine.
- `workflow_instances` track active workflows per entity, and `workflow_tasks` assign review tasks to specific users or roles. `workflow_history` records every transition with `performed_by`, `performed_at`, and `comments` — providing a complete audit trail of the governance process.
- API endpoints in `backend/src/api/workflow.rs` expose task management (`/api/v1/workflow/tasks/pending`), instance inspection (`/api/v1/workflow/instances/{instance_id}`), state transition (`/api/v1/workflow/instances/{instance_id}/transition`), and task completion (`/api/v1/workflow/tasks/{task_id}/complete`).
- The domain types in `backend/src/domain/workflow.rs` define `WorkflowTransitionRequest` (with `action` and `comments`) and `CompleteTaskRequest` (with `decision` and `comments`), ensuring transitions always carry context.

---

## Principle 6: AI-Assisted, Human-Governed

AI (Claude/OpenAI) provides suggestions for metadata enrichment — definitions, classifications, descriptions based on financial services standards. AI NEVER auto-publishes metadata. All AI suggestions are presented to users for review, acceptance, modification, or rejection. AI operates on the suggestion layer only.

**Why:** AI can overcome knowledge gaps and improve consistency, but metadata governance requires human accountability. The tool uses external AI APIs (not local inference) to leverage the best available models.

**Enforcement:**

- The `ai_suggestions` table (migration `backend/migrations/011_ai_and_audit.sql`) stores every AI-generated suggestion with `status` constrained to `CHECK(status IN ('PENDING','ACCEPTED','REJECTED','MODIFIED'))`. The default status is `PENDING` — no suggestion is ever inserted as ACCEPTED.
- Each suggestion records its `source` (CLAUDE or OPENAI), `model`, `confidence` score, and `rationale`, giving reviewers full transparency into how the suggestion was generated.
- The `accepted_by` and `accepted_at` columns are `NULL` on creation and only populated when a human user explicitly accepts the suggestion. This creates an auditable chain of accountability.
- The `ai_feedback` table captures user ratings (1-5) and feedback text per suggestion, creating a feedback loop for improving AI quality.
- The AI enrichment API endpoint (`POST /api/v1/ai/enrich` in `backend/src/api/ai.rs`) returns an `AiEnrichResponse` containing a `Vec<AiSuggestion>` — a list of suggestions, not modifications. The response includes `field_name`, `suggested_value`, `confidence`, and `rationale` for each suggestion. The API never writes directly to domain tables.
- The `AiConfig` in `backend/src/config.rs` configures Claude as the primary provider with OpenAI as fallback, supporting `primary_provider`, model selection, and separate API keys for each.
- Per-entity enrichment endpoints (e.g., `POST /api/v1/glossary/terms/{term_id}/ai-enrich`) follow the same pattern: generate suggestions, store them as PENDING, return them for human review.

---

## Principle 7: Rust Only (Backend)

The entire backend is coded in Rust only. No polyglot runtime, no scripting sidecars. Rust is chosen for its type safety, memory safety, performance, and ability to encode business invariants at compile time.

Frontend is TypeScript/React — the only exception to "Rust only" and it is a clearly separated concern.

**Why:** Type safety catches entire categories of bugs at compile time. A metadata platform must be reliable.

**Enforcement:**

- The workspace `Cargo.toml` at the project root defines a single workspace member: `backend`. All backend logic — API handlers, domain types, authentication, workflow, naming validation, AI integration, notifications, error handling — lives in Rust crates under `backend/src/`.
- The backend `Cargo.toml` (`backend/Cargo.toml`) lists only Rust dependencies: `axum` for HTTP, `sqlx` for database access, `utoipa` for OpenAPI, `jsonwebtoken` and `openidconnect` for auth, `reqwest` for external HTTP calls (AI APIs), `validator` for input validation, and `regex` for naming patterns. There are no Python, Node.js, or scripting dependencies.
- The `backend/src/lib.rs` module tree exposes `api`, `auth`, `config`, `db`, `domain`, `error`, `ai`, `naming`, `notifications`, and `workflow` — all implemented in Rust.
- The frontend is physically separated in the `frontend/` directory with its own `package.json`, `vite.config.ts`, and `tsconfig.json`. It communicates with the backend exclusively through the REST API. The Vite dev server proxies `/api` to the backend — there is no shared runtime.
- Database migrations in `backend/migrations/*.sql` are the only non-Rust files in the backend, and they are executed by `sqlx::migrate!` within the Rust binary at startup.

---

## Principle 8: Naming Standards Enforcement

Technical metadata (table names, column names, schema names, API paths, keys, triggers) must conform to configurable naming standards. Standards are enforced at the application layer (validation), database layer (triggers), and are configurable via the `naming_standards` table.

**Why:** Inconsistent naming is the first sign of ungoverned metadata. The tool enforces what it preaches.

**Enforcement:**

- The `naming_standards` table (migration `backend/migrations/004_data_dictionary.sql`) stores configurable standards with `applies_to` (TABLE, COLUMN, SCHEMA, API, KEY, TRIGGER), `pattern_regex`, `is_mandatory`, `example_valid`, and `example_invalid`. Eight default standards are seeded: snake_case for tables, columns, and schemas; kebab-case for API paths; suffix rules for PKs, FKs, timestamps; and prefix rules for booleans.
- **Database layer:** The `validate_column_naming_standards()` trigger function (migration `backend/migrations/012_meta_and_triggers.sql`) fires `BEFORE INSERT OR UPDATE OF column_name ON technical_columns`. It iterates over all mandatory standards where `applies_to = 'COLUMN'`, tests the column name against each `pattern_regex`, and sets `naming_standard_compliant` (boolean) and `naming_standard_violation` (text listing violated standards) directly on the row. This enforcement cannot be bypassed by any application code.
- **Application layer:** The `naming` module (`backend/src/naming/mod.rs`) provides `validate_name(name, entity_type, standards)` which returns a `NamingValidationResult` containing a boolean `is_compliant` and a vector of `NamingViolation` structs. Each violation includes the standard name, a descriptive message, and an optional suggestion. API handlers call this function before database writes, returning `AppError::NamingViolation` (HTTP 422) on failure.
- The tool's own schema follows these standards: all table names (`glossary_terms`, `data_elements`, `quality_rules`), column names (`created_at`, `is_cde`, `owner_user_id`), and API paths (`/api/v1/data-dictionary/elements`, `/api/v1/data-quality/rules`) conform to the seeded naming standards. The tool practises what it enforces.

---

## Principle 9: Audit Everything

Every change to every metadata entity is recorded in the audit trail: who changed what, when, before/after values. Workflow transitions, task completions, AI suggestions, and login events are all logged. The audit trail is append-only.

**Why:** Regulatory compliance and accountability. In financial services, auditability is non-negotiable.

**Enforcement:**

- The `audit_log` table (migration `backend/migrations/011_ai_and_audit.sql`) records every INSERT, UPDATE, and DELETE with `table_name`, `record_id`, `action`, `old_values` (JSONB), `new_values` (JSONB), `changed_fields`, `changed_by`, `changed_at`, `ip_address`, and `user_agent`. This table has no `UPDATE` or `DELETE` API — it is append-only by design.
- The `audit_trigger_function()` (migration 011) is a generic PostgreSQL trigger that can be attached to any table. It captures `TG_OP` (INSERT/UPDATE/DELETE), serialises the old and new row as JSONB, and reads the current user from `app.current_user_id` session variable. This ensures auditing happens at the database level, independent of application code.
- The `login_audit_log` table captures authentication events: `event_type`, `user_id`, `attempted_username`, `ip_address`, `user_agent`, `success`, and `failure_reason`. Both successful and failed login attempts are recorded.
- Workflow history is separately tracked in `workflow_history` (migration 009), recording every state transition with `from_state_id`, `to_state_id`, `action`, `performed_by`, `performed_at`, and `comments`.
- AI suggestion lifecycle is audited through the `ai_suggestions` table itself — `status` transitions from PENDING to ACCEPTED/REJECTED/MODIFIED are recorded with `accepted_by` and `accepted_at`, and `ai_feedback` captures user ratings.
- Indexes on `audit_log` support efficient querying: `idx_audit_log_table` (by table name), `idx_audit_log_record` (by table + record), `idx_audit_log_user` (by who changed), and `idx_audit_log_date` (by when, descending). Similar indexes exist on `login_audit_log`.

---

## Principle 10: Secure-by-Design

- Authentication on every API endpoint (Microsoft Entra ID SSO + JWT)
- Role-based access control at the domain level
- Input validation at trust boundaries
- Parameterized SQL queries only (never string interpolation)
- No PII in logs, error messages, or API error responses
- Structured tracing for observability
- CORS restricted to configured frontend origin

**Why:** A metadata management tool contains sensitive information about an organization's data landscape. Security is not an afterthought.

**Enforcement:**

- **Authentication:** The `require_auth` middleware in `backend/src/auth/middleware.rs` intercepts every protected request, validates the `Authorization: Bearer <JWT>` header, and rejects requests without valid tokens with `AppError::Unauthorized` (HTTP 401). SSO is implemented via OpenID Connect with Microsoft Entra ID (`openidconnect` crate in `backend/Cargo.toml`), configured through `EntraConfig` in `backend/src/config.rs` with `tenant_id`, `client_id`, `client_secret`, and `redirect_uri`.
- **RBAC:** The `require_role` middleware in `backend/src/auth/middleware.rs` checks user roles against the required roles for each endpoint. Roles are stored in the `roles` table and linked to users. Workflow transitions can require specific roles via `workflow_transitions.required_role_id`.
- **Parameterized queries:** SQLx (`sqlx` crate with `postgres` feature) enforces parameterized queries at compile time. The `sqlx::query!` and `sqlx::query_as!` macros verify SQL against the actual database schema during compilation, making SQL injection structurally impossible.
- **Error handling:** The `AppError` enum in `backend/src/error.rs` maps internal errors to safe HTTP responses. Database errors return `"DATABASE_ERROR"` without leaking schema details. Internal errors return `"INTERNAL_ERROR"` without stack traces. No user IDs, emails, or other PII appear in error responses — only structured error codes and sanitised messages.
- **CORS:** The `CorsLayer` in `backend/src/main.rs` restricts `allow_origin` to `config.frontend_url` (loaded from the `FRONTEND_URL` environment variable). Cross-origin requests from any other origin are rejected.
- **Tracing:** The `tracing` and `tracing-subscriber` crates (with `env-filter` and `json` features) provide structured logging. The `TraceLayer` from `tower-http` is applied to all routes for HTTP-level observability. Log levels are controlled via the `RUST_LOG` environment variable.
- **Secrets management:** API keys (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`), JWT secrets, and Entra credentials are loaded from environment variables via `backend/src/config.rs`, never hardcoded. The `.env` file is listed in `.gitignore`.

---

## Principle 11: Single Source of Truth (Create Once, Use Many)

Every piece of metadata is defined once and referenced everywhere. Business glossary terms are defined in the glossary and linked to data elements. Data elements are defined in the dictionary and linked to applications, processes, quality rules, and lineage graphs. No duplication.

**Why:** Duplicated metadata diverges. The tool enforces referential integrity between all metadata domains.

**Enforcement:**

- **Glossary -> Dictionary:** `data_elements.glossary_term_id` is a foreign key to `glossary_terms.term_id` (migration 004). A business term is defined once in the glossary and linked to every data element that represents it. The `DataElementFullView` in `backend/src/domain/data_dictionary.rs` includes `glossary_term_name`, resolved via JOIN — the term name is never duplicated into the data element.
- **Dictionary -> Quality:** `quality_rules.element_id` references `data_elements.element_id` (migration 005). Quality rules are defined once and linked to the data element they govern. `quality_rules.column_id` further links to `technical_columns` for column-level rules.
- **Dictionary -> Processes:** `process_data_elements` is a junction table linking `business_processes` to `data_elements` with a `UNIQUE(process_id, element_id)` constraint (migration 008). Each linkage specifies `usage_type` (INPUT/OUTPUT/BOTH) without duplicating the element's definition.
- **Dictionary -> Applications:** `data_elements` link to `applications` through lineage graphs and process-application associations. `process_applications` links processes to applications (migration 008).
- **Dictionary -> Lineage:** `lineage_nodes` reference data elements, applications, and processes by ID (migration 006). Lineage graphs compose existing entities by reference, never by copy.
- **Domains as shared reference:** `glossary_domains` is shared between glossary terms and data elements via `domain_id` foreign keys. Domains are defined once and used across entity types.
- **Statuses as shared reference:** `entity_statuses` (migration 003) provides a single status table shared by all domain entities via `status_id`, ensuring consistent workflow states without per-entity status duplication.

---

## Principle 12: Critical Data Element Propagation

All data elements linked to a Critical Business Process are automatically designated as Critical Data Elements (CDEs). This is enforced via database triggers and cannot be bypassed. CDE designation propagates automatically when processes are marked critical or when elements are linked to existing critical processes.

**Why:** In financial services, regulatory frameworks (BCBS 239) require that critical data supporting critical business processes is identified, governed, and quality-assured.

**Enforcement:**

- **Trigger on process criticality change:** The `auto_designate_cde_for_critical_process()` function and `trg_critical_process_cde` trigger (migration `backend/migrations/008_processes.sql`) fire `AFTER UPDATE OF is_critical ON business_processes`. When `is_critical` changes from FALSE to TRUE, the trigger updates all linked `data_elements` via `process_data_elements`, setting `is_cde = TRUE`, appending a rationale string (`'Auto-designated: linked to critical business process "<name>"'`), and recording `cde_designated_at`.
- **Trigger on element-process linkage:** The `auto_designate_cde_on_process_link()` function and `trg_process_link_cde` trigger (migration 008) fire `AFTER INSERT ON process_data_elements`. When a data element is newly linked to a process that is already critical (`is_critical = TRUE`), the trigger designates that element as a CDE with the same rationale pattern.
- **Cannot be bypassed:** Both triggers execute at the PostgreSQL level, independent of the application. Direct SQL inserts, bulk imports, and API calls all pass through these triggers. There is no application-level flag to suppress them.
- **Data model support:** The `data_elements` table (migration 004) has dedicated CDE columns: `is_cde` (boolean, default FALSE), `cde_rationale` (text, supports multiple rationale entries separated by semicolons), `cde_designated_at` (timestamp), and `cde_designated_by` (user reference). A partial index `idx_data_elements_cde` on `is_cde = TRUE` supports efficient CDE listing.
- **API exposure:** The `GET /api/v1/data-dictionary/elements/cde` endpoint (registered in `backend/src/main.rs`, implemented in `backend/src/api/data_dictionary.rs`) lists all CDEs, making the propagation results immediately visible to governance users.

---

## Principle 13: AI-Maintained Codebase

The codebase is primarily maintained by AI (Claude Code). Every piece of code must be written so that an AI reading it for the first time has all context needed to understand, extend, and modify it correctly. This requires comprehensive documentation at every level: crates, modules, types, traits, functions, fields.

**Why:** Consistent, comprehensive documentation enables AI to produce correct code and prevents knowledge loss.

**Enforcement:**

- `CLAUDE.md` at the project root provides the AI maintainer with a complete overview of the project architecture, module structure, build commands, naming conventions, and key design decisions. This file is the entry point for every AI coding session.
- `METADATA_TOOL_PRINCIPLES.md` (this file) documents the non-negotiable principles so the AI maintainer understands the constraints that govern every decision.
- Every module file includes a doc comment explaining its purpose. For example, `backend/src/naming/mod.rs` opens with `// Naming standards enforcement module` and `// Validates technical metadata names against configurable patterns`. The `backend/src/ai/mod.rs` explains `// AI integration module - Claude (primary) and OpenAI (fallback)` and `// Provides metadata enrichment suggestions based on financial services standards`.
- Domain types document their fields through descriptive naming that follows Principle 8. Struct-level doc comments in `backend/src/domain/*.rs` explain the purpose (e.g., `/// Response combining business and technical metadata for a data element` on `DataElementFullView`, `/// Pending tasks for the current user's dashboard` on `PendingTaskView`).
- SQL migrations in `backend/migrations/*.sql` include comments explaining the purpose of each table, trigger, and function. Migration files are numbered sequentially (001 through 012) so the schema evolution is self-documenting.
- The `AppError` enum in `backend/src/error.rs` uses descriptive variant names (`NamingViolation`, `Workflow`, `AiService`) that make error handling self-explanatory without requiring external documentation.
- All API routes in `backend/src/main.rs` are grouped by domain with inline comments (`// Health`, `// Auth`, `// Business Glossary`, etc.), making the routing table scannable.

---

## Principle 14: Bank-Agnostic Design

The tool is designed for any financial institution. No proprietary bank information, branding, design elements, or intellectual property is used. All terminology follows industry standards (DAMA, BCBS 239, ISO). Demo data uses generic financial services examples.

**Why:** The tool's value is in its universality. Brand-neutral design ensures broad applicability.

**Enforcement:**

- The OpenAPI spec in `backend/src/main.rs` identifies the tool as `"Metadata Management Tool"` with the description `"Enterprise metadata lifecycle management for data governance"`. No bank name, logo, or proprietary term appears anywhere in the codebase.
- Data classification levels in migration 004 follow the generic industry standard: PUBLIC, INTERNAL, CONFIDENTIAL, RESTRICTED — not any bank's specific classification taxonomy.
- Quality dimensions in migration 005 follow the DAMA standard: Completeness, Uniqueness, Validity, Timeliness, Accuracy, Consistency.
- The `business_processes` table includes `regulatory_requirement` as a free-text field, supporting any regulatory framework (BCBS 239, GDPR, SOX, MiFID II) without hardcoding a specific regime.
- The frontend uses a deep navy primary colour (#1B3A5C) as defined in `frontend/src/theme/themeConfig.ts` — a professional, brand-neutral choice documented in `CLAUDE.md`.
- Naming standard patterns in migration 004 are generic industry conventions (snake_case for tables/columns, kebab-case for APIs) and are configurable through the `naming_standards` table — institutions can add their own standards without modifying code.
- The dual license (MIT OR Apache-2.0, declared in workspace `Cargo.toml`) ensures the tool can be adopted by any institution without proprietary licensing constraints.
- `CLAUDE.md` explicitly states: *"This tool is bank-agnostic — designed for any financial institution. No proprietary bank IP is used."* This directive governs all code contributions.
