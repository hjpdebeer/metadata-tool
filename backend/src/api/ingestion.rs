//! Technical metadata ingestion endpoint for automated schema crawlers.
//!
//! Handles batch upsert of hierarchical technical metadata (source systems,
//! schemas, tables, columns, relationships) from external tools such as
//! database crawlers and schema introspection scripts.
//!
//! **Design decisions:**
//! - Auto-accept: ingested technical metadata does NOT go through workflow.
//!   It is structural metadata from trusted sources (API keys).
//! - Stale detection: updates `last_seen_at` on all touched records. Records
//!   NOT in the payload keep their old `last_seen_at`.
//! - Upsert by natural keys: system_code, (system_id, schema_name),
//!   (schema_id, table_name), (table_id, column_name).
//! - Naming validation: runs naming standards on columns (same as
//!   `create_column` in `data_dictionary.rs`).
//! - Partial success: individual errors do not abort the whole operation.

use std::collections::HashMap;
use std::time::Instant;

use axum::Extension;
use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::naming;

// ===========================================================================
// Request types
// ===========================================================================

/// Top-level request body for the technical metadata ingestion endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestTechnicalRequest {
    pub source_system: IngestSourceSystem,
    pub options: Option<IngestOptions>,
}

/// A source system with its full schema/table/column hierarchy to ingest.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestSourceSystem {
    pub system_code: String,
    pub system_name: String,
    /// DATABASE, API, FILE, STREAM
    pub system_type: Option<String>,
    /// Optional link to an application in the Application Register (by app_code).
    pub application_code: Option<String>,
    /// PRODUCTION, UAT, DEVELOPMENT, DR
    pub environment: Option<String>,
    pub description: Option<String>,
    pub schemas: Vec<IngestSchema>,
}

/// A database schema within the source system.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestSchema {
    pub schema_name: String,
    pub description: Option<String>,
    pub tables: Vec<IngestTable>,
}

/// A table or view within a schema.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestTable {
    pub table_name: String,
    /// TABLE, VIEW, MATERIALIZED_VIEW
    pub table_type: Option<String>,
    pub description: Option<String>,
    pub row_count: Option<i64>,
    pub is_pii: Option<bool>,
    pub columns: Vec<IngestColumn>,
    pub relationships: Option<Vec<IngestRelationship>>,
}

/// A column within a table.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestColumn {
    pub column_name: String,
    pub ordinal_position: Option<i32>,
    pub data_type: String,
    pub max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
    pub is_nullable: Option<bool>,
    pub is_primary_key: Option<bool>,
    pub is_foreign_key: Option<bool>,
    pub default_expression: Option<String>,
}

/// A foreign-key or constraint relationship between columns.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestRelationship {
    pub source_column: String,
    pub target_table: String,
    pub target_column: String,
    /// FOREIGN_KEY, PRIMARY_KEY, UNIQUE, INDEX, CHECK
    pub relationship_type: String,
    pub constraint_name: Option<String>,
}

/// Options controlling ingestion behaviour.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestOptions {
    /// Count tables/columns whose `last_seen_at` is older than the current
    /// ingestion run. Defaults to true.
    pub mark_stale: Option<bool>,
    /// Validate and report counts without writing to the database. Defaults to false.
    pub dry_run: Option<bool>,
}

// ===========================================================================
// Response types
// ===========================================================================

/// Response returned after a technical metadata ingestion operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestTechnicalResponse {
    /// "completed" or "dry_run"
    pub status: String,
    pub summary: IngestSummary,
    pub errors: Vec<IngestError>,
    pub warnings: Vec<IngestWarning>,
    pub duration_ms: i64,
}

/// Counts of created, updated, and unchanged records at each level.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestSummary {
    pub systems: IngestCounts,
    pub schemas: IngestCounts,
    pub tables: IngestCounts,
    pub columns: IngestCounts,
    pub relationships: IngestCounts,
    pub stale_flagged: IngestStaleCounts,
}

/// Created/updated/unchanged counters.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct IngestCounts {
    pub created: i64,
    pub updated: i64,
    pub unchanged: i64,
}

/// Counts of stale (not-seen-in-this-run) records.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct IngestStaleCounts {
    pub tables: i64,
    pub columns: i64,
}

/// An error that occurred while processing part of the payload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestError {
    /// Path within the payload, e.g. "schemas[0].tables[2].columns[5]"
    pub path: String,
    pub message: String,
}

/// A non-fatal warning, typically a naming standard violation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestWarning {
    /// Path within the payload, e.g. "schemas[0].tables[2].columns[5]"
    pub path: String,
    pub message: String,
}

// ===========================================================================
// Internal row types for upsert RETURNING
// ===========================================================================

/// Row returned from an upsert, telling us whether the row was inserted or updated.
#[derive(sqlx::FromRow)]
struct UpsertResult {
    id: Uuid,
    was_inserted: bool,
}

/// Row type for loading naming standards from the database.
#[derive(sqlx::FromRow)]
struct NamingStandardRow {
    standard_name: String,
    applies_to: String,
    pattern_regex: String,
    description: String,
    example_valid: Option<String>,
    example_invalid: Option<String>,
    is_mandatory: bool,
}

// ===========================================================================
// Handler
// ===========================================================================

/// Ingest hierarchical technical metadata from an external schema crawler.
///
/// Upserts source systems, schemas, tables, columns, and relationships using
/// natural keys. Validates column names against naming standards and reports
/// stale records that were not included in the payload.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/ingest/technical",
    request_body = IngestTechnicalRequest,
    responses(
        (status = 200, description = "Ingestion completed", body = IngestTechnicalResponse),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn ingest_technical(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<IngestTechnicalRequest>,
) -> AppResult<Json<IngestTechnicalResponse>> {
    let started = Instant::now();
    let started_at: DateTime<Utc> = Utc::now();

    // -----------------------------------------------------------------------
    // 1. Validate request body
    // -----------------------------------------------------------------------
    let sys = &body.source_system;
    let system_code = sys.system_code.trim().to_string();
    if system_code.is_empty() {
        return Err(AppError::Validation(
            "source_system.system_code is required".into(),
        ));
    }
    let system_name = sys.system_name.trim().to_string();
    if system_name.is_empty() {
        return Err(AppError::Validation(
            "source_system.system_name is required".into(),
        ));
    }
    if sys.schemas.is_empty() {
        return Err(AppError::Validation(
            "source_system.schemas must contain at least one schema".into(),
        ));
    }

    let options = body.options.as_ref();
    let mark_stale = options.and_then(|o| o.mark_stale).unwrap_or(true);
    let dry_run = options.and_then(|o| o.dry_run).unwrap_or(false);

    let mut errors: Vec<IngestError> = Vec::new();
    let mut warnings: Vec<IngestWarning> = Vec::new();
    let mut sys_counts = IngestCounts::default();
    let mut schema_counts = IngestCounts::default();
    let mut table_counts = IngestCounts::default();
    let mut col_counts = IngestCounts::default();
    let mut rel_counts = IngestCounts::default();
    let mut stale_counts = IngestStaleCounts::default();

    if dry_run {
        // Dry-run: validate only, return counts as all zeros
        let duration_ms = started.elapsed().as_millis() as i64;
        return Ok(Json(IngestTechnicalResponse {
            status: "dry_run".into(),
            summary: IngestSummary {
                systems: sys_counts,
                schemas: schema_counts,
                tables: table_counts,
                columns: col_counts,
                relationships: rel_counts,
                stale_flagged: stale_counts,
            },
            errors,
            warnings,
            duration_ms,
        }));
    }

    // -----------------------------------------------------------------------
    // 2. Resolve optional application_code to application_id
    // -----------------------------------------------------------------------
    let application_id: Option<Uuid> = if let Some(app_code) = sys.application_code.as_deref() {
        let app_code = app_code.trim();
        if app_code.is_empty() {
            None
        } else {
            let id = sqlx::query_scalar::<_, Uuid>(
                "SELECT application_id FROM applications WHERE application_code = $1 AND deleted_at IS NULL AND is_current_version = TRUE",
            )
            .bind(app_code)
            .fetch_optional(&state.pool)
            .await?;
            if id.is_none() {
                warnings.push(IngestWarning {
                    path: "source_system.application_code".into(),
                    message: format!(
                        "application_code '{app_code}' not found in Application Register — ignored"
                    ),
                });
            }
            id
        }
    } else {
        None
    };

    // -----------------------------------------------------------------------
    // 3. Load naming standards once for column validation
    // -----------------------------------------------------------------------
    let standards_rows = sqlx::query_as::<_, NamingStandardRow>(
        r#"
        SELECT standard_name, applies_to, pattern_regex, description,
               example_valid, example_invalid, is_mandatory
        FROM naming_standards
        WHERE deleted_at IS NULL
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let standards: Vec<naming::NamingStandard> = standards_rows
        .into_iter()
        .map(|r| naming::NamingStandard {
            name: r.standard_name,
            applies_to: r.applies_to,
            pattern: r.pattern_regex,
            description: r.description,
            example_valid: r.example_valid.unwrap_or_default(),
            example_invalid: r.example_invalid.unwrap_or_default(),
            is_mandatory: r.is_mandatory,
        })
        .collect();

    // -----------------------------------------------------------------------
    // 4. Upsert source system
    // -----------------------------------------------------------------------
    let system_type = sys.system_type.as_deref().unwrap_or("DATABASE");
    let environment = sys.environment.as_deref();
    let sys_description = sys.description.as_deref();

    let system_row = sqlx::query_as::<_, UpsertResult>(
        r#"
        INSERT INTO source_systems (system_code, system_name, system_type, description,
                                    application_id, environment, last_seen_at)
        VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)
        ON CONFLICT (system_code)
        DO UPDATE SET
            system_name   = EXCLUDED.system_name,
            system_type   = EXCLUDED.system_type,
            description   = COALESCE(EXCLUDED.description, source_systems.description),
            application_id = COALESCE(EXCLUDED.application_id, source_systems.application_id),
            environment   = COALESCE(EXCLUDED.environment, source_systems.environment),
            last_seen_at  = CURRENT_TIMESTAMP,
            updated_at    = CURRENT_TIMESTAMP
        RETURNING system_id AS id, (xmax = 0) AS was_inserted
        "#,
    )
    .bind(&system_code)
    .bind(&system_name)
    .bind(system_type)
    .bind(sys_description)
    .bind(application_id)
    .bind(environment)
    .fetch_one(&state.pool)
    .await?;

    let system_id = system_row.id;
    if system_row.was_inserted {
        sys_counts.created += 1;
    } else {
        sys_counts.updated += 1;
    }

    // -----------------------------------------------------------------------
    // 5. Process schemas
    // -----------------------------------------------------------------------
    for (si, schema) in sys.schemas.iter().enumerate() {
        let schema_name = schema.schema_name.trim().to_string();
        if schema_name.is_empty() {
            errors.push(IngestError {
                path: format!("schemas[{si}]"),
                message: "schema_name is required".into(),
            });
            continue;
        }

        let schema_row = match sqlx::query_as::<_, UpsertResult>(
            r#"
            INSERT INTO technical_schemas (system_id, schema_name, description, last_seen_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (system_id, schema_name)
            DO UPDATE SET
                description  = COALESCE(EXCLUDED.description, technical_schemas.description),
                last_seen_at = CURRENT_TIMESTAMP,
                updated_at   = CURRENT_TIMESTAMP
            RETURNING schema_id AS id, (xmax = 0) AS was_inserted
            "#,
        )
        .bind(system_id)
        .bind(&schema_name)
        .bind(schema.description.as_deref())
        .fetch_one(&state.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                errors.push(IngestError {
                    path: format!("schemas[{si}]"),
                    message: format!("failed to upsert schema '{schema_name}': {e}"),
                });
                continue;
            }
        };

        let schema_id = schema_row.id;
        if schema_row.was_inserted {
            schema_counts.created += 1;
        } else {
            schema_counts.updated += 1;
        }

        // -------------------------------------------------------------------
        // 6. Process tables
        // -------------------------------------------------------------------
        for (ti, table) in schema.tables.iter().enumerate() {
            let table_name = table.table_name.trim().to_string();
            if table_name.is_empty() {
                errors.push(IngestError {
                    path: format!("schemas[{si}].tables[{ti}]"),
                    message: "table_name is required".into(),
                });
                continue;
            }

            let table_type = table.table_type.as_deref().unwrap_or("TABLE");
            let is_pii = table.is_pii.unwrap_or(false);

            let table_row = match sqlx::query_as::<_, UpsertResult>(
                r#"
                INSERT INTO technical_tables (schema_id, table_name, table_type, description,
                                              row_count, is_pii, last_seen_at)
                VALUES ($1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP)
                ON CONFLICT (schema_id, table_name)
                DO UPDATE SET
                    table_type   = EXCLUDED.table_type,
                    description  = COALESCE(EXCLUDED.description, technical_tables.description),
                    row_count    = COALESCE(EXCLUDED.row_count, technical_tables.row_count),
                    is_pii       = EXCLUDED.is_pii,
                    last_seen_at = CURRENT_TIMESTAMP,
                    updated_at   = CURRENT_TIMESTAMP
                RETURNING table_id AS id, (xmax = 0) AS was_inserted
                "#,
            )
            .bind(schema_id)
            .bind(&table_name)
            .bind(table_type)
            .bind(table.description.as_deref())
            .bind(table.row_count)
            .bind(is_pii)
            .fetch_one(&state.pool)
            .await
            {
                Ok(row) => row,
                Err(e) => {
                    errors.push(IngestError {
                        path: format!("schemas[{si}].tables[{ti}]"),
                        message: format!("failed to upsert table '{table_name}': {e}"),
                    });
                    continue;
                }
            };

            let table_id = table_row.id;
            if table_row.was_inserted {
                table_counts.created += 1;
            } else {
                table_counts.updated += 1;
            }

            // ---------------------------------------------------------------
            // 7. Process columns
            // ---------------------------------------------------------------
            for (ci, column) in table.columns.iter().enumerate() {
                let column_name = column.column_name.trim().to_string();
                if column_name.is_empty() {
                    errors.push(IngestError {
                        path: format!("schemas[{si}].tables[{ti}].columns[{ci}]"),
                        message: "column_name is required".into(),
                    });
                    continue;
                }
                let col_data_type = column.data_type.trim().to_string();
                if col_data_type.is_empty() {
                    errors.push(IngestError {
                        path: format!("schemas[{si}].tables[{ti}].columns[{ci}]"),
                        message: "data_type is required".into(),
                    });
                    continue;
                }

                // Run naming validation
                let validation = naming::validate_name(&column_name, "COLUMN", &standards);
                let naming_compliant = validation.is_compliant;
                let naming_violation = if validation.violations.is_empty() {
                    None
                } else {
                    let msg = validation
                        .violations
                        .iter()
                        .map(|v| v.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                    warnings.push(IngestWarning {
                        path: format!("schemas[{si}].tables[{ti}].columns[{ci}]"),
                        message: format!("naming standard violation on '{column_name}': {msg}"),
                    });
                    Some(msg)
                };

                let ordinal = column.ordinal_position.unwrap_or((ci + 1) as i32);
                let is_nullable = column.is_nullable.unwrap_or(true);
                let is_pk = column.is_primary_key.unwrap_or(false);
                let is_fk = column.is_foreign_key.unwrap_or(false);

                let col_result = match sqlx::query_as::<_, UpsertResult>(
                    r#"
                    INSERT INTO technical_columns (
                        table_id, column_name, ordinal_position, data_type,
                        max_length, numeric_precision, numeric_scale,
                        is_nullable, is_primary_key, is_foreign_key,
                        default_expression,
                        naming_standard_compliant, naming_standard_violation,
                        last_seen_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, CURRENT_TIMESTAMP)
                    ON CONFLICT (table_id, column_name)
                    DO UPDATE SET
                        ordinal_position          = EXCLUDED.ordinal_position,
                        data_type                 = EXCLUDED.data_type,
                        max_length                = EXCLUDED.max_length,
                        numeric_precision         = EXCLUDED.numeric_precision,
                        numeric_scale             = EXCLUDED.numeric_scale,
                        is_nullable               = EXCLUDED.is_nullable,
                        is_primary_key            = EXCLUDED.is_primary_key,
                        is_foreign_key            = EXCLUDED.is_foreign_key,
                        default_expression        = EXCLUDED.default_expression,
                        naming_standard_compliant = EXCLUDED.naming_standard_compliant,
                        naming_standard_violation = EXCLUDED.naming_standard_violation,
                        last_seen_at              = CURRENT_TIMESTAMP,
                        updated_at                = CURRENT_TIMESTAMP
                    RETURNING column_id AS id, (xmax = 0) AS was_inserted
                    "#,
                )
                .bind(table_id)
                .bind(&column_name)
                .bind(ordinal)
                .bind(&col_data_type)
                .bind(column.max_length)
                .bind(column.numeric_precision)
                .bind(column.numeric_scale)
                .bind(is_nullable)
                .bind(is_pk)
                .bind(is_fk)
                .bind(column.default_expression.as_deref())
                .bind(naming_compliant)
                .bind(naming_violation.as_deref())
                .fetch_one(&state.pool)
                .await
                {
                    Ok(row) => row,
                    Err(e) => {
                        errors.push(IngestError {
                            path: format!("schemas[{si}].tables[{ti}].columns[{ci}]"),
                            message: format!("failed to upsert column '{column_name}': {e}"),
                        });
                        continue;
                    }
                };

                if col_result.was_inserted {
                    col_counts.created += 1;
                } else {
                    col_counts.updated += 1;
                }
            }

            // ---------------------------------------------------------------
            // 8. Process relationships
            // ---------------------------------------------------------------
            if let Some(relationships) = &table.relationships {
                for (ri, rel) in relationships.iter().enumerate() {
                    let source_col = rel.source_column.trim();
                    let target_tbl = rel.target_table.trim();
                    let target_col = rel.target_column.trim();

                    if source_col.is_empty() || target_tbl.is_empty() || target_col.is_empty() {
                        errors.push(IngestError {
                            path: format!("schemas[{si}].tables[{ti}].relationships[{ri}]"),
                            message: "source_column, target_table, and target_column are required"
                                .into(),
                        });
                        continue;
                    }

                    // Resolve source column ID within the current table
                    let source_column_id = match sqlx::query_scalar::<_, Uuid>(
                        r#"
                        SELECT column_id FROM technical_columns
                        WHERE table_id = $1 AND column_name = $2 AND deleted_at IS NULL
                        "#,
                    )
                    .bind(table_id)
                    .bind(source_col)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        Some(id) => id,
                        None => {
                            errors.push(IngestError {
                                path: format!("schemas[{si}].tables[{ti}].relationships[{ri}]"),
                                message: format!(
                                    "source column '{source_col}' not found in table '{table_name}'"
                                ),
                            });
                            continue;
                        }
                    };

                    // Resolve target column ID within the same source system
                    let target_column_id = match sqlx::query_scalar::<_, Uuid>(
                        r#"
                        SELECT tc.column_id
                        FROM technical_columns tc
                        JOIN technical_tables tt ON tt.table_id = tc.table_id AND tt.deleted_at IS NULL
                        JOIN technical_schemas ts ON ts.schema_id = tt.schema_id AND ts.deleted_at IS NULL
                        WHERE ts.system_id = $1
                          AND tt.table_name = $2
                          AND tc.column_name = $3
                          AND tc.deleted_at IS NULL
                        LIMIT 1
                        "#,
                    )
                    .bind(system_id)
                    .bind(target_tbl)
                    .bind(target_col)
                    .fetch_optional(&state.pool)
                    .await?
                    {
                        Some(id) => id,
                        None => {
                            warnings.push(IngestWarning {
                                path: format!("schemas[{si}].tables[{ti}].relationships[{ri}]"),
                                message: format!(
                                    "target column '{target_tbl}.{target_col}' not found in system '{system_code}' — relationship skipped"
                                ),
                            });
                            continue;
                        }
                    };

                    let rel_type = rel.relationship_type.trim().to_uppercase();
                    let constraint_name = rel.constraint_name.as_deref();

                    let rel_result = match sqlx::query_as::<_, UpsertResult>(
                        r#"
                        INSERT INTO column_relationships (
                            source_column_id, target_column_id, relationship_type, constraint_name
                        )
                        VALUES ($1, $2, $3, $4)
                        ON CONFLICT (source_column_id, target_column_id, relationship_type)
                        DO UPDATE SET
                            constraint_name = COALESCE(EXCLUDED.constraint_name, column_relationships.constraint_name)
                        RETURNING relationship_id AS id, (xmax = 0) AS was_inserted
                        "#,
                    )
                    .bind(source_column_id)
                    .bind(target_column_id)
                    .bind(&rel_type)
                    .bind(constraint_name)
                    .fetch_one(&state.pool)
                    .await
                    {
                        Ok(row) => row,
                        Err(e) => {
                            errors.push(IngestError {
                                path: format!("schemas[{si}].tables[{ti}].relationships[{ri}]"),
                                message: format!("failed to upsert relationship: {e}"),
                            });
                            continue;
                        }
                    };

                    if rel_result.was_inserted {
                        rel_counts.created += 1;
                    } else {
                        rel_counts.updated += 1;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 9. Stale detection
    // -----------------------------------------------------------------------
    if mark_stale {
        // Count tables in this system whose last_seen_at is before the start
        // of this ingestion run (or is NULL — never ingested).
        let stale_tables = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM technical_tables tt
            JOIN technical_schemas ts ON ts.schema_id = tt.schema_id AND ts.deleted_at IS NULL
            WHERE ts.system_id = $1
              AND tt.deleted_at IS NULL
              AND (tt.last_seen_at IS NULL OR tt.last_seen_at < $2)
            "#,
        )
        .bind(system_id)
        .bind(started_at)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

        let stale_columns = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM technical_columns tc
            JOIN technical_tables tt ON tt.table_id = tc.table_id AND tt.deleted_at IS NULL
            JOIN technical_schemas ts ON ts.schema_id = tt.schema_id AND ts.deleted_at IS NULL
            WHERE ts.system_id = $1
              AND tc.deleted_at IS NULL
              AND (tc.last_seen_at IS NULL OR tc.last_seen_at < $2)
            "#,
        )
        .bind(system_id)
        .bind(started_at)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

        stale_counts.tables = stale_tables;
        stale_counts.columns = stale_columns;
    }

    // -----------------------------------------------------------------------
    // 10. Log to ingestion_log
    // -----------------------------------------------------------------------
    let duration_ms = started.elapsed().as_millis() as i64;

    let summary = IngestSummary {
        systems: sys_counts,
        schemas: schema_counts,
        tables: table_counts,
        columns: col_counts,
        relationships: rel_counts,
        stale_flagged: stale_counts.clone(),
    };

    let summary_json = serde_json::to_value(&summary).unwrap_or_default();
    let errors_json = serde_json::to_value(&errors).unwrap_or_default();
    let warnings_json = serde_json::to_value(&warnings).unwrap_or_default();

    // Log the ingestion — api_key_id is NULL when invoked via JWT auth (future:
    // populate from API key middleware). Use the authenticated user via claims.sub.
    let _log_result = sqlx::query(
        r#"
        INSERT INTO ingestion_log (ingestion_type, source_system_code, summary, errors, warnings, duration_ms)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind("technical")
    .bind(&system_code)
    .bind(&summary_json)
    .bind(&errors_json)
    .bind(&warnings_json)
    .bind(duration_ms as i32)
    .execute(&state.pool)
    .await;

    // Ignore log insertion failures — they should not block the response.
    if let Err(e) = &_log_result {
        tracing::warn!(error = %e, "failed to insert ingestion_log record");
    }

    // -----------------------------------------------------------------------
    // 11. Return response
    // -----------------------------------------------------------------------
    Ok(Json(IngestTechnicalResponse {
        status: "completed".into(),
        summary,
        errors,
        warnings,
        duration_ms,
    }))
}

// ===========================================================================
// FEATURE 1: Element Ingestion
// ===========================================================================

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Top-level request body for the data element ingestion endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestElementsRequest {
    pub elements: Vec<IngestElement>,
    pub options: Option<IngestElementOptions>,
}

/// A data element to ingest.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestElement {
    pub element_code: String,
    pub element_name: String,
    pub description: String,
    pub data_type: String,
    pub business_definition: Option<String>,
    pub business_rules: Option<String>,
    pub format_pattern: Option<String>,
    pub is_nullable: Option<bool>,
    pub is_cde: Option<bool>,
    pub cde_rationale: Option<String>,
    pub is_pii: Option<bool>,
    /// Resolve to glossary_term_id via glossary_terms.term_code
    pub glossary_term_code: Option<String>,
    /// Resolve to domain_id via glossary_domains.domain_code
    pub domain_code: Option<String>,
    /// Resolve to classification_id via data_classifications.classification_code
    pub classification_code: Option<String>,
    /// Resolve to owner_user_id via users.email
    pub owner_email: Option<String>,
    /// Resolve to steward_user_id via users.email
    pub steward_email: Option<String>,
}

/// Options controlling element ingestion behaviour.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestElementOptions {
    /// If true, set status to ACCEPTED (no workflow). Defaults to true for API key sources.
    pub auto_accept: Option<bool>,
    /// Validate and report counts without writing to the database. Defaults to false.
    pub dry_run: Option<bool>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response returned after a data element ingestion operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestElementsResponse {
    /// "completed" or "dry_run"
    pub status: String,
    pub summary: IngestElementSummary,
    pub errors: Vec<IngestError>,
    pub warnings: Vec<IngestWarning>,
    pub duration_ms: i64,
}

/// Counts of created, updated, and unchanged data elements.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct IngestElementSummary {
    pub created: i64,
    pub updated: i64,
    pub unchanged: i64,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Ingest data elements from an external source.
///
/// Upserts data elements by `element_code`. Resolves glossary term codes,
/// domain codes, classification codes, and user emails to their respective IDs.
/// Supports auto-accept mode (skips workflow) and dry-run mode.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/ingest/elements",
    request_body = IngestElementsRequest,
    responses(
        (status = 200, description = "Ingestion completed", body = IngestElementsResponse),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn ingest_elements(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<IngestElementsRequest>,
) -> AppResult<Json<IngestElementsResponse>> {
    let started = Instant::now();

    // -----------------------------------------------------------------------
    // 1. Validate request body
    // -----------------------------------------------------------------------
    if body.elements.is_empty() {
        return Err(AppError::Validation(
            "elements must contain at least one element".into(),
        ));
    }

    let options = body.options.as_ref();
    let auto_accept = options.and_then(|o| o.auto_accept).unwrap_or(true);
    let dry_run = options.and_then(|o| o.dry_run).unwrap_or(false);

    let mut errors: Vec<IngestError> = Vec::new();
    let mut warnings: Vec<IngestWarning> = Vec::new();
    let mut summary = IngestElementSummary::default();

    if dry_run {
        let duration_ms = started.elapsed().as_millis() as i64;
        return Ok(Json(IngestElementsResponse {
            status: "dry_run".into(),
            summary,
            errors,
            warnings,
            duration_ms,
        }));
    }

    // -----------------------------------------------------------------------
    // 2. Resolve the ACCEPTED status_id (if auto_accept) or DRAFT
    // -----------------------------------------------------------------------
    let target_status_code = if auto_accept { "ACCEPTED" } else { "DRAFT" };
    let status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = $1",
    )
    .bind(target_status_code)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| {
        AppError::Internal(anyhow::anyhow!(
            "status '{target_status_code}' not found in entity_statuses"
        ))
    })?;

    // -----------------------------------------------------------------------
    // 3. Build lookup caches for codes/emails referenced in the payload
    // -----------------------------------------------------------------------
    // Collect unique codes/emails for batch resolution
    let mut term_codes: Vec<String> = Vec::new();
    let mut domain_codes: Vec<String> = Vec::new();
    let mut classification_codes: Vec<String> = Vec::new();
    let mut emails: Vec<String> = Vec::new();

    for el in &body.elements {
        if let Some(code) = &el.glossary_term_code {
            let code = code.trim().to_string();
            if !code.is_empty() && !term_codes.contains(&code) {
                term_codes.push(code);
            }
        }
        if let Some(code) = &el.domain_code {
            let code = code.trim().to_string();
            if !code.is_empty() && !domain_codes.contains(&code) {
                domain_codes.push(code);
            }
        }
        if let Some(code) = &el.classification_code {
            let code = code.trim().to_string();
            if !code.is_empty() && !classification_codes.contains(&code) {
                classification_codes.push(code);
            }
        }
        for email in [&el.owner_email, &el.steward_email].into_iter().flatten() {
            let email = email.trim().to_lowercase();
            if !email.is_empty() && !emails.contains(&email) {
                emails.push(email);
            }
        }
    }

    // Batch resolve: glossary_term_code -> term_id
    let term_map: HashMap<String, Uuid> = if !term_codes.is_empty() {
        sqlx::query_as::<_, (String, Uuid)>(
            r#"SELECT term_code, term_id FROM glossary_terms
               WHERE term_code = ANY($1) AND is_current_version = TRUE AND deleted_at IS NULL"#,
        )
        .bind(&term_codes)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .collect()
    } else {
        HashMap::new()
    };

    // Batch resolve: domain_code -> domain_id
    let domain_map: HashMap<String, Uuid> = if !domain_codes.is_empty() {
        sqlx::query_as::<_, (String, Uuid)>(
            "SELECT domain_code, domain_id FROM glossary_domains WHERE domain_code = ANY($1)",
        )
        .bind(&domain_codes)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .collect()
    } else {
        HashMap::new()
    };

    // Batch resolve: classification_code -> classification_id
    let classification_map: HashMap<String, Uuid> = if !classification_codes.is_empty() {
        sqlx::query_as::<_, (String, Uuid)>(
            "SELECT classification_code, classification_id FROM data_classifications WHERE classification_code = ANY($1)",
        )
        .bind(&classification_codes)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .collect()
    } else {
        HashMap::new()
    };

    // Batch resolve: email -> user_id
    let user_map: HashMap<String, Uuid> = if !emails.is_empty() {
        sqlx::query_as::<_, (String, Uuid)>(
            "SELECT LOWER(email), user_id FROM users WHERE LOWER(email) = ANY($1) AND deleted_at IS NULL",
        )
        .bind(&emails)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .collect()
    } else {
        HashMap::new()
    };

    // -----------------------------------------------------------------------
    // 4. Process elements
    // -----------------------------------------------------------------------
    let created_by = claims.sub;

    for (idx, el) in body.elements.iter().enumerate() {
        let element_code = el.element_code.trim().to_string();
        if element_code.is_empty() {
            errors.push(IngestError {
                path: format!("elements[{idx}]"),
                message: "element_code is required".into(),
            });
            continue;
        }
        let element_name = el.element_name.trim().to_string();
        if element_name.is_empty() {
            errors.push(IngestError {
                path: format!("elements[{idx}]"),
                message: "element_name is required".into(),
            });
            continue;
        }
        let description = el.description.trim().to_string();
        if description.is_empty() {
            errors.push(IngestError {
                path: format!("elements[{idx}]"),
                message: "description is required".into(),
            });
            continue;
        }
        let data_type = el.data_type.trim().to_string();
        if data_type.is_empty() {
            errors.push(IngestError {
                path: format!("elements[{idx}]"),
                message: "data_type is required".into(),
            });
            continue;
        }

        // Resolve optional foreign keys
        let glossary_term_id: Option<Uuid> = el.glossary_term_code.as_deref().and_then(|code| {
            let code = code.trim();
            if code.is_empty() {
                return None;
            }
            match term_map.get(code) {
                Some(id) => Some(*id),
                None => {
                    warnings.push(IngestWarning {
                        path: format!("elements[{idx}].glossary_term_code"),
                        message: format!("glossary_term_code '{code}' not found — ignored"),
                    });
                    None
                }
            }
        });

        let domain_id: Option<Uuid> = el.domain_code.as_deref().and_then(|code| {
            let code = code.trim();
            if code.is_empty() {
                return None;
            }
            match domain_map.get(code) {
                Some(id) => Some(*id),
                None => {
                    warnings.push(IngestWarning {
                        path: format!("elements[{idx}].domain_code"),
                        message: format!("domain_code '{code}' not found — ignored"),
                    });
                    None
                }
            }
        });

        let classification_id: Option<Uuid> = el.classification_code.as_deref().and_then(|code| {
            let code = code.trim();
            if code.is_empty() {
                return None;
            }
            match classification_map.get(code) {
                Some(id) => Some(*id),
                None => {
                    warnings.push(IngestWarning {
                        path: format!("elements[{idx}].classification_code"),
                        message: format!("classification_code '{code}' not found — ignored"),
                    });
                    None
                }
            }
        });

        let owner_user_id: Option<Uuid> = el.owner_email.as_deref().and_then(|email| {
            let email = email.trim().to_lowercase();
            if email.is_empty() {
                return None;
            }
            match user_map.get(&email) {
                Some(id) => Some(*id),
                None => {
                    warnings.push(IngestWarning {
                        path: format!("elements[{idx}].owner_email"),
                        message: format!("user '{email}' not found — ignored"),
                    });
                    None
                }
            }
        });

        let steward_user_id: Option<Uuid> = el.steward_email.as_deref().and_then(|email| {
            let email = email.trim().to_lowercase();
            if email.is_empty() {
                return None;
            }
            match user_map.get(&email) {
                Some(id) => Some(*id),
                None => {
                    warnings.push(IngestWarning {
                        path: format!("elements[{idx}].steward_email"),
                        message: format!("user '{email}' not found — ignored"),
                    });
                    None
                }
            }
        });

        let is_nullable = el.is_nullable.unwrap_or(true);
        let is_cde = el.is_cde.unwrap_or(false);
        let is_pii = el.is_pii.unwrap_or(false);

        // Upsert by element_code (current version, not deleted)
        let result = match sqlx::query_as::<_, UpsertResult>(
            r#"
            INSERT INTO data_elements (
                element_code, element_name, description, data_type,
                business_definition, business_rules, format_pattern,
                is_nullable, is_cde, cde_rationale, is_pii,
                glossary_term_id, domain_id, classification_id,
                owner_user_id, steward_user_id,
                status_id, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            ON CONFLICT (element_code, version_number)
            DO UPDATE SET
                element_name       = EXCLUDED.element_name,
                description        = EXCLUDED.description,
                data_type          = EXCLUDED.data_type,
                business_definition = COALESCE(EXCLUDED.business_definition, data_elements.business_definition),
                business_rules     = COALESCE(EXCLUDED.business_rules, data_elements.business_rules),
                format_pattern     = COALESCE(EXCLUDED.format_pattern, data_elements.format_pattern),
                is_nullable        = EXCLUDED.is_nullable,
                is_cde             = EXCLUDED.is_cde,
                cde_rationale      = COALESCE(EXCLUDED.cde_rationale, data_elements.cde_rationale),
                is_pii             = EXCLUDED.is_pii,
                glossary_term_id   = COALESCE(EXCLUDED.glossary_term_id, data_elements.glossary_term_id),
                domain_id          = COALESCE(EXCLUDED.domain_id, data_elements.domain_id),
                classification_id  = COALESCE(EXCLUDED.classification_id, data_elements.classification_id),
                owner_user_id      = COALESCE(EXCLUDED.owner_user_id, data_elements.owner_user_id),
                steward_user_id    = COALESCE(EXCLUDED.steward_user_id, data_elements.steward_user_id),
                status_id          = EXCLUDED.status_id,
                updated_by         = $18,
                updated_at         = CURRENT_TIMESTAMP
            RETURNING element_id AS id, (xmax = 0) AS was_inserted
            "#,
        )
        .bind(&element_code)
        .bind(&element_name)
        .bind(&description)
        .bind(&data_type)
        .bind(el.business_definition.as_deref())
        .bind(el.business_rules.as_deref())
        .bind(el.format_pattern.as_deref())
        .bind(is_nullable)
        .bind(is_cde)
        .bind(el.cde_rationale.as_deref())
        .bind(is_pii)
        .bind(glossary_term_id)
        .bind(domain_id)
        .bind(classification_id)
        .bind(owner_user_id)
        .bind(steward_user_id)
        .bind(status_id)
        .bind(created_by)
        .fetch_one(&state.pool)
        .await
        {
            Ok(row) => row,
            Err(e) => {
                errors.push(IngestError {
                    path: format!("elements[{idx}]"),
                    message: format!("failed to upsert element '{element_code}': {e}"),
                });
                continue;
            }
        };

        if result.was_inserted {
            summary.created += 1;
        } else {
            summary.updated += 1;
        }
    }

    // -----------------------------------------------------------------------
    // 5. Log to ingestion_log
    // -----------------------------------------------------------------------
    let duration_ms = started.elapsed().as_millis() as i64;

    let summary_json = serde_json::to_value(&summary).unwrap_or_default();
    let errors_json = serde_json::to_value(&errors).unwrap_or_default();
    let warnings_json = serde_json::to_value(&warnings).unwrap_or_default();

    let _log_result = sqlx::query(
        r#"
        INSERT INTO ingestion_log (ingestion_type, summary, errors, warnings, duration_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind("elements")
    .bind(&summary_json)
    .bind(&errors_json)
    .bind(&warnings_json)
    .bind(duration_ms as i32)
    .execute(&state.pool)
    .await;

    if let Err(e) = &_log_result {
        tracing::warn!(error = %e, "failed to insert ingestion_log record");
    }

    // -----------------------------------------------------------------------
    // 6. Return response
    // -----------------------------------------------------------------------
    Ok(Json(IngestElementsResponse {
        status: "completed".into(),
        summary,
        errors,
        warnings,
        duration_ms,
    }))
}

// ===========================================================================
// FEATURE 2: Column-to-Element Linking
// ===========================================================================

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for linking technical columns to data elements.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LinkColumnsRequest {
    pub links: Vec<ColumnElementLink>,
}

/// A single column-to-element link specification.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ColumnElementLink {
    pub system_code: String,
    pub schema_name: String,
    pub table_name: String,
    pub column_name: String,
    pub element_code: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response returned after a column-element linking operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LinkColumnsResponse {
    /// "completed"
    pub status: String,
    pub summary: LinkColumnsSummary,
    pub errors: Vec<IngestError>,
    pub warnings: Vec<IngestWarning>,
    pub duration_ms: i64,
}

/// Counts for the column linking operation.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct LinkColumnsSummary {
    pub linked: i64,
    pub not_found_column: i64,
    pub not_found_element: i64,
    pub failed: i64,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Link technical columns to data elements by resolving natural keys.
///
/// For each link in the payload, resolves the column via system_code, schema_name,
/// table_name, column_name and the element via element_code, then sets the
/// element_id on the technical column.
#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/ingest/link-columns",
    request_body = LinkColumnsRequest,
    responses(
        (status = 200, description = "Linking completed", body = LinkColumnsResponse),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn link_columns(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<LinkColumnsRequest>,
) -> AppResult<Json<LinkColumnsResponse>> {
    let started = Instant::now();

    if body.links.is_empty() {
        return Err(AppError::Validation(
            "links must contain at least one link".into(),
        ));
    }

    let mut errors: Vec<IngestError> = Vec::new();
    let mut warnings: Vec<IngestWarning> = Vec::new();
    let mut summary = LinkColumnsSummary::default();

    for (idx, link) in body.links.iter().enumerate() {
        let system_code = link.system_code.trim();
        let schema_name = link.schema_name.trim();
        let table_name = link.table_name.trim();
        let column_name = link.column_name.trim();
        let element_code = link.element_code.trim();

        if system_code.is_empty()
            || schema_name.is_empty()
            || table_name.is_empty()
            || column_name.is_empty()
            || element_code.is_empty()
        {
            errors.push(IngestError {
                path: format!("links[{idx}]"),
                message: "all fields (system_code, schema_name, table_name, column_name, element_code) are required".into(),
            });
            summary.failed += 1;
            continue;
        }

        // Resolve column via JOIN chain
        let column_id = match sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT tc.column_id
            FROM technical_columns tc
            JOIN technical_tables tt ON tt.table_id = tc.table_id AND tt.deleted_at IS NULL
            JOIN technical_schemas ts ON ts.schema_id = tt.schema_id AND ts.deleted_at IS NULL
            JOIN source_systems ss ON ss.system_id = ts.system_id AND ss.deleted_at IS NULL
            WHERE ss.system_code = $1
              AND ts.schema_name = $2
              AND tt.table_name = $3
              AND tc.column_name = $4
              AND tc.deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(system_code)
        .bind(schema_name)
        .bind(table_name)
        .bind(column_name)
        .fetch_optional(&state.pool)
        .await?
        {
            Some(id) => id,
            None => {
                warnings.push(IngestWarning {
                    path: format!("links[{idx}]"),
                    message: format!(
                        "column '{system_code}.{schema_name}.{table_name}.{column_name}' not found — skipped"
                    ),
                });
                summary.not_found_column += 1;
                continue;
            }
        };

        // Resolve element_code -> element_id
        let element_id = match sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT element_id FROM data_elements
            WHERE element_code = $1 AND is_current_version = TRUE AND deleted_at IS NULL
            "#,
        )
        .bind(element_code)
        .fetch_optional(&state.pool)
        .await?
        {
            Some(id) => id,
            None => {
                warnings.push(IngestWarning {
                    path: format!("links[{idx}]"),
                    message: format!("element_code '{element_code}' not found — skipped"),
                });
                summary.not_found_element += 1;
                continue;
            }
        };

        // Update the column with the element_id
        match sqlx::query(
            "UPDATE technical_columns SET element_id = $1, updated_at = CURRENT_TIMESTAMP WHERE column_id = $2",
        )
        .bind(element_id)
        .bind(column_id)
        .execute(&state.pool)
        .await
        {
            Ok(_) => {
                summary.linked += 1;
            }
            Err(e) => {
                errors.push(IngestError {
                    path: format!("links[{idx}]"),
                    message: format!("failed to link column: {e}"),
                });
                summary.failed += 1;
            }
        }
    }

    let duration_ms = started.elapsed().as_millis() as i64;

    Ok(Json(LinkColumnsResponse {
        status: "completed".into(),
        summary,
        errors,
        warnings,
        duration_ms,
    }))
}

// ===========================================================================
// FEATURE 3: Quality Score Ingestion
// ===========================================================================

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Top-level request body for the quality score ingestion endpoint.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct IngestScoresRequest {
    pub profiling_run: ProfilingRun,
}

/// A profiling run containing quality scores from an external tool.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ProfilingRun {
    /// Unique identifier for this profiling run (from the external tool).
    pub run_id: String,
    /// Source system code (for reference/filtering).
    pub source_system_code: Option<String>,
    /// ISO 8601 timestamp when the profiling run was executed.
    pub run_timestamp: Option<String>,
    /// Name of the profiling tool (e.g. "Great Expectations", "dbt", "Ataccama").
    pub tool_name: Option<String>,
    /// Quality score entries from this run.
    pub scores: Vec<ScoreEntry>,
}

/// A single quality score entry from a profiling run.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScoreEntry {
    /// Resolve to rule_id via quality_rules.rule_code.
    pub rule_code: Option<String>,
    /// Resolve to element_id via data_elements.element_code (alternative to rule_code).
    pub element_code: Option<String>,
    /// Resolve to dimension_id via quality_dimensions.dimension_code (used with element_code).
    pub dimension_code: Option<String>,
    pub records_evaluated: Option<i64>,
    pub records_passed: Option<i64>,
    pub records_failed: Option<i64>,
    /// Pass rate as a decimal (0.0 to 100.0). Calculated from records if not provided.
    pub pass_rate: Option<f64>,
    /// PASS, FAIL, WARNING. Auto-determined from pass_rate vs threshold if not provided.
    pub status: Option<String>,
    /// Free-text details or error messages from the profiling tool.
    pub details: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response returned after a quality score ingestion operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestScoresResponse {
    /// "completed"
    pub status: String,
    pub scores_created: i64,
    pub scores_failed: i64,
    pub errors: Vec<IngestError>,
    pub duration_ms: i64,
}

// ---------------------------------------------------------------------------
// Internal row types
// ---------------------------------------------------------------------------

/// Row returned when resolving a quality rule by rule_code.
#[derive(sqlx::FromRow)]
struct ResolvedRule {
    rule_id: Uuid,
    element_id: Option<Uuid>,
    dimension_id: Uuid,
    threshold_percentage: Option<f64>,
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Ingest quality scores from an external profiling tool.
///
/// Accepts a profiling run payload containing score entries. Each entry is
/// resolved by `rule_code` (to a quality rule) or by `element_code` +
/// `dimension_code` (to an element/dimension pair). Pass rates are calculated
/// from record counts if not provided, and status is auto-determined from
/// the rule's threshold when available.
#[utoipa::path(
    post,
    path = "/api/v1/data-quality/ingest/scores",
    request_body = IngestScoresRequest,
    responses(
        (status = 200, description = "Ingestion completed", body = IngestScoresResponse),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "data_quality"
)]
pub async fn ingest_scores(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<IngestScoresRequest>,
) -> AppResult<Json<IngestScoresResponse>> {
    let started = Instant::now();

    // -----------------------------------------------------------------------
    // 1. Validate request body
    // -----------------------------------------------------------------------
    let run = &body.profiling_run;
    let run_id = run.run_id.trim().to_string();
    if run_id.is_empty() {
        return Err(AppError::Validation(
            "profiling_run.run_id is required".into(),
        ));
    }
    if run.scores.is_empty() {
        return Err(AppError::Validation(
            "profiling_run.scores must contain at least one entry".into(),
        ));
    }

    let source_system_code = run.source_system_code.as_deref().map(|s| s.trim());
    let tool_name = run.tool_name.as_deref().map(|s| s.trim());

    // Parse run_timestamp or default to now
    let profiled_at: DateTime<Utc> = if let Some(ts) = &run.run_timestamp {
        ts.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
    } else {
        Utc::now()
    };

    let mut errors: Vec<IngestError> = Vec::new();
    let mut scores_created: i64 = 0;
    let mut scores_failed: i64 = 0;

    // -----------------------------------------------------------------------
    // 2. Process each score entry
    // -----------------------------------------------------------------------
    for (idx, entry) in run.scores.iter().enumerate() {
        let path = format!("scores[{idx}]");

        // --- Resolve rule_code to rule details ---
        let resolved_rule: Option<ResolvedRule> = if let Some(code) = &entry.rule_code {
            let code = code.trim();
            if code.is_empty() {
                None
            } else {
                match sqlx::query_as::<_, ResolvedRule>(
                    r#"
                    SELECT rule_id, element_id, dimension_id,
                           threshold_percentage::FLOAT8 AS threshold_percentage
                    FROM quality_rules
                    WHERE rule_code = $1 AND deleted_at IS NULL AND is_current_version = TRUE
                    "#,
                )
                .bind(code)
                .fetch_optional(&state.pool)
                .await
                {
                    Ok(Some(row)) => Some(row),
                    Ok(None) => {
                        errors.push(IngestError {
                            path: path.clone(),
                            message: format!("rule_code '{code}' not found"),
                        });
                        scores_failed += 1;
                        continue;
                    }
                    Err(e) => {
                        errors.push(IngestError {
                            path: path.clone(),
                            message: format!("failed to resolve rule_code '{code}': {e}"),
                        });
                        scores_failed += 1;
                        continue;
                    }
                }
            }
        } else {
            None
        };

        // Determine rule_id, element_id, dimension_id
        let rule_id: Option<Uuid> = resolved_rule.as_ref().map(|r| r.rule_id);
        let threshold: Option<f64> = resolved_rule.as_ref().and_then(|r| r.threshold_percentage);

        // Element ID: from rule or from element_code
        let element_id: Option<Uuid> = if let Some(ref rule) = resolved_rule {
            rule.element_id
        } else if let Some(code) = &entry.element_code {
            let code = code.trim();
            if code.is_empty() {
                None
            } else {
                match sqlx::query_scalar::<_, Uuid>(
                    "SELECT element_id FROM data_elements WHERE element_code = $1 AND deleted_at IS NULL AND is_current_version = TRUE",
                )
                .bind(code)
                .fetch_optional(&state.pool)
                .await
                {
                    Ok(id) => {
                        if id.is_none() {
                            errors.push(IngestError {
                                path: path.clone(),
                                message: format!("element_code '{code}' not found"),
                            });
                            scores_failed += 1;
                            continue;
                        }
                        id
                    }
                    Err(e) => {
                        errors.push(IngestError {
                            path: path.clone(),
                            message: format!("failed to resolve element_code '{code}': {e}"),
                        });
                        scores_failed += 1;
                        continue;
                    }
                }
            }
        } else {
            None
        };

        // Dimension ID: from rule or from dimension_code
        let dimension_id: Option<Uuid> = if let Some(ref rule) = resolved_rule {
            Some(rule.dimension_id)
        } else if let Some(code) = &entry.dimension_code {
            let code = code.trim();
            if code.is_empty() {
                None
            } else {
                match sqlx::query_scalar::<_, Uuid>(
                    "SELECT dimension_id FROM quality_dimensions WHERE dimension_code = $1",
                )
                .bind(code)
                .fetch_optional(&state.pool)
                .await
                {
                    Ok(id) => {
                        if id.is_none() {
                            errors.push(IngestError {
                                path: path.clone(),
                                message: format!("dimension_code '{code}' not found"),
                            });
                            scores_failed += 1;
                            continue;
                        }
                        id
                    }
                    Err(e) => {
                        errors.push(IngestError {
                            path: path.clone(),
                            message: format!("failed to resolve dimension_code '{code}': {e}"),
                        });
                        scores_failed += 1;
                        continue;
                    }
                }
            }
        } else {
            None
        };

        // Must have at least one of rule_id or element_id
        if rule_id.is_none() && element_id.is_none() {
            errors.push(IngestError {
                path: path.clone(),
                message: "score entry must resolve to either a rule (rule_code) or an element (element_code)".into(),
            });
            scores_failed += 1;
            continue;
        }

        // --- Calculate pass_rate ---
        let pass_rate: Option<f64> = if let Some(rate) = entry.pass_rate {
            Some(rate)
        } else if let (Some(evaluated), Some(passed)) =
            (entry.records_evaluated, entry.records_passed)
        {
            if evaluated > 0 {
                Some((passed as f64 / evaluated as f64) * 100.0)
            } else {
                Some(100.0)
            }
        } else {
            None
        };

        // --- Determine status ---
        let status = if let Some(s) = &entry.status {
            let s = s.trim().to_uppercase();
            match s.as_str() {
                "PASS" | "FAIL" | "WARNING" | "ERROR" | "SKIPPED" => s,
                _ => {
                    errors.push(IngestError {
                        path: path.clone(),
                        message: format!(
                            "invalid status '{s}' — must be PASS, FAIL, WARNING, ERROR, or SKIPPED"
                        ),
                    });
                    scores_failed += 1;
                    continue;
                }
            }
        } else if let Some(rate) = pass_rate {
            let thresh = threshold.unwrap_or(100.0);
            if rate >= thresh {
                "PASS".to_string()
            } else {
                "FAIL".to_string()
            }
        } else {
            "PASS".to_string()
        };

        // overall_score defaults to pass_rate for compatibility
        let overall_score = pass_rate.unwrap_or(0.0);

        // --- INSERT into quality_scores ---
        match sqlx::query(
            r#"
            INSERT INTO quality_scores (
                rule_id, element_id, dimension_id, table_id,
                profiling_run_id, source_system_code,
                records_evaluated, records_passed, records_failed,
                pass_rate, overall_score, status, details,
                tool_name, profiled_at, period_start, period_end
            )
            VALUES ($1, $2, $3, NULL, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14, $14)
            "#,
        )
        .bind(rule_id)
        .bind(element_id)
        .bind(dimension_id)
        .bind(&run_id)
        .bind(source_system_code)
        .bind(entry.records_evaluated)
        .bind(entry.records_passed)
        .bind(entry.records_failed)
        .bind(pass_rate)
        .bind(overall_score)
        .bind(&status)
        .bind(entry.details.as_deref())
        .bind(tool_name)
        .bind(profiled_at)
        .execute(&state.pool)
        .await
        {
            Ok(_) => {
                scores_created += 1;
            }
            Err(e) => {
                errors.push(IngestError {
                    path,
                    message: format!("failed to insert score: {e}"),
                });
                scores_failed += 1;
            }
        }
    }

    // -----------------------------------------------------------------------
    // 3. Log to ingestion_log
    // -----------------------------------------------------------------------
    let duration_ms = started.elapsed().as_millis() as i64;

    let summary_json = serde_json::json!({
        "scores_created": scores_created,
        "scores_failed": scores_failed,
        "run_id": run_id,
    });
    let errors_json = serde_json::to_value(&errors).unwrap_or_default();

    let _log_result = sqlx::query(
        r#"
        INSERT INTO ingestion_log (ingestion_type, source_system_code, summary, errors, duration_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind("quality_scores")
    .bind(source_system_code)
    .bind(&summary_json)
    .bind(&errors_json)
    .bind(duration_ms as i32)
    .execute(&state.pool)
    .await;

    if let Err(e) = &_log_result {
        tracing::warn!(error = %e, "failed to insert ingestion_log record");
    }

    // -----------------------------------------------------------------------
    // 4. Return response
    // -----------------------------------------------------------------------
    Ok(Json(IngestScoresResponse {
        status: "completed".into(),
        scores_created,
        scores_failed,
        errors,
        duration_ms,
    }))
}
