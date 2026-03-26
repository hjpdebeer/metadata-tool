//! Admin panel API endpoints for system settings, lookup table management,
//! and API key management.
//!
//! All endpoints require the ADMIN role (SEC-001). Lookup table handlers use
//! exhaustive `match` on table names — no dynamic SQL is constructed.

use axum::Extension;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::settings::{
    self, SystemSettingResponse, SystemSettingRow, TestConnectionResponse, UpdateSettingRequest,
    UpdateSettingResponse, mask_value,
};

// ---------------------------------------------------------------------------
// Admin role guard (same pattern as users.rs)
// ---------------------------------------------------------------------------

fn require_admin(claims: &Claims) -> AppResult<()> {
    if !claims.roles.iter().any(|r| r == "ADMIN") {
        return Err(AppError::Forbidden(
            "admin role required for this operation".into(),
        ));
    }
    Ok(())
}

// ===========================================================================
// Settings endpoints
// ===========================================================================

// ---------------------------------------------------------------------------
// list_settings — GET /api/v1/admin/settings
// ---------------------------------------------------------------------------

/// List all system settings with masked values for encrypted fields.
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings",
    responses(
        (status = 200, description = "List of all system settings", body = SettingsListResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_settings(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<SettingsListResponse>> {
    require_admin(&claims)?;

    let rows = sqlx::query_as::<_, SystemSettingRow>(
        r#"
        SELECT s.setting_key, s.setting_value, s.is_encrypted, s.category,
               s.display_name, s.description, s.validation_regex,
               s.updated_by, s.updated_at, s.created_at
        FROM system_settings s
        ORDER BY s.category, s.created_at
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    // Fetch updater names in bulk
    let updater_ids: Vec<Uuid> = rows.iter().filter_map(|r| r.updated_by).collect();
    let updater_names: std::collections::HashMap<Uuid, String> = if !updater_ids.is_empty() {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT user_id, display_name FROM users WHERE user_id = ANY($1)",
        )
        .bind(&updater_ids)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .collect()
    } else {
        std::collections::HashMap::new()
    };

    let secret = encryption_secret(&state);

    let settings = rows
        .into_iter()
        .map(|row| {
            let is_set = !row.setting_value.is_empty();
            let display_value = if row.is_encrypted {
                if is_set {
                    // Decrypt then mask for display
                    match settings::get_setting_sync(&row.setting_value, &secret) {
                        Ok(decrypted) => mask_value(&decrypted),
                        Err(_) => "****".to_string(),
                    }
                } else {
                    String::new()
                }
            } else {
                row.setting_value.clone()
            };

            let updated_by_name = row
                .updated_by
                .and_then(|id| updater_names.get(&id).cloned());

            SystemSettingResponse {
                key: row.setting_key,
                value: display_value,
                is_encrypted: row.is_encrypted,
                category: row.category,
                display_name: row.display_name,
                description: row.description,
                validation_regex: row.validation_regex,
                is_set,
                updated_at: row.updated_at,
                updated_by_name,
            }
        })
        .collect();

    // Invalidate and reload cache while we have fresh data
    if let Some(cache) = &state.settings_cache {
        cache.invalidate().await;
    }

    Ok(Json(SettingsListResponse { settings }))
}

/// Wrapper response for settings list.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SettingsListResponse {
    pub settings: Vec<SystemSettingResponse>,
}

// ---------------------------------------------------------------------------
// update_setting — PUT /api/v1/admin/settings/{key}
// ---------------------------------------------------------------------------

/// Update a system setting value.
/// Requires ADMIN role.
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/{key}",
    params(("key" = String, Path, description = "Setting key")),
    request_body = UpdateSettingRequest,
    responses(
        (status = 200, description = "Setting updated", body = UpdateSettingResponse),
        (status = 404, description = "Setting not found")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn update_setting(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(key): Path<String>,
    Json(body): Json<UpdateSettingRequest>,
) -> AppResult<Json<UpdateSettingResponse>> {
    require_admin(&claims)?;

    // SEC-025: Input length validation
    if key.len() > 128 {
        return Err(AppError::Validation(
            "setting key exceeds 128 characters".into(),
        ));
    }
    if body.value.len() > 4000 {
        return Err(AppError::Validation(
            "setting value exceeds 4000 characters".into(),
        ));
    }

    let secret = encryption_secret(&state);
    settings::set_setting(&state.pool, &key, &body.value, claims.sub, &secret).await?;

    // Invalidate cache so next read picks up the new value
    if let Some(cache) = &state.settings_cache {
        cache.invalidate().await;
    }

    let updated_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT updated_at FROM system_settings WHERE setting_key = $1",
    )
    .bind(&key)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(UpdateSettingResponse {
        key,
        is_set: !body.value.is_empty(),
        updated_at,
    }))
}

// ---------------------------------------------------------------------------
// reveal_setting — GET /api/v1/admin/settings/{key}/reveal
// ---------------------------------------------------------------------------

/// Reveal the full decrypted value of a setting.
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/{key}/reveal",
    params(("key" = String, Path, description = "Setting key")),
    responses(
        (status = 200, description = "Revealed setting value", body = RevealSettingResponse),
        (status = 404, description = "Setting not found")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn reveal_setting(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(key): Path<String>,
) -> AppResult<Json<RevealSettingResponse>> {
    require_admin(&claims)?;

    let secret = encryption_secret(&state);
    let value = settings::get_setting(&state.pool, &key, &secret)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("setting not found: {key}")))?;

    Ok(Json(RevealSettingResponse { key, value }))
}

/// Response for revealing a setting value.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RevealSettingResponse {
    pub key: String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// test_connection — POST /api/v1/admin/settings/test-connection/{key}
// ---------------------------------------------------------------------------

/// Test the connection for an API key setting.
/// Requires ADMIN role.
#[utoipa::path(
    post,
    path = "/api/v1/admin/settings/test-connection/{key}",
    params(("key" = String, Path, description = "Setting key to test")),
    responses(
        (status = 200, description = "Connection test result", body = TestConnectionResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn test_connection(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(key): Path<String>,
) -> AppResult<Json<TestConnectionResponse>> {
    require_admin(&claims)?;

    let secret = encryption_secret(&state);
    let api_key = settings::get_setting(&state.pool, &key, &secret)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("setting not found: {key}")))?;

    if api_key.is_empty() {
        return Ok(Json(TestConnectionResponse {
            success: false,
            message: "API key is not configured".to_string(),
        }));
    }

    let result = match key.as_str() {
        "anthropic_api_key" => test_anthropic_key(&api_key).await,
        "openai_api_key" => test_openai_key(&api_key).await,
        "graph_client_secret" => {
            // For Graph, we need tenant_id and client_id too
            let tenant = settings::get_setting(&state.pool, "graph_tenant_id", &secret)
                .await?
                .unwrap_or_default();
            let client_id = settings::get_setting(&state.pool, "graph_client_id", &secret)
                .await?
                .unwrap_or_default();
            test_graph_connection(&tenant, &client_id, &api_key).await
        }
        _ => Ok(TestConnectionResponse {
            success: false,
            message: format!("Test connection not supported for setting: {key}"),
        }),
    };

    Ok(Json(result?))
}

async fn test_anthropic_key(api_key: &str) -> AppResult<TestConnectionResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("http client error: {e}")))?;

    let response = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => Ok(TestConnectionResponse {
            success: true,
            message: "Successfully connected to Anthropic API".to_string(),
        }),
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Ok(TestConnectionResponse {
                success: false,
                message: format!("Anthropic API returned {status}: {body}"),
            })
        }
        Err(e) => Ok(TestConnectionResponse {
            success: false,
            message: format!("Connection failed: {e}"),
        }),
    }
}

async fn test_openai_key(api_key: &str) -> AppResult<TestConnectionResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("http client error: {e}")))?;

    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => Ok(TestConnectionResponse {
            success: true,
            message: "Successfully connected to OpenAI API".to_string(),
        }),
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Ok(TestConnectionResponse {
                success: false,
                message: format!("OpenAI API returned {status}: {body}"),
            })
        }
        Err(e) => Ok(TestConnectionResponse {
            success: false,
            message: format!("Connection failed: {e}"),
        }),
    }
}

async fn test_graph_connection(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
) -> AppResult<TestConnectionResponse> {
    if tenant_id.is_empty() || client_id.is_empty() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Graph Tenant ID and Client ID must be configured first".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("http client error: {e}")))?;

    let token_url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");

    let response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("scope", "https://graph.microsoft.com/.default"),
        ])
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => Ok(TestConnectionResponse {
            success: true,
            message: "Successfully authenticated with Microsoft Graph".to_string(),
        }),
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Ok(TestConnectionResponse {
                success: false,
                message: format!("Microsoft Graph returned {status}: {body}"),
            })
        }
        Err(e) => Ok(TestConnectionResponse {
            success: false,
            message: format!("Connection failed: {e}"),
        }),
    }
}

// ===========================================================================
// Lookup table endpoints (SEC-001: exhaustive match, no dynamic SQL)
// ===========================================================================

/// Generic lookup table row.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct LookupRow {
    pub id: Uuid,
    pub code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    /// Extra fields specific to the table type, serialized as JSON
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// Request body for creating or updating a lookup row.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct LookupRowRequest {
    pub code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

/// Paginated response for lookup rows.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct LookupListResponse {
    pub data: Vec<LookupRow>,
    pub total_count: i64,
}

/// Response for delete usage check.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UsageCountResponse {
    pub usage_count: i64,
    pub table_name: String,
}

/// Query parameters for lookup list.
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct LookupListParams {
    pub search: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Table name to DB mapping (SEC-001: no dynamic SQL)
// ---------------------------------------------------------------------------

// Reserved for future generic lookup table CRUD refactoring (replaces exhaustive match)
#[allow(dead_code)]
struct LookupTableConfig {
    db_table: &'static str,
    pk_column: &'static str,
    code_column: Option<&'static str>,
    name_column: &'static str,
    desc_column: Option<&'static str>,
}

fn resolve_table(table_name: &str) -> AppResult<LookupTableConfig> {
    match table_name {
        "domains" => Ok(LookupTableConfig {
            db_table: "glossary_domains",
            pk_column: "domain_id",
            code_column: Some("domain_code"),
            name_column: "domain_name",
            desc_column: Some("description"),
        }),
        "categories" => Ok(LookupTableConfig {
            db_table: "glossary_categories",
            pk_column: "category_id",
            code_column: None,
            name_column: "category_name",
            desc_column: Some("description"),
        }),
        "term-types" => Ok(LookupTableConfig {
            db_table: "glossary_term_types",
            pk_column: "term_type_id",
            code_column: Some("type_code"),
            name_column: "type_name",
            desc_column: Some("description"),
        }),
        "classifications" => Ok(LookupTableConfig {
            db_table: "data_classifications",
            pk_column: "classification_id",
            code_column: Some("classification_code"),
            name_column: "classification_name",
            desc_column: Some("description"),
        }),
        "units-of-measure" => Ok(LookupTableConfig {
            db_table: "glossary_units_of_measure",
            pk_column: "unit_id",
            code_column: Some("unit_code"),
            name_column: "unit_name",
            desc_column: Some("description"),
        }),
        "review-frequencies" => Ok(LookupTableConfig {
            db_table: "glossary_review_frequencies",
            pk_column: "frequency_id",
            code_column: Some("frequency_code"),
            name_column: "frequency_name",
            desc_column: None,
        }),
        "confidence-levels" => Ok(LookupTableConfig {
            db_table: "glossary_confidence_levels",
            pk_column: "confidence_id",
            code_column: Some("level_code"),
            name_column: "level_name",
            desc_column: Some("description"),
        }),
        "visibility-levels" => Ok(LookupTableConfig {
            db_table: "glossary_visibility_levels",
            pk_column: "visibility_id",
            code_column: Some("visibility_code"),
            name_column: "visibility_name",
            desc_column: Some("description"),
        }),
        "languages" => Ok(LookupTableConfig {
            db_table: "glossary_languages",
            pk_column: "language_id",
            code_column: Some("language_code"),
            name_column: "language_name",
            desc_column: None,
        }),
        "regulatory-tags" => Ok(LookupTableConfig {
            db_table: "glossary_regulatory_tags",
            pk_column: "tag_id",
            code_column: Some("tag_code"),
            name_column: "tag_name",
            desc_column: Some("description"),
        }),
        "subject-areas" => Ok(LookupTableConfig {
            db_table: "glossary_subject_areas",
            pk_column: "subject_area_id",
            code_column: Some("area_code"),
            name_column: "area_name",
            desc_column: Some("description"),
        }),
        "organisational-units" => Ok(LookupTableConfig {
            db_table: "organisational_units",
            pk_column: "unit_id",
            code_column: Some("unit_code"),
            name_column: "unit_name",
            desc_column: Some("description"),
        }),
        // Application lookups
        "app-classifications" => Ok(LookupTableConfig {
            db_table: "application_classifications",
            pk_column: "classification_id",
            code_column: Some("classification_code"),
            name_column: "classification_name",
            desc_column: Some("description"),
        }),
        "dr-tiers" => Ok(LookupTableConfig {
            db_table: "disaster_recovery_tiers",
            pk_column: "dr_tier_id",
            code_column: Some("tier_code"),
            name_column: "tier_name",
            desc_column: Some("description"),
        }),
        "lifecycle-stages" => Ok(LookupTableConfig {
            db_table: "application_lifecycle_stages",
            pk_column: "stage_id",
            code_column: Some("stage_code"),
            name_column: "stage_name",
            desc_column: Some("description"),
        }),
        "criticality-tiers" => Ok(LookupTableConfig {
            db_table: "application_criticality_tiers",
            pk_column: "tier_id",
            code_column: Some("tier_code"),
            name_column: "tier_name",
            desc_column: Some("description"),
        }),
        "risk-ratings" => Ok(LookupTableConfig {
            db_table: "application_risk_ratings",
            pk_column: "rating_id",
            code_column: Some("rating_code"),
            name_column: "rating_name",
            desc_column: Some("description"),
        }),
        // Process lookups
        "process-categories" => Ok(LookupTableConfig {
            db_table: "process_categories",
            pk_column: "category_id",
            code_column: None,
            name_column: "category_name",
            desc_column: Some("description"),
        }),
        // Glossary tags
        "tags" => Ok(LookupTableConfig {
            db_table: "glossary_tags",
            pk_column: "tag_id",
            code_column: None,
            name_column: "tag_name",
            desc_column: Some("description"),
        }),
        _ => Err(AppError::NotFound(format!(
            "unknown lookup table: {table_name}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// list_lookup — GET /api/v1/admin/lookups/{table_name}
// ---------------------------------------------------------------------------

/// List rows from a lookup table.
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/admin/lookups/{table_name}",
    params(
        ("table_name" = String, Path, description = "Lookup table name"),
        LookupListParams
    ),
    responses(
        (status = 200, description = "Lookup table rows", body = LookupListResponse),
        (status = 404, description = "Unknown table name")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_lookup(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(table_name): Path<String>,
    Query(params): Query<LookupListParams>,
) -> AppResult<Json<LookupListResponse>> {
    require_admin(&claims)?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(100).clamp(1, 500);
    let offset = (page - 1) * page_size;
    let search = params.search.as_deref();

    // SEC-001: each table has its own static SQL — no dynamic SQL
    match table_name.as_str() {
        "domains" => list_lookup_impl(
            &state.pool,
            r#"SELECT domain_id AS id, domain_code AS code, domain_name AS name, description
               FROM glossary_domains
               WHERE ($1::TEXT IS NULL OR domain_name ILIKE '%' || $1 || '%' OR domain_code ILIKE '%' || $1 || '%')
               ORDER BY domain_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_domains
               WHERE ($1::TEXT IS NULL OR domain_name ILIKE '%' || $1 || '%' OR domain_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "categories" => list_lookup_impl(
            &state.pool,
            r#"SELECT category_id AS id, NULL::VARCHAR AS code, category_name AS name, description
               FROM glossary_categories
               WHERE ($1::TEXT IS NULL OR category_name ILIKE '%' || $1 || '%')
               ORDER BY category_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_categories
               WHERE ($1::TEXT IS NULL OR category_name ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "term-types" => list_lookup_impl(
            &state.pool,
            r#"SELECT term_type_id AS id, type_code AS code, type_name AS name, description
               FROM glossary_term_types
               WHERE ($1::TEXT IS NULL OR type_name ILIKE '%' || $1 || '%' OR type_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, type_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_term_types
               WHERE ($1::TEXT IS NULL OR type_name ILIKE '%' || $1 || '%' OR type_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "classifications" => list_lookup_impl(
            &state.pool,
            r#"SELECT classification_id AS id, classification_code AS code, classification_name AS name, description
               FROM data_classifications
               WHERE ($1::TEXT IS NULL OR classification_name ILIKE '%' || $1 || '%' OR classification_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, classification_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM data_classifications
               WHERE ($1::TEXT IS NULL OR classification_name ILIKE '%' || $1 || '%' OR classification_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "units-of-measure" => list_lookup_impl(
            &state.pool,
            r#"SELECT unit_id AS id, unit_code AS code, unit_name AS name, description
               FROM glossary_units_of_measure
               WHERE ($1::TEXT IS NULL OR unit_name ILIKE '%' || $1 || '%' OR unit_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, unit_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_units_of_measure
               WHERE ($1::TEXT IS NULL OR unit_name ILIKE '%' || $1 || '%' OR unit_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "review-frequencies" => list_lookup_impl(
            &state.pool,
            r#"SELECT frequency_id AS id, frequency_code AS code, frequency_name AS name, NULL::TEXT AS description
               FROM glossary_review_frequencies
               WHERE ($1::TEXT IS NULL OR frequency_name ILIKE '%' || $1 || '%' OR frequency_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, frequency_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_review_frequencies
               WHERE ($1::TEXT IS NULL OR frequency_name ILIKE '%' || $1 || '%' OR frequency_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "confidence-levels" => list_lookup_impl(
            &state.pool,
            r#"SELECT confidence_id AS id, level_code AS code, level_name AS name, description
               FROM glossary_confidence_levels
               WHERE ($1::TEXT IS NULL OR level_name ILIKE '%' || $1 || '%' OR level_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, level_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_confidence_levels
               WHERE ($1::TEXT IS NULL OR level_name ILIKE '%' || $1 || '%' OR level_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "visibility-levels" => list_lookup_impl(
            &state.pool,
            r#"SELECT visibility_id AS id, visibility_code AS code, visibility_name AS name, description
               FROM glossary_visibility_levels
               WHERE ($1::TEXT IS NULL OR visibility_name ILIKE '%' || $1 || '%' OR visibility_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, visibility_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_visibility_levels
               WHERE ($1::TEXT IS NULL OR visibility_name ILIKE '%' || $1 || '%' OR visibility_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "languages" => list_lookup_impl(
            &state.pool,
            r#"SELECT language_id AS id, language_code AS code, language_name AS name, NULL::TEXT AS description
               FROM glossary_languages
               WHERE ($1::TEXT IS NULL OR language_name ILIKE '%' || $1 || '%' OR language_code ILIKE '%' || $1 || '%')
               ORDER BY language_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_languages
               WHERE ($1::TEXT IS NULL OR language_name ILIKE '%' || $1 || '%' OR language_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "regulatory-tags" => list_lookup_impl(
            &state.pool,
            r#"SELECT tag_id AS id, tag_code AS code, tag_name AS name, description
               FROM glossary_regulatory_tags
               WHERE ($1::TEXT IS NULL OR tag_name ILIKE '%' || $1 || '%' OR tag_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, tag_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_regulatory_tags
               WHERE ($1::TEXT IS NULL OR tag_name ILIKE '%' || $1 || '%' OR tag_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "subject-areas" => list_lookup_impl(
            &state.pool,
            r#"SELECT subject_area_id AS id, area_code AS code, area_name AS name, description
               FROM glossary_subject_areas
               WHERE ($1::TEXT IS NULL OR area_name ILIKE '%' || $1 || '%' OR area_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, area_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_subject_areas
               WHERE ($1::TEXT IS NULL OR area_name ILIKE '%' || $1 || '%' OR area_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "organisational-units" => list_lookup_impl(
            &state.pool,
            r#"SELECT unit_id AS id, unit_code AS code, unit_name AS name, description
               FROM organisational_units
               WHERE ($1::TEXT IS NULL OR unit_name ILIKE '%' || $1 || '%' OR unit_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, unit_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM organisational_units
               WHERE ($1::TEXT IS NULL OR unit_name ILIKE '%' || $1 || '%' OR unit_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        // Application lookups
        "app-classifications" => list_lookup_impl(
            &state.pool,
            r#"SELECT classification_id AS id, classification_code AS code, classification_name AS name, description
               FROM application_classifications
               WHERE ($1::TEXT IS NULL OR classification_name ILIKE '%' || $1 || '%' OR classification_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, classification_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM application_classifications
               WHERE ($1::TEXT IS NULL OR classification_name ILIKE '%' || $1 || '%' OR classification_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "dr-tiers" => list_lookup_impl(
            &state.pool,
            r#"SELECT dr_tier_id AS id, tier_code AS code, tier_name AS name, description
               FROM disaster_recovery_tiers
               WHERE ($1::TEXT IS NULL OR tier_name ILIKE '%' || $1 || '%' OR tier_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, tier_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM disaster_recovery_tiers
               WHERE ($1::TEXT IS NULL OR tier_name ILIKE '%' || $1 || '%' OR tier_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "lifecycle-stages" => list_lookup_impl(
            &state.pool,
            r#"SELECT stage_id AS id, stage_code AS code, stage_name AS name, description
               FROM application_lifecycle_stages
               WHERE ($1::TEXT IS NULL OR stage_name ILIKE '%' || $1 || '%' OR stage_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, stage_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM application_lifecycle_stages
               WHERE ($1::TEXT IS NULL OR stage_name ILIKE '%' || $1 || '%' OR stage_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "criticality-tiers" => list_lookup_impl(
            &state.pool,
            r#"SELECT tier_id AS id, tier_code AS code, tier_name AS name, description
               FROM application_criticality_tiers
               WHERE ($1::TEXT IS NULL OR tier_name ILIKE '%' || $1 || '%' OR tier_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, tier_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM application_criticality_tiers
               WHERE ($1::TEXT IS NULL OR tier_name ILIKE '%' || $1 || '%' OR tier_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        "risk-ratings" => list_lookup_impl(
            &state.pool,
            r#"SELECT rating_id AS id, rating_code AS code, rating_name AS name, description
               FROM application_risk_ratings
               WHERE ($1::TEXT IS NULL OR rating_name ILIKE '%' || $1 || '%' OR rating_code ILIKE '%' || $1 || '%')
               ORDER BY display_order, rating_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM application_risk_ratings
               WHERE ($1::TEXT IS NULL OR rating_name ILIKE '%' || $1 || '%' OR rating_code ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        // Process lookups
        "process-categories" => list_lookup_impl(
            &state.pool,
            r#"SELECT category_id AS id, NULL::VARCHAR AS code, category_name AS name, description
               FROM process_categories
               WHERE ($1::TEXT IS NULL OR category_name ILIKE '%' || $1 || '%')
               ORDER BY category_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM process_categories
               WHERE ($1::TEXT IS NULL OR category_name ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        // Glossary tags
        "tags" => list_lookup_impl(
            &state.pool,
            r#"SELECT tag_id AS id, NULL::VARCHAR AS code, tag_name AS name, description
               FROM glossary_tags
               WHERE ($1::TEXT IS NULL OR tag_name ILIKE '%' || $1 || '%')
               ORDER BY tag_name LIMIT $2 OFFSET $3"#,
            r#"SELECT COUNT(*) FROM glossary_tags
               WHERE ($1::TEXT IS NULL OR tag_name ILIKE '%' || $1 || '%')"#,
            search, page_size, offset,
        ).await,
        _ => Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
    }
}

/// Internal helper to execute a list query with count.
async fn list_lookup_impl(
    pool: &sqlx::PgPool,
    list_sql: &str,
    count_sql: &str,
    search: Option<&str>,
    page_size: i64,
    offset: i64,
) -> AppResult<Json<LookupListResponse>> {
    let total_count = sqlx::query_scalar::<_, i64>(count_sql)
        .bind(search)
        .fetch_one(pool)
        .await?;

    let rows = sqlx::query_as::<_, LookupQueryRow>(list_sql)
        .bind(search)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let data = rows
        .into_iter()
        .map(|r| LookupRow {
            id: r.id,
            code: r.code,
            name: r.name,
            description: r.description,
            extra: None,
        })
        .collect();

    Ok(Json(LookupListResponse { data, total_count }))
}

#[derive(Debug, sqlx::FromRow)]
struct LookupQueryRow {
    id: Uuid,
    code: Option<String>,
    name: String,
    description: Option<String>,
}

// ---------------------------------------------------------------------------
// create_lookup — POST /api/v1/admin/lookups/{table_name}
// ---------------------------------------------------------------------------

/// Add a row to a lookup table.
/// Requires ADMIN role.
#[utoipa::path(
    post,
    path = "/api/v1/admin/lookups/{table_name}",
    params(("table_name" = String, Path, description = "Lookup table name")),
    request_body = LookupRowRequest,
    responses(
        (status = 201, description = "Row created", body = LookupRow),
        (status = 404, description = "Unknown table name")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn create_lookup(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(table_name): Path<String>,
    Json(body): Json<LookupRowRequest>,
) -> AppResult<(StatusCode, Json<LookupRow>)> {
    require_admin(&claims)?;

    if body.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }

    // SEC-001: exhaustive match on table names
    let row = match table_name.as_str() {
        "domains" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for domains".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_domains (domain_code, domain_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING domain_id AS id, domain_code AS code, domain_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "categories" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_categories (category_name, description)
                   VALUES ($1, $2)
                   RETURNING category_id AS id, NULL::VARCHAR AS code, category_name AS name, description"#,
            )
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "term-types" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for term types".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_term_types (type_code, type_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING term_type_id AS id, type_code AS code, type_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "classifications" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for classifications".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO data_classifications (classification_code, classification_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING classification_id AS id, classification_code AS code, classification_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "units-of-measure" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for units of measure".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_units_of_measure (unit_code, unit_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING unit_id AS id, unit_code AS code, unit_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "review-frequencies" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for review frequencies".into()));
            }
            let months = body.extra
                .as_ref()
                .and_then(|e| e.get("months_interval"))
                .and_then(|v| v.as_i64())
                .unwrap_or(12) as i32;
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_review_frequencies (frequency_code, frequency_name, months_interval)
                   VALUES ($1, $2, $3)
                   RETURNING frequency_id AS id, frequency_code AS code, frequency_name AS name, NULL::TEXT AS description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(months)
            .fetch_one(&state.pool)
            .await?
        }
        "confidence-levels" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for confidence levels".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_confidence_levels (level_code, level_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING confidence_id AS id, level_code AS code, level_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "visibility-levels" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for visibility levels".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_visibility_levels (visibility_code, visibility_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING visibility_id AS id, visibility_code AS code, visibility_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "languages" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for languages".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_languages (language_code, language_name)
                   VALUES ($1, $2)
                   RETURNING language_id AS id, language_code AS code, language_name AS name, NULL::TEXT AS description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .fetch_one(&state.pool)
            .await?
        }
        "regulatory-tags" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for regulatory tags".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_regulatory_tags (tag_code, tag_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING tag_id AS id, tag_code AS code, tag_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "subject-areas" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for subject areas".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_subject_areas (area_code, area_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING subject_area_id AS id, area_code AS code, area_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "organisational-units" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for organisational units".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO organisational_units (unit_code, unit_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING unit_id AS id, unit_code AS code, unit_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        // Application lookups
        "app-classifications" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for app classifications".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO application_classifications (classification_code, classification_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING classification_id AS id, classification_code AS code, classification_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "dr-tiers" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for DR tiers".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO disaster_recovery_tiers (tier_code, tier_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING dr_tier_id AS id, tier_code AS code, tier_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "lifecycle-stages" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for lifecycle stages".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO application_lifecycle_stages (stage_code, stage_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING stage_id AS id, stage_code AS code, stage_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "criticality-tiers" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for criticality tiers".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO application_criticality_tiers (tier_code, tier_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING tier_id AS id, tier_code AS code, tier_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        "risk-ratings" => {
            let code = body.code.as_deref().unwrap_or("").to_string();
            if code.is_empty() {
                return Err(AppError::Validation("code is required for risk ratings".into()));
            }
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO application_risk_ratings (rating_code, rating_name, description)
                   VALUES ($1, $2, $3)
                   RETURNING rating_id AS id, rating_code AS code, rating_name AS name, description"#,
            )
            .bind(&code)
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        // Process lookups
        "process-categories" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO process_categories (category_name, description)
                   VALUES ($1, $2)
                   RETURNING category_id AS id, NULL::VARCHAR AS code, category_name AS name, description"#,
            )
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        // Glossary tags
        "tags" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"INSERT INTO glossary_tags (tag_name, description)
                   VALUES ($1, $2)
                   RETURNING tag_id AS id, NULL::VARCHAR AS code, tag_name AS name, description"#,
            )
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .fetch_one(&state.pool)
            .await?
        }
        _ => return Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
    };

    Ok((
        StatusCode::CREATED,
        Json(LookupRow {
            id: row.id,
            code: row.code,
            name: row.name,
            description: row.description,
            extra: None,
        }),
    ))
}

// ---------------------------------------------------------------------------
// update_lookup — PUT /api/v1/admin/lookups/{table_name}/{id}
// ---------------------------------------------------------------------------

/// Update a row in a lookup table.
/// Requires ADMIN role.
#[utoipa::path(
    put,
    path = "/api/v1/admin/lookups/{table_name}/{id}",
    params(
        ("table_name" = String, Path, description = "Lookup table name"),
        ("id" = Uuid, Path, description = "Row ID")
    ),
    request_body = LookupRowRequest,
    responses(
        (status = 200, description = "Row updated", body = LookupRow),
        (status = 404, description = "Not found")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn update_lookup(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((table_name, id)): Path<(String, Uuid)>,
    Json(body): Json<LookupRowRequest>,
) -> AppResult<Json<LookupRow>> {
    require_admin(&claims)?;

    if body.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }

    // SEC-001: exhaustive match
    let row = match table_name.as_str() {
        "domains" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_domains
                   SET domain_code = COALESCE($1, domain_code),
                       domain_name = $2,
                       description = $3,
                       updated_at = CURRENT_TIMESTAMP
                   WHERE domain_id = $4
                   RETURNING domain_id AS id, domain_code AS code, domain_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "categories" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_categories
                   SET category_name = $1, description = $2
                   WHERE category_id = $3
                   RETURNING category_id AS id, NULL::VARCHAR AS code, category_name AS name, description"#,
            )
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "term-types" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_term_types
                   SET type_code = COALESCE($1, type_code), type_name = $2, description = $3
                   WHERE term_type_id = $4
                   RETURNING term_type_id AS id, type_code AS code, type_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "classifications" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE data_classifications
                   SET classification_code = COALESCE($1, classification_code),
                       classification_name = $2, description = $3
                   WHERE classification_id = $4
                   RETURNING classification_id AS id, classification_code AS code, classification_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "units-of-measure" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_units_of_measure
                   SET unit_code = COALESCE($1, unit_code), unit_name = $2, description = $3
                   WHERE unit_id = $4
                   RETURNING unit_id AS id, unit_code AS code, unit_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "review-frequencies" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_review_frequencies
                   SET frequency_code = COALESCE($1, frequency_code), frequency_name = $2
                   WHERE frequency_id = $3
                   RETURNING frequency_id AS id, frequency_code AS code, frequency_name AS name, NULL::TEXT AS description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "confidence-levels" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_confidence_levels
                   SET level_code = COALESCE($1, level_code), level_name = $2, description = $3
                   WHERE confidence_id = $4
                   RETURNING confidence_id AS id, level_code AS code, level_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "visibility-levels" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_visibility_levels
                   SET visibility_code = COALESCE($1, visibility_code), visibility_name = $2, description = $3
                   WHERE visibility_id = $4
                   RETURNING visibility_id AS id, visibility_code AS code, visibility_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "languages" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_languages
                   SET language_code = COALESCE($1, language_code), language_name = $2
                   WHERE language_id = $3
                   RETURNING language_id AS id, language_code AS code, language_name AS name, NULL::TEXT AS description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "regulatory-tags" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_regulatory_tags
                   SET tag_code = COALESCE($1, tag_code), tag_name = $2, description = $3
                   WHERE tag_id = $4
                   RETURNING tag_id AS id, tag_code AS code, tag_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "subject-areas" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_subject_areas
                   SET area_code = COALESCE($1, area_code), area_name = $2, description = $3
                   WHERE subject_area_id = $4
                   RETURNING subject_area_id AS id, area_code AS code, area_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "organisational-units" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE organisational_units
                   SET unit_code = COALESCE($1, unit_code), unit_name = $2, description = $3
                   WHERE unit_id = $4
                   RETURNING unit_id AS id, unit_code AS code, unit_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        // Application lookups
        "app-classifications" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE application_classifications
                   SET classification_code = COALESCE($1, classification_code),
                       classification_name = $2, description = $3
                   WHERE classification_id = $4
                   RETURNING classification_id AS id, classification_code AS code, classification_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "dr-tiers" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE disaster_recovery_tiers
                   SET tier_code = COALESCE($1, tier_code),
                       tier_name = $2, description = $3
                   WHERE dr_tier_id = $4
                   RETURNING dr_tier_id AS id, tier_code AS code, tier_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "lifecycle-stages" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE application_lifecycle_stages
                   SET stage_code = COALESCE($1, stage_code),
                       stage_name = $2, description = $3
                   WHERE stage_id = $4
                   RETURNING stage_id AS id, stage_code AS code, stage_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "criticality-tiers" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE application_criticality_tiers
                   SET tier_code = COALESCE($1, tier_code),
                       tier_name = $2, description = $3
                   WHERE tier_id = $4
                   RETURNING tier_id AS id, tier_code AS code, tier_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        "risk-ratings" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE application_risk_ratings
                   SET rating_code = COALESCE($1, rating_code),
                       rating_name = $2, description = $3
                   WHERE rating_id = $4
                   RETURNING rating_id AS id, rating_code AS code, rating_name AS name, description"#,
            )
            .bind(body.code.as_deref())
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        // Process lookups
        "process-categories" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE process_categories
                   SET category_name = $1, description = $2
                   WHERE category_id = $3
                   RETURNING category_id AS id, NULL::VARCHAR AS code, category_name AS name, description"#,
            )
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        // Glossary tags
        "tags" => {
            sqlx::query_as::<_, LookupQueryRow>(
                r#"UPDATE glossary_tags
                   SET tag_name = $1, description = $2
                   WHERE tag_id = $3
                   RETURNING tag_id AS id, NULL::VARCHAR AS code, tag_name AS name, description"#,
            )
            .bind(body.name.trim())
            .bind(body.description.as_deref())
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
        }
        _ => return Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
    };

    let row =
        row.ok_or_else(|| AppError::NotFound(format!("row not found in {table_name}: {id}")))?;

    Ok(Json(LookupRow {
        id: row.id,
        code: row.code,
        name: row.name,
        description: row.description,
        extra: None,
    }))
}

// ---------------------------------------------------------------------------
// delete_lookup — DELETE /api/v1/admin/lookups/{table_name}/{id}
// ---------------------------------------------------------------------------

/// Delete a row from a lookup table (with usage check).
/// Requires ADMIN role.
#[utoipa::path(
    delete,
    path = "/api/v1/admin/lookups/{table_name}/{id}",
    params(
        ("table_name" = String, Path, description = "Lookup table name"),
        ("id" = Uuid, Path, description = "Row ID")
    ),
    responses(
        (status = 204, description = "Row deleted"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Row is in use and cannot be deleted")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn delete_lookup(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((table_name, id)): Path<(String, Uuid)>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;

    // First check usage count
    let usage_count = get_usage_count(&state.pool, &table_name, id).await?;
    if usage_count > 0 {
        // Get the name for a helpful error message
        let cfg = resolve_table(&table_name)?;
        let _name = cfg.name_column; // used in message below
        return Err(AppError::Conflict(format!(
            "cannot delete: this item is referenced by {usage_count} entities"
        )));
    }

    // SEC-001: exhaustive match for the actual delete
    let rows_affected = match table_name.as_str() {
        "domains" => sqlx::query("DELETE FROM glossary_domains WHERE domain_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        "categories" => sqlx::query("DELETE FROM glossary_categories WHERE category_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        "term-types" => sqlx::query("DELETE FROM glossary_term_types WHERE term_type_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        "classifications" => {
            sqlx::query("DELETE FROM data_classifications WHERE classification_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "units-of-measure" => {
            sqlx::query("DELETE FROM glossary_units_of_measure WHERE unit_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "review-frequencies" => {
            sqlx::query("DELETE FROM glossary_review_frequencies WHERE frequency_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "confidence-levels" => {
            sqlx::query("DELETE FROM glossary_confidence_levels WHERE confidence_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "visibility-levels" => {
            sqlx::query("DELETE FROM glossary_visibility_levels WHERE visibility_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "languages" => sqlx::query("DELETE FROM glossary_languages WHERE language_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        "regulatory-tags" => sqlx::query("DELETE FROM glossary_regulatory_tags WHERE tag_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        "subject-areas" => {
            sqlx::query("DELETE FROM glossary_subject_areas WHERE subject_area_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "organisational-units" => {
            sqlx::query("DELETE FROM organisational_units WHERE unit_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        // Application lookups
        "app-classifications" => {
            sqlx::query("DELETE FROM application_classifications WHERE classification_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "dr-tiers" => sqlx::query("DELETE FROM disaster_recovery_tiers WHERE dr_tier_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        "lifecycle-stages" => {
            sqlx::query("DELETE FROM application_lifecycle_stages WHERE stage_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "criticality-tiers" => {
            sqlx::query("DELETE FROM application_criticality_tiers WHERE tier_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        "risk-ratings" => sqlx::query("DELETE FROM application_risk_ratings WHERE rating_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        // Process lookups
        "process-categories" => {
            sqlx::query("DELETE FROM process_categories WHERE category_id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?
                .rows_affected()
        }
        // Glossary tags
        "tags" => sqlx::query("DELETE FROM glossary_tags WHERE tag_id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?
            .rows_affected(),
        _ => {
            return Err(AppError::NotFound(format!(
                "unknown lookup table: {table_name}"
            )));
        }
    };

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "row not found in {table_name}: {id}"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// get_usage_count — GET /api/v1/admin/lookups/{table_name}/{id}/usage-count
// ---------------------------------------------------------------------------

/// Check usage count for a lookup row before deletion.
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/admin/lookups/{table_name}/{id}/usage-count",
    params(
        ("table_name" = String, Path, description = "Lookup table name"),
        ("id" = Uuid, Path, description = "Row ID")
    ),
    responses(
        (status = 200, description = "Usage count", body = UsageCountResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn get_lookup_usage_count(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((table_name, id)): Path<(String, Uuid)>,
) -> AppResult<Json<UsageCountResponse>> {
    require_admin(&claims)?;

    let usage_count = get_usage_count(&state.pool, &table_name, id).await?;

    Ok(Json(UsageCountResponse {
        usage_count,
        table_name,
    }))
}

/// Count references to a lookup value across relevant tables.
/// SEC-001: no dynamic SQL — each table has specific static queries.
async fn get_usage_count(pool: &sqlx::PgPool, table_name: &str, id: Uuid) -> AppResult<i64> {
    let count: i64 = match table_name {
        "domains" => {
            let gt = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE domain_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?;
            let de = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM data_elements WHERE domain_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?;
            gt + de
        }
        "categories" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE category_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "term-types" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE term_type_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "classifications" => {
            let gt = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE classification_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?;
            let de = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM data_elements WHERE classification_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?;
            gt + de
        }
        "units-of-measure" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE unit_of_measure_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "review-frequencies" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE review_frequency_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "confidence-levels" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE confidence_level_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "visibility-levels" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE visibility_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "languages" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_terms WHERE language_id = $1 AND deleted_at IS NULL",
            ).bind(id).fetch_one(pool).await?
        }
        "regulatory-tags" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_term_regulatory_tags WHERE tag_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        "subject-areas" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_term_subject_areas WHERE subject_area_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        "organisational-units" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM organisational_units WHERE parent_unit_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        // Application lookups
        "app-classifications" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM applications WHERE classification_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        "dr-tiers" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM applications WHERE dr_tier_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        "lifecycle-stages" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM applications WHERE lifecycle_stage_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        "criticality-tiers" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM applications WHERE criticality_tier_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        "risk-ratings" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM applications WHERE risk_rating_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        // Process lookups
        "process-categories" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM business_processes WHERE category_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        // Glossary tags
        "tags" => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM glossary_term_tags WHERE tag_id = $1",
            ).bind(id).fetch_one(pool).await?
        }
        _ => return Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
    };

    Ok(count)
}

// ---------------------------------------------------------------------------
// Helper: get encryption secret from AppState
// ---------------------------------------------------------------------------

fn encryption_secret(state: &AppState) -> String {
    std::env::var("SETTINGS_ENCRYPTION_KEY").unwrap_or_else(|_| state.config.jwt_secret.clone())
}

// ===========================================================================
// API Key Management Endpoints
// ===========================================================================

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for creating a new API key.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    pub key_name: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Response returned when an API key is created.
/// Contains the full key which is shown ONCE and never stored.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct CreateApiKeyResponse {
    pub key_id: Uuid,
    pub key_name: String,
    /// The full API key — shown ONCE. Not stored in the database.
    pub api_key: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// A single API key row for listing (no secret material).
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ApiKeyListItem {
    pub key_id: Uuid,
    pub key_name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by_name: Option<String>,
}

/// Response containing the list of API keys.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<ApiKeyListItem>,
}

/// Internal row type for API key list query.
#[derive(Debug, sqlx::FromRow)]
struct ApiKeyRow {
    key_id: Uuid,
    key_name: String,
    key_prefix: String,
    scopes: Vec<String>,
    is_active: bool,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    created_by_name: Option<String>,
}

// ---------------------------------------------------------------------------
// create_api_key — POST /api/v1/admin/api-keys
// ---------------------------------------------------------------------------

/// Generate a new API key for service account authentication.
///
/// Generates a random 48-character key with "mdt_" prefix. The full key is
/// returned ONCE in the response. Only the bcrypt hash and prefix are stored.
/// Requires ADMIN role.
#[utoipa::path(
    post,
    path = "/api/v1/admin/api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "API key created", body = CreateApiKeyResponse),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateApiKeyRequest>,
) -> AppResult<Json<CreateApiKeyResponse>> {
    require_admin(&claims)?;

    // Validate inputs
    let key_name = body.key_name.trim().to_string();
    if key_name.is_empty() {
        return Err(AppError::Validation("key_name is required".into()));
    }
    if key_name.len() > 128 {
        return Err(AppError::Validation(
            "key_name exceeds 128 characters".into(),
        ));
    }
    if body.scopes.is_empty() {
        return Err(AppError::Validation(
            "at least one scope is required".into(),
        ));
    }

    // Validate scope values
    let valid_scopes = [
        "ingest:technical",
        "ingest:elements",
        "ingest:link-columns",
        "read:all",
        "read:technical",
    ];
    for scope in &body.scopes {
        if !valid_scopes.contains(&scope.as_str()) {
            return Err(AppError::Validation(format!(
                "invalid scope '{}'. Valid scopes: {}",
                scope,
                valid_scopes.join(", ")
            )));
        }
    }

    // Generate random 48-char key with "mdt_" prefix.
    // Scope the RNG so it's dropped before any .await (ThreadRng is !Send).
    let (full_key, key_prefix) = {
        let charset: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::rng();
        let random_part: String = (0..48)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();
        let full_key = format!("mdt_{random_part}");
        let key_prefix = full_key[..8].to_string();
        (full_key, key_prefix)
    };

    // Hash with pgcrypto bcrypt
    let key_hash: String = sqlx::query_scalar("SELECT crypt($1, gen_salt('bf', 10))")
        .bind(&full_key)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to hash API key: {e}")))?;

    // Insert into api_keys
    let key_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO api_keys (key_name, key_hash, key_prefix, scopes, created_by, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING key_id
        "#,
    )
    .bind(&key_name)
    .bind(&key_hash)
    .bind(&key_prefix)
    .bind(&body.scopes)
    .bind(claims.sub)
    .bind(body.expires_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CreateApiKeyResponse {
        key_id,
        key_name,
        api_key: full_key,
        key_prefix,
        scopes: body.scopes,
        expires_at: body.expires_at,
    }))
}

// ---------------------------------------------------------------------------
// list_api_keys — GET /api/v1/admin/api-keys
// ---------------------------------------------------------------------------

/// List all API keys with metadata (no secret material).
/// Requires ADMIN role.
#[utoipa::path(
    get,
    path = "/api/v1/admin/api-keys",
    responses(
        (status = 200, description = "List of API keys", body = ApiKeyListResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<ApiKeyListResponse>> {
    require_admin(&claims)?;

    let rows = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT ak.key_id, ak.key_name, ak.key_prefix, ak.scopes, ak.is_active,
               ak.last_used_at, ak.created_at, ak.expires_at,
               u.display_name AS created_by_name
        FROM api_keys ak
        LEFT JOIN users u ON u.user_id = ak.created_by
        ORDER BY ak.created_at DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let api_keys = rows
        .into_iter()
        .map(|r| ApiKeyListItem {
            key_id: r.key_id,
            key_name: r.key_name,
            key_prefix: r.key_prefix,
            scopes: r.scopes,
            is_active: r.is_active,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
            expires_at: r.expires_at,
            created_by_name: r.created_by_name,
        })
        .collect();

    Ok(Json(ApiKeyListResponse { api_keys }))
}

// ---------------------------------------------------------------------------
// deactivate_api_key — DELETE /api/v1/admin/api-keys/{key_id}
// ---------------------------------------------------------------------------

/// Deactivate an API key (soft delete).
/// Requires ADMIN role.
#[utoipa::path(
    delete,
    path = "/api/v1/admin/api-keys/{key_id}",
    params(("key_id" = Uuid, Path, description = "API key ID")),
    responses(
        (status = 204, description = "API key deactivated"),
        (status = 404, description = "API key not found")
    ),
    security(("bearer_auth" = [])),
    tag = "admin"
)]
pub async fn deactivate_api_key(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(key_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    require_admin(&claims)?;

    let rows_affected = sqlx::query(
        "UPDATE api_keys SET is_active = FALSE, updated_at = CURRENT_TIMESTAMP WHERE key_id = $1",
    )
    .bind(key_id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("API key not found: {key_id}")));
    }

    Ok(StatusCode::NO_CONTENT)
}
