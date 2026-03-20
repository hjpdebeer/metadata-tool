//! Admin panel API endpoints for system settings and lookup table management.
//!
//! All endpoints require the ADMIN role (SEC-001). Lookup table handlers use
//! exhaustive `match` on table names — no dynamic SQL is constructed.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::error::{AppError, AppResult};
use crate::settings::{
    self, mask_value, SystemSettingResponse, SystemSettingRow, TestConnectionResponse,
    UpdateSettingRequest, UpdateSettingResponse,
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
        return Err(AppError::Validation("setting key exceeds 128 characters".into()));
    }
    if body.value.len() > 4000 {
        return Err(AppError::Validation("setting value exceeds 4000 characters".into()));
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

    let token_url = format!(
        "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token"
    );

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
        _ => return Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
    };

    let row = row.ok_or_else(|| {
        AppError::NotFound(format!("row not found in {table_name}: {id}"))
    })?;

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
        "domains" => {
            sqlx::query("DELETE FROM glossary_domains WHERE domain_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "categories" => {
            sqlx::query("DELETE FROM glossary_categories WHERE category_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "term-types" => {
            sqlx::query("DELETE FROM glossary_term_types WHERE term_type_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "classifications" => {
            sqlx::query("DELETE FROM data_classifications WHERE classification_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "units-of-measure" => {
            sqlx::query("DELETE FROM glossary_units_of_measure WHERE unit_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "review-frequencies" => {
            sqlx::query("DELETE FROM glossary_review_frequencies WHERE frequency_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "confidence-levels" => {
            sqlx::query("DELETE FROM glossary_confidence_levels WHERE confidence_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "visibility-levels" => {
            sqlx::query("DELETE FROM glossary_visibility_levels WHERE visibility_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "languages" => {
            sqlx::query("DELETE FROM glossary_languages WHERE language_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "regulatory-tags" => {
            sqlx::query("DELETE FROM glossary_regulatory_tags WHERE tag_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "subject-areas" => {
            sqlx::query("DELETE FROM glossary_subject_areas WHERE subject_area_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        "organisational-units" => {
            sqlx::query("DELETE FROM organisational_units WHERE unit_id = $1")
                .bind(id).execute(&state.pool).await?.rows_affected()
        }
        _ => return Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
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
        _ => return Err(AppError::NotFound(format!("unknown lookup table: {table_name}"))),
    };

    Ok(count)
}

// ---------------------------------------------------------------------------
// Helper: get encryption secret from AppState
// ---------------------------------------------------------------------------

fn encryption_secret(state: &AppState) -> String {
    std::env::var("SETTINGS_ENCRYPTION_KEY")
        .unwrap_or_else(|_| state.config.jwt_secret.clone())
}
