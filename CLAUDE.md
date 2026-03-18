# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Enterprise metadata lifecycle management tool for financial institutions. Multi-user, role-based, workflow-driven platform for governing business glossary terms, data dictionaries, data quality rules, data lineage, business applications, and business processes.

**Owner**: Hendrik de Beer (hjpdebeer@protonmail.com) — sole author, all IP retained.

## Architecture

**Modular monolith** — Rust backend with clearly separated domain modules, React SPA frontend.

- **Backend** (`backend/`): Rust, Axum web framework, SQLx for PostgreSQL, utoipa for OpenAPI
- **Frontend** (`frontend/`): React 19 + TypeScript + Vite + Ant Design + React Flow (lineage visualization)
- **Database**: PostgreSQL 17, 3NF normalized, 12 migration files in `backend/migrations/`
- **Auth**: Microsoft Entra ID SSO via OpenID Connect, JWT sessions, RBAC
- **AI**: Claude API (primary), OpenAI (fallback) for metadata enrichment

## Build & Run Commands

```bash
# Backend
cd backend && cargo build            # Build
cd backend && cargo run              # Run (auto-migrates DB)
cd backend && cargo test             # Run tests
cd backend && cargo clippy           # Lint
cd backend && cargo fmt              # Format

# Frontend
cd frontend && npm install           # Install deps
cd frontend && npm run dev           # Dev server (port 5173, proxies /api to 8080)
cd frontend && npm run build         # Production build
cd frontend && npm run preview       # Preview production build

# Database
docker compose up -d                 # Start PostgreSQL
docker compose down                  # Stop PostgreSQL
```

## Key Architecture Decisions

### Backend module structure
Each domain (glossary, data_dictionary, data_quality, lineage, applications, processes, workflow) has:
- `domain/{module}.rs` — Rust structs (models, request/response types) with SQLx `FromRow` and utoipa `ToSchema` derives
- `api/{module}.rs` — Axum route handlers with utoipa OpenAPI annotations
- Routes registered in `main.rs`

Shared app state (`db::AppState`) holds the PgPool and AppConfig, passed to all handlers via Axum's `State` extractor.

### Database schema
- 12 sequential migrations in `backend/migrations/` (001_extensions through 012_meta_and_triggers)
- UUIDs for all primary keys (gen_random_uuid)
- Soft deletes via `deleted_at` column where appropriate
- Full-text search via `TSVECTOR` columns on glossary_terms and data_elements
- `entity_statuses` table shared across all domain entities for workflow states
- Naming standards enforced via DB trigger on `technical_columns`
- Critical Business Process → auto-CDE designation via DB trigger on `business_processes` and `process_data_elements`
- Self-describing metadata in `meta_tables` and `meta_columns`

### Workflow
Generic workflow engine supporting all entity types. State machine: Draft → Proposed → Under Review → Accepted/Revised/Rejected/Deprecated. Workflow definitions, transitions, instances, tasks, and history. Tasks assigned to users or roles.

### API design
- All routes under `/api/v1/`
- OpenAPI spec auto-generated via utoipa, served at `/swagger-ui/`
- JWT Bearer auth on all protected routes
- Consistent error responses via `AppError` enum in `error.rs`

### Frontend
- Ant Design component library with custom theme in `src/theme/themeConfig.ts`
- Deep navy (#1B3A5C) primary color — professional, brand-neutral
- React Router for client-side routing
- Axios client with JWT interceptor in `src/services/api.ts`
- Vite dev server proxies `/api` to backend at port 8080

## Environment Configuration

Copy `.env.example` to `.env`. Required variables:
- `DATABASE_URL` — PostgreSQL connection string
- `JWT_SECRET` — Secret for signing JWT tokens
- `ENTRA_*` — Microsoft Entra ID SSO credentials
- `GRAPH_*` — Microsoft Graph API for email notifications
- `ANTHROPIC_API_KEY` — For AI metadata enrichment (primary)
- `OPENAI_API_KEY` — Fallback AI provider

## Naming Standards

Technical metadata naming enforced both at DB level (triggers) and API level. Configurable in `naming_standards` table. Defaults:
- Tables/columns/schemas: `snake_case`
- API paths: `kebab-case`
- PKs/FKs end with `_id`
- Booleans prefixed with `is_`/`has_`/`can_`
- Timestamps end with `_at`/`_date`/`_time`

## Important Notes

- This tool is **bank-agnostic** — designed for any financial institution. No proprietary bank IP is used.
- AI enrichment generates suggestions based on **financial services standard definitions** (not bank-specific).
- The `workflow` module constants are in `backend/src/workflow/mod.rs` — reference these when building workflow logic rather than hardcoding state strings.
- All domain entities go through the same workflow lifecycle — new entity types should follow the same pattern.
