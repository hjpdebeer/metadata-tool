# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Enterprise metadata lifecycle management tool for financial institutions. Multi-user, role-based, workflow-driven platform for governing business glossary terms, data dictionaries, data quality rules, data lineage, business applications, and business processes.

**Owner**: Hendrik de Beer (hjpdebeer@protonmail.com) — sole author, all IP retained.

## Foundational Documents

These documents govern all design decisions and code contributions. Read them before making changes:

- **[METADATA_TOOL_PRINCIPLES.md](METADATA_TOOL_PRINCIPLES.md)** — 14 non-negotiable foundational principles. Every change must comply.
- **[CODING_STANDARDS.md](CODING_STANDARDS.md)** — Naming conventions, module organisation, type design, error handling, testing, database conventions, and pre-commit checklist.
- **[docs/architecture/decisions/](docs/architecture/decisions/)** — Architecture Decision Records (ADRs) explaining key design choices.

## Architecture

**Modular monolith** (ADR-0001) — Rust backend with clearly separated domain modules, React SPA frontend.

- **Backend** (`backend/`): Rust, Axum web framework, SQLx for PostgreSQL, utoipa for OpenAPI
- **Frontend** (`frontend/`): React 19 + TypeScript + Vite + Ant Design + React Flow (lineage visualization)
- **Database**: PostgreSQL 17, 3NF normalized, 12 migration files in `backend/migrations/`
- **Auth**: Microsoft Entra ID SSO via OpenID Connect, JWT sessions, RBAC
- **AI**: Claude API (primary), OpenAI (fallback) for metadata enrichment (ADR-0004)

## Build & Run Commands

```bash
# Backend
cd backend && cargo build                                        # Build
cd backend && cargo run                                          # Run (auto-migrates DB)
cd backend && cargo test                                         # Run all tests
cd backend && cargo test test_name                               # Run single test
cd backend && cargo clippy --all-targets -- -D warnings          # Lint
cd backend && cargo fmt                                          # Format
cd backend && cargo fmt -- --check                               # Check formatting

# Frontend
cd frontend && npm install                                       # Install deps
cd frontend && npm run dev                                       # Dev server (port 5173, proxies /api to 8080)
cd frontend && npm run build                                     # Production build (type check + bundle)

# Database
docker compose up -d                                             # Start PostgreSQL
docker compose down                                              # Stop PostgreSQL

# Pre-commit (run all before committing — see CODING_STANDARDS.md Section 14)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
cd frontend && npm run build
```

### Required Dev Tools

```bash
rustup update stable                    # Latest stable Rust
cargo install cargo-deny cargo-audit    # Dependency audit tools
brew install postgresql@17              # Or use docker compose
```

## Key Architecture Decisions

### Backend module structure (Principle 2, 13)
Each domain (glossary, data_dictionary, data_quality, lineage, applications, processes, workflow) has:
- `domain/{module}.rs` — Rust structs (models, request/response types) with SQLx `FromRow` and utoipa `ToSchema` derives
- `api/{module}.rs` — Axum route handlers with utoipa OpenAPI annotations
- Routes registered in `main.rs`

Shared app state (`db::AppState`) holds the PgPool and AppConfig, passed to all handlers via Axum's `State` extractor.

### Database schema (Principle 2, 3, 9)
- 12 sequential migrations in `backend/migrations/` (001_extensions through 012_meta_and_triggers)
- UUIDs for all primary keys (`gen_random_uuid`)
- Soft deletes via `deleted_at` column where appropriate
- Full-text search via `TSVECTOR` columns on glossary_terms and data_elements
- `entity_statuses` table shared across all domain entities for workflow states
- Naming standards enforced via DB trigger on `technical_columns` (Principle 8)
- Critical Business Process → auto-CDE designation via DB trigger (Principle 12, ADR-0005)
- Self-describing metadata in `meta_tables` and `meta_columns` (Principle 2)

### Workflow engine (Principle 5, ADR-0003)
Generic workflow engine supporting all entity types. State machine: Draft → Proposed → Under Review → Accepted/Revised/Rejected/Deprecated. Workflow definitions, transitions, instances, tasks, and history. Tasks assigned to users or roles.

### API design (Principle 1, ADR-0002)
- All routes under `/api/v1/`
- OpenAPI spec auto-generated via utoipa, served at `/swagger-ui/`
- JWT Bearer auth on all protected routes (Principle 10)
- Consistent error responses via `AppError` enum in `error.rs`
- Pagination: `page` + `page_size` query params

### Frontend
- Ant Design component library with custom theme in `src/theme/themeConfig.ts`
- Deep navy (#1B3A5C) primary color — professional, brand-neutral (Principle 14)
- React Router for client-side routing
- Axios client with JWT interceptor in `src/services/api.ts`
- Vite dev server proxies `/api` to backend at port 8080

## Dependency Management

All external dependencies are declared in the root `Cargo.toml` under `[workspace.dependencies]`. Backend crate references them with `workspace = true`. Never add dependency versions directly to `backend/Cargo.toml`.

## Environment Configuration

Copy `.env.example` to `.env`. Required variables:
- `DATABASE_URL` — PostgreSQL connection string
- `JWT_SECRET` — Secret for signing JWT tokens
- `ENTRA_*` — Microsoft Entra ID SSO credentials
- `GRAPH_*` — Microsoft Graph API for email notifications
- `ANTHROPIC_API_KEY` — For AI metadata enrichment (primary)
- `OPENAI_API_KEY` — Fallback AI provider

## Naming Conventions (Summary)

Full rules in CODING_STANDARDS.md Section 1. Key points:

**Rust**: RFC 430 compliant. Acronyms are single words in PascalCase (`Cde`, `Api`, `Sso`, `Jwt`, not `CDE`, `API`, `SSO`, `JWT`). Exception: `SCREAMING_SNAKE_CASE` constants keep full caps.

**Database**: `snake_case` for all objects. `_id` suffix for keys, `_at` for timestamps, `is_`/`has_` prefix for booleans. Enforced via `naming_standards` table and DB triggers.

**API paths**: `kebab-case` under `/api/v1/`.

**Frontend**: PascalCase components, camelCase functions/variables, kebab-case CSS.

## Git Workflow

- Branch naming: `feature/`, `fix/`, `docs/`, `refactor/`, `chore/` + kebab-case
- Commit messages: conventional commits (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`, `test:`)
- No direct commits to `main` — all changes through pull requests
- PR template auto-populates principles compliance checklist

## Important Notes

- This tool is **bank-agnostic** (Principle 14) — designed for any financial institution. No proprietary bank IP is used anywhere.
- AI enrichment generates suggestions based on **financial services standard definitions** (Principle 6) — suggestions require human review.
- The `workflow` module constants are in `backend/src/workflow/mod.rs` — reference these when building workflow logic rather than hardcoding state strings.
- All domain entities go through the same workflow lifecycle (Principle 5) — new entity types should follow the same pattern.
- CDE propagation is automatic via DB triggers (Principle 12) — never implement CDE logic in application code that bypasses the triggers.
