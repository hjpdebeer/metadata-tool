# Metadata Management Tool

An enterprise metadata lifecycle management platform for financial institutions. Enables Data Stewards, Data Producers, and Data Consumers to define, govern, and track metadata across business and technical domains.

## Features

- **Business Glossary** — Define and govern business terms with approval workflows
- **Data Dictionary** — Manage business and technical metadata; identify Critical Data Elements (CDEs)
- **Data Quality** — Define quality rules across 6 dimensions (Completeness, Uniqueness, Validity, Timeliness, Accuracy, Consistency)
- **Data Lineage** — Capture and visualize both business and technical data lineage
- **Business Application Registry** — Inventory applications with ownership, classification, and data element links
- **Business Process Registry** — Document processes; critical processes auto-designate CDEs
- **Workflow Engine** — Configurable approval workflows (Draft → Proposed → Review → Accepted/Revised/Rejected)
- **Role-Based Access Control** — Data Owner, Data Steward, Data Producer, Data Consumer, and more
- **SSO Integration** — Microsoft Entra ID (Azure AD) via OpenID Connect
- **Email Notifications** — Microsoft Graph API integration for Outlook
- **AI-Assisted Metadata** — Claude (primary) / OpenAI (fallback) for auto-populating definitions and classifications
- **API-First Design** — Full REST API with OpenAPI/Swagger documentation
- **Naming Standards Enforcement** — Configurable rules for tables, columns, schemas, and APIs

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Axum, SQLx, Tower |
| Database | PostgreSQL 17 (3NF normalized, 67+ tables) |
| Frontend | React 19, TypeScript, Vite, Ant Design, React Flow |
| Auth | OpenID Connect (Microsoft Entra ID), JWT |
| Email | Microsoft Graph API |
| AI | Anthropic Claude API, OpenAI API |
| API Docs | utoipa + Swagger UI |

## Prerequisites

- Rust (latest stable)
- Node.js 20+
- PostgreSQL 17+ (or Docker)
- Microsoft Entra ID tenant (for SSO)
- Anthropic API key (for AI features)

## Quick Start

```bash
# Start PostgreSQL
docker compose up -d

# Configure environment
cp .env.example .env
# Edit .env with your credentials

# Run backend (auto-runs migrations)
cd backend
cargo run

# In another terminal — run frontend
cd frontend
npm install
npm run dev
```

The API will be at `http://localhost:8080` with Swagger UI at `http://localhost:8080/swagger-ui/`.
The frontend will be at `http://localhost:5173`.

## Project Structure

```
metadata-tool/
├── backend/                 # Rust API server
│   ├── src/
│   │   ├── main.rs          # Server entry point, route registration
│   │   ├── api/             # HTTP route handlers per domain
│   │   ├── domain/          # Domain models and request/response types
│   │   ├── db/              # Database pool and shared state
│   │   ├── auth/            # JWT validation, RBAC middleware
│   │   ├── workflow/        # Workflow state machine constants
│   │   ├── naming/          # Naming standards validation
│   │   ├── ai/              # AI integration (Claude/OpenAI)
│   │   ├── notifications/   # Email via Microsoft Graph
│   │   └── error.rs         # Unified error handling
│   └── migrations/          # PostgreSQL migrations (001-012)
├── frontend/                # React + TypeScript SPA
│   └── src/
│       ├── App.tsx           # Root routes
│       ├── layouts/          # Page layout with sidebar
│       ├── pages/            # Page components
│       ├── services/         # API client (axios)
│       └── theme/            # Ant Design theme configuration
├── docker-compose.yml       # PostgreSQL for local dev
├── .env.example             # Environment variable template
├── LICENSE-MIT
└── LICENSE-APACHE
```

## License

Dual-licensed under MIT and Apache 2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

Copyright (c) 2026 Hendrik de Beer <hjpdebeer@protonmail.com>
