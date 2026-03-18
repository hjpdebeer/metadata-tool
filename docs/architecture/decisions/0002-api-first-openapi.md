# ADR-0002: API-First Design with Generated OpenAPI

**Status:** Accepted

**Date:** 2026-03-18

## Context

The metadata management platform must support two primary interaction patterns:

1. **UI interaction:** Human users browsing, editing, and approving metadata through a web interface.
2. **Programmatic access:** Discovery tools ingesting metadata (e.g., schema crawlers pushing data element definitions), design tools reading metadata (e.g., ETL tools querying lineage), and CI/CD pipelines triggering quality rule validation.

API contracts must stay in sync with the implementation. Hand-maintained OpenAPI specs drift from code over time, leading to broken integrations and incorrect documentation.

## Decision

Use **utoipa** to generate OpenAPI 3.1 specifications directly from Rust types and endpoint definitions at compile time. The generated spec is served alongside the application:

- **Swagger UI** is available at `/swagger-ui/` for interactive exploration and testing.
- **Raw OpenAPI spec** is available at `/api-doc/openapi.json` for programmatic consumption.
- All functionality is available via the **REST API before any UI is built**. The UI is a consumer of the API, not a bypass around it.

API design conventions:

- Resource-oriented URLs (`/api/v1/glossary/terms`, `/api/v1/data-dictionary/elements`)
- Standard HTTP methods and status codes
- Consistent pagination, filtering, and error response formats
- Versioned API prefix (`/api/v1/`)

## Consequences

### Positive

- **API contracts are always accurate:** The spec is generated from the same types that handle requests. If a field is added to a struct, it appears in the spec automatically.
- **Frontend independence:** The web UI and external tools can be developed against the spec without waiting for backend completion.
- **External tool integration:** Discovery and design tools can auto-generate clients from the OpenAPI spec.
- **Interactive documentation:** Swagger UI provides a zero-effort, always-current API explorer.

### Negative

- **utoipa macro overhead:** Derive macros add compile-time cost and require annotation discipline on every endpoint and type.
- **API-first discipline required:** New features must always start with the API endpoint definition, even if only the UI will use it initially. This is a positive constraint but requires enforcement.

### Neutral

- **No separate spec file to maintain:** There is no standalone `openapi.yaml` in the repository. The spec exists only as generated output. This eliminates drift but means the spec is not directly editable.
