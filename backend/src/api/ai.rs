use axum::Extension;
use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::ai::*;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Helper: fetch lookup table values with UUIDs for AI prompts
// (CODING_STANDARDS Section 15.6)
// ---------------------------------------------------------------------------

/// Row type for `SELECT id, name` queries against lookup tables.
#[derive(sqlx::FromRow)]
struct IdName {
    id: Uuid,
    name: String,
}

/// Fetch all glossary-related lookup tables as `{id, name}` arrays.
/// These are embedded in the AI prompt so the model returns a UUID directly
/// instead of guessing a display name (eliminates fuzzy matching).
async fn fetch_glossary_lookups(pool: &sqlx::PgPool) -> Result<serde_json::Value, AppError> {
    let domains = sqlx::query_as::<_, IdName>(
        "SELECT domain_id AS id, domain_name AS name FROM glossary_domains ORDER BY domain_name",
    )
    .fetch_all(pool)
    .await?;

    let categories = sqlx::query_as::<_, IdName>(
        "SELECT category_id AS id, category_name AS name FROM glossary_categories ORDER BY category_name",
    )
    .fetch_all(pool)
    .await?;

    let classifications = sqlx::query_as::<_, IdName>(
        "SELECT classification_id AS id, classification_name AS name FROM data_classifications ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    let term_types = sqlx::query_as::<_, IdName>(
        "SELECT term_type_id AS id, type_name AS name FROM glossary_term_types ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    let units = sqlx::query_as::<_, IdName>(
        "SELECT unit_id AS id, unit_name AS name FROM glossary_units_of_measure ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    Ok(serde_json::json!({
        "domain": domains.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
        "category": categories.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
        "data_classification": classifications.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
        "term_type": term_types.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
        "unit_of_measure": units.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// Helper: fetch entity data as JSON for the AI prompt
// ---------------------------------------------------------------------------

async fn fetch_entity_data(
    pool: &sqlx::PgPool,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(serde_json::Value, Vec<String>), AppError> {
    match entity_type {
        "glossary_term" => {
            let row = sqlx::query_as::<_, crate::domain::glossary::GlossaryTerm>(
                r#"
                SELECT
                    term_id, term_name, term_code, definition, abbreviation,
                    business_context, examples, definition_notes, counter_examples,
                    formula, unit_of_measure_id,
                    term_type_id, domain_id, category_id, classification_id,
                    owner_user_id, steward_user_id, domain_owner_user_id,
                    approver_user_id, organisational_unit,
                    status_id, version_number, is_current_version,
                    approved_at, review_frequency_id, next_review_date,
                    parent_term_id, source_reference, regulatory_reference,
                    used_in_reports, used_in_policies, regulatory_reporting_usage,
                    is_cbt, golden_source_app_id, confidence_level_id,
                    visibility_id, language_id, external_reference,
                    previous_version_id,
                    created_by, updated_by, created_at, updated_at
                FROM glossary_terms
                WHERE term_id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("glossary term not found: {entity_id}")))?;

            // Only send AI-enrichable text fields — exclude IDs, FKs, timestamps, system fields
            let json = serde_json::json!({
                "term_name": row.term_name,
                "definition": row.definition,
                "abbreviation": row.abbreviation,
                "business_context": row.business_context,
                "examples": row.examples,
                "definition_notes": row.definition_notes,
                "counter_examples": row.counter_examples,
                "formula": row.formula,
                "source_reference": row.source_reference,
                "regulatory_reference": row.regulatory_reference,
                "used_in_reports": row.used_in_reports,
                "used_in_policies": row.used_in_policies,
                "regulatory_reporting_usage": row.regulatory_reporting_usage,
                "external_reference": row.external_reference,
            });

            // Track which fields already have values
            let mut existing = Vec::new();
            if !row.term_name.is_empty() {
                existing.push("term_name".to_string());
            }
            if !row.definition.is_empty() {
                existing.push("definition".to_string());
            }
            if row.abbreviation.is_some() {
                existing.push("abbreviation".to_string());
            }
            if row.business_context.is_some() {
                existing.push("business_context".to_string());
            }
            if row.examples.is_some() {
                existing.push("examples".to_string());
            }
            if row.definition_notes.is_some() {
                existing.push("definition_notes".to_string());
            }
            if row.counter_examples.is_some() {
                existing.push("counter_examples".to_string());
            }
            if row.formula.is_some() {
                existing.push("formula".to_string());
            }
            if row.source_reference.is_some() {
                existing.push("source_reference".to_string());
            }
            if row.regulatory_reference.is_some() {
                existing.push("regulatory_reference".to_string());
            }
            if row.used_in_reports.is_some() {
                existing.push("used_in_reports".to_string());
            }
            if row.used_in_policies.is_some() {
                existing.push("used_in_policies".to_string());
            }
            if row.regulatory_reporting_usage.is_some() {
                existing.push("regulatory_reporting_usage".to_string());
            }
            if row.external_reference.is_some() {
                existing.push("external_reference".to_string());
            }
            // Track existing lookup field values so AI skips them
            if row.domain_id.is_some() {
                existing.push("domain".to_string());
            }
            if row.category_id.is_some() {
                existing.push("category".to_string());
            }
            if row.classification_id.is_some() {
                existing.push("data_classification".to_string());
            }
            if row.term_type_id.is_some() {
                existing.push("term_type".to_string());
            }
            if row.unit_of_measure_id.is_some() {
                existing.push("unit_of_measure".to_string());
            }

            Ok((json, existing))
        }
        "data_element" => {
            let row = sqlx::query_as::<_, crate::domain::data_dictionary::DataElement>(
                r#"
                SELECT
                    element_id, element_name, element_code, description,
                    business_definition, business_rules, data_type,
                    max_length, numeric_precision, numeric_scale,
                    format_pattern, allowed_values, default_value,
                    is_nullable, is_cde, cde_rationale, cde_designated_at,
                    glossary_term_id, domain_id, classification_id,
                    status_id, owner_user_id,
                    steward_user_id, approver_user_id, organisational_unit,
                    review_frequency_id, next_review_date, approved_at,
                    is_pii, version_number, is_current_version,
                    previous_version_id,
                    created_by, updated_by,
                    created_at, updated_at
                FROM data_elements
                WHERE element_id = $1 AND deleted_at IS NULL
                "#,
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("data element not found: {entity_id}")))?;

            // Only send AI-enrichable text fields — exclude IDs, FKs, timestamps, system fields
            let json = serde_json::json!({
                "element_name": row.element_name,
                "element_code": row.element_code,
                "description": row.description,
                "business_definition": row.business_definition,
                "business_rules": row.business_rules,
                "data_type": row.data_type,
                "format_pattern": row.format_pattern,
                "default_value": row.default_value,
            });

            let mut existing = Vec::new();
            if !row.element_name.is_empty() {
                existing.push("element_name".to_string());
            }
            if !row.description.is_empty() {
                existing.push("description".to_string());
            }
            if row.business_definition.is_some() {
                existing.push("business_definition".to_string());
            }
            if row.business_rules.is_some() {
                existing.push("business_rules".to_string());
            }
            if row.format_pattern.is_some() {
                existing.push("format_pattern".to_string());
            }
            // Track existing lookup field values so AI skips them
            if row.domain_id.is_some() {
                existing.push("domain".to_string());
            }
            if row.classification_id.is_some() {
                existing.push("data_classification".to_string());
            }

            Ok((json, existing))
        }
        "application" => {
            let row = sqlx::query_as::<_, crate::domain::applications::Application>(
                "SELECT * FROM applications WHERE application_id = $1 AND deleted_at IS NULL",
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("application not found: {entity_id}")))?;

            let json = serde_json::json!({
                "application_name": row.application_name,
                "application_code": row.application_code,
                "description": row.description,
                "abbreviation": row.abbreviation,
                "business_capability": row.business_capability,
                "user_base": row.user_base,
                "regulatory_scope": row.regulatory_scope,
                "license_type": row.license_type,
                "vendor": row.vendor,
                "vendor_product_name": row.vendor_product_name,
                "deployment_type": row.deployment_type,
            });

            let mut existing = Vec::new();
            if !row.application_name.is_empty() {
                existing.push("application_name".to_string());
            }
            if !row.description.is_empty() {
                existing.push("description".to_string());
            }
            if row.abbreviation.is_some() {
                existing.push("abbreviation".to_string());
            }
            if row.business_capability.is_some() {
                existing.push("business_capability".to_string());
            }
            if row.user_base.is_some() {
                existing.push("user_base".to_string());
            }
            if row.regulatory_scope.is_some() {
                existing.push("regulatory_scope".to_string());
            }
            if row.license_type.is_some() {
                existing.push("license_type".to_string());
            }
            if row.data_classification_id.is_some() {
                existing.push("data_classification".to_string());
            }

            Ok((json, existing))
        }
        _ => Err(AppError::Validation(format!(
            "unsupported entity type for AI enrichment: {entity_type} — supported types: glossary_term, data_element, application"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Helper: apply accepted suggestion value to the entity
// ---------------------------------------------------------------------------

async fn apply_suggestion_to_entity(
    pool: &sqlx::PgPool,
    entity_type: &str,
    entity_id: Uuid,
    field_name: &str,
    value: &str,
    user_id: Uuid,
) -> Result<(), AppError> {
    // SEC-001: All SQL strings are compile-time constants — no format!() with user input.
    // Each allowed column gets its own static query to prevent SQL injection.
    match entity_type {
        "glossary_term" => {
            match field_name {
                // --- Text columns: direct column update ---
                "definition" => {
                    sqlx::query("UPDATE glossary_terms SET definition = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "business_context" => {
                    sqlx::query("UPDATE glossary_terms SET business_context = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "examples" => {
                    sqlx::query("UPDATE glossary_terms SET examples = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "abbreviation" => {
                    sqlx::query("UPDATE glossary_terms SET abbreviation = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "source_reference" => {
                    sqlx::query("UPDATE glossary_terms SET source_reference = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "regulatory_reference" => {
                    sqlx::query("UPDATE glossary_terms SET regulatory_reference = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "definition_notes" => {
                    sqlx::query("UPDATE glossary_terms SET definition_notes = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "counter_examples" => {
                    sqlx::query("UPDATE glossary_terms SET counter_examples = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "formula" => {
                    sqlx::query("UPDATE glossary_terms SET formula = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "used_in_reports" => {
                    sqlx::query("UPDATE glossary_terms SET used_in_reports = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "used_in_policies" => {
                    sqlx::query("UPDATE glossary_terms SET used_in_policies = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "regulatory_reporting_usage" => {
                    sqlx::query("UPDATE glossary_terms SET regulatory_reporting_usage = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "external_reference" => {
                    sqlx::query("UPDATE glossary_terms SET external_reference = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "organisational_unit" => {
                    sqlx::query("UPDATE glossary_terms SET organisational_unit = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                // --- Lookup columns: UUID-first resolution (CODING_STANDARDS Section 15.6) ---
                // AI now returns UUIDs directly from the lookup list embedded in the prompt.
                // Falls back to ILIKE name match for backward compatibility.
                "domain" => {
                    if let Some(id) = crate::db::resolve_lookup(
                        pool, value,
                        "SELECT domain_id FROM glossary_domains WHERE domain_name ILIKE $1 LIMIT 1",
                    ).await {
                        sqlx::query("UPDATE glossary_terms SET domain_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                            .bind(id).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "category" => {
                    if let Some(id) = crate::db::resolve_lookup(
                        pool, value,
                        "SELECT category_id FROM glossary_categories WHERE category_name ILIKE $1 LIMIT 1",
                    ).await {
                        sqlx::query("UPDATE glossary_terms SET category_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                            .bind(id).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "data_classification" => {
                    if let Some(id) = crate::db::resolve_lookup(
                        pool, value,
                        "SELECT classification_id FROM data_classifications WHERE classification_name ILIKE $1 LIMIT 1",
                    ).await {
                        sqlx::query("UPDATE glossary_terms SET classification_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                            .bind(id).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "term_type" => {
                    if let Some(id) = crate::db::resolve_lookup(
                        pool, value,
                        "SELECT term_type_id FROM glossary_term_types WHERE type_name ILIKE $1 LIMIT 1",
                    ).await {
                        sqlx::query("UPDATE glossary_terms SET term_type_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                            .bind(id).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "unit_of_measure" => {
                    if let Some(id) = crate::db::resolve_lookup(
                        pool, value,
                        "SELECT unit_id FROM glossary_units_of_measure WHERE unit_name ILIKE $1 LIMIT 1",
                    ).await {
                        sqlx::query("UPDATE glossary_terms SET unit_of_measure_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE term_id = $3 AND deleted_at IS NULL")
                            .bind(id).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                // parent_term, child_terms, related_terms are NOT AI-suggestible.
                // They must be selected from existing terms by the user.

                // --- Junction columns: parse comma-separated, resolve each, insert junction rows ---
                "regulatory_tags" => {
                    for tag_name in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        if let Some(id) = crate::db::resolve_lookup(
                            pool,
                            tag_name,
                            "SELECT tag_id FROM glossary_regulatory_tags WHERE tag_name ILIKE $1 OR tag_code ILIKE $1 LIMIT 1",
                        ).await {
                            sqlx::query("INSERT INTO glossary_term_regulatory_tags (term_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                                .bind(entity_id).bind(id).execute(pool).await?;
                        }
                    }
                }
                "subject_areas" => {
                    for area_name in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        if let Some(id) = crate::db::resolve_lookup(
                            pool,
                            area_name,
                            "SELECT subject_area_id FROM glossary_subject_areas WHERE area_name ILIKE $1 OR area_code ILIKE $1 LIMIT 1",
                        ).await {
                            sqlx::query("INSERT INTO glossary_term_subject_areas (term_id, subject_area_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                                .bind(entity_id).bind(id).execute(pool).await?;
                        }
                    }
                }
                "tags" => {
                    for tag_name in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        let tag_id = sqlx::query_scalar::<_, Uuid>(
                            "INSERT INTO glossary_tags (tag_name) VALUES ($1) ON CONFLICT (tag_name) DO UPDATE SET tag_name = $1 RETURNING tag_id"
                        ).bind(tag_name).fetch_one(pool).await?;
                        sqlx::query("INSERT INTO glossary_term_tags (term_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                            .bind(entity_id).bind(tag_id).execute(pool).await?;
                    }
                }
                "synonyms" => {
                    // Insert each synonym as an alias in glossary_term_aliases
                    for synonym in value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        sqlx::query(
                            "INSERT INTO glossary_term_aliases (term_id, alias_name, alias_type) VALUES ($1, $2, 'SYNONYM') ON CONFLICT DO NOTHING"
                        ).bind(entity_id).bind(synonym).execute(pool).await?;
                    }
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "cannot apply suggestion to field '{field_name}' on glossary_term"
                    )));
                }
            }
        }
        "data_element" => {
            // SEC-001: Each allowed column gets its own static SQL query.
            match field_name {
                "description" => {
                    sqlx::query("UPDATE data_elements SET description = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "business_definition" => {
                    sqlx::query("UPDATE data_elements SET business_definition = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "business_rules" => {
                    sqlx::query("UPDATE data_elements SET business_rules = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "format_pattern" => {
                    sqlx::query("UPDATE data_elements SET format_pattern = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "default_value" => {
                    sqlx::query("UPDATE data_elements SET default_value = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "data_type" => {
                    sqlx::query("UPDATE data_elements SET data_type = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "max_length" => {
                    if let Ok(val) = value.parse::<i32>() {
                        sqlx::query("UPDATE data_elements SET max_length = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                            .bind(val).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "numeric_precision" => {
                    if let Ok(val) = value.parse::<i32>() {
                        sqlx::query("UPDATE data_elements SET numeric_precision = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                            .bind(val).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "numeric_scale" => {
                    if let Ok(val) = value.parse::<i32>() {
                        sqlx::query("UPDATE data_elements SET numeric_scale = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                            .bind(val).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                // Lookup columns: UUID-first resolution (CODING_STANDARDS Section 15.6)
                "domain" => {
                    if let Ok(uuid) = Uuid::parse_str(value) {
                        sqlx::query("UPDATE data_elements SET domain_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                            .bind(uuid).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                "data_classification" => {
                    if let Ok(uuid) = Uuid::parse_str(value) {
                        sqlx::query("UPDATE data_elements SET classification_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE element_id = $3 AND deleted_at IS NULL")
                            .bind(uuid).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "cannot apply suggestion to field '{field_name}' on data_element"
                    )));
                }
            }
        }
        "application" => {
            match field_name {
                "description" => {
                    sqlx::query("UPDATE applications SET description = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "abbreviation" => {
                    sqlx::query("UPDATE applications SET abbreviation = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "business_capability" => {
                    sqlx::query("UPDATE applications SET business_capability = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "user_base" => {
                    sqlx::query("UPDATE applications SET user_base = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "regulatory_scope" => {
                    sqlx::query("UPDATE applications SET regulatory_scope = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "license_type" => {
                    sqlx::query("UPDATE applications SET license_type = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "vendor" => {
                    sqlx::query("UPDATE applications SET vendor = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "vendor_product_name" => {
                    sqlx::query("UPDATE applications SET vendor_product_name = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                "deployment_type" => {
                    sqlx::query("UPDATE applications SET deployment_type = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                        .bind(value).bind(user_id).bind(entity_id).execute(pool).await?;
                }
                // Lookup field: data_classification (UUID from embedded prompt)
                "data_classification" => {
                    if let Ok(uuid) = Uuid::parse_str(value) {
                        sqlx::query("UPDATE applications SET data_classification_id = $1, updated_by = $2, updated_at = CURRENT_TIMESTAMP WHERE application_id = $3 AND deleted_at IS NULL")
                            .bind(uuid).bind(user_id).bind(entity_id).execute(pool).await?;
                    }
                }
                _ => {
                    return Err(AppError::Validation(format!(
                        "cannot apply suggestion to field '{field_name}' on application"
                    )));
                }
            }
        }
        _ => {
            return Err(AppError::Validation(format!(
                "cannot apply suggestion to unsupported entity type: {entity_type}"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// enrich — POST /api/v1/ai/enrich
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/enrich",
    request_body = AiEnrichRequest,
    responses(
        (status = 200, description = "AI enrichment suggestions", body = AiEnrichResponse),
        (status = 404, description = "Entity not found"),
        (status = 502, description = "AI service error")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn enrich(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<AiEnrichRequest>,
) -> AppResult<Json<AiEnrichResponse>> {
    // SEC-025: Input length validation
    if body.entity_type.len() > 64 {
        return Err(AppError::Validation(
            "entity_type exceeds maximum length".into(),
        ));
    }

    // Fetch entity data
    let (entity_data, existing_fields) =
        fetch_entity_data(&state.pool, &body.entity_type, body.entity_id).await?;

    // Fetch lookup tables for the AI prompt (CODING_STANDARDS Section 15.6)
    let lookups = match body.entity_type.as_str() {
        "glossary_term" => fetch_glossary_lookups(&state.pool).await?,
        "data_element" => {
            // Embed domains and data_classifications for the lookup-in-prompt pattern (Section 15.6)
            let domains = sqlx::query_as::<_, IdName>(
                "SELECT domain_id AS id, domain_name AS name FROM glossary_domains ORDER BY domain_name",
            )
            .fetch_all(&state.pool)
            .await?;
            let classifications = sqlx::query_as::<_, IdName>(
                "SELECT classification_id AS id, classification_name AS name FROM data_classifications ORDER BY display_order ASC",
            )
            .fetch_all(&state.pool)
            .await?;
            serde_json::json!({
                "domain": domains.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
                "data_classification": classifications.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
            })
        }
        "application" => {
            // Embed data_classifications for the lookup-in-prompt pattern (Section 15.6)
            let classifications = sqlx::query_as::<_, IdName>(
                "SELECT classification_id AS id, classification_name AS name FROM data_classifications ORDER BY display_order ASC",
            )
            .fetch_all(&state.pool)
            .await?;
            serde_json::json!({
                "data_classification": classifications.iter().map(|r| serde_json::json!({"id": r.id, "name": r.name})).collect::<Vec<_>>(),
            })
        }
        _ => serde_json::json!({}),
    };

    // Call AI enrichment service
    let result = crate::ai::enrich_entity(
        &state.config.ai,
        &body.entity_type,
        entity_data,
        existing_fields.clone(),
        lookups,
    )
    .await?;

    // Filter suggestions — backend is the authoritative gate, not the AI prompt.
    // Drop: disallowed fields, empty values, AND fields that already have values.
    let filtered_suggestions: Vec<_> = result
        .suggestions
        .iter()
        .filter(|s| {
            // Drop disallowed field names
            if s.field_name.ends_with("_id")
                || s.field_name.ends_with("_at")
                || s.field_name.ends_with("_by")
                || matches!(
                    s.field_name.as_str(),
                    "status_id"
                        | "version_number"
                        | "is_current_version"
                        | "is_cbt"
                        | "is_cde"
                        | "is_cba"
                        | "is_nullable"
                        | "is_active"
                        | "is_critical"
                        | "parent_term"
                        | "child_terms"
                        | "related_terms"
                        | "golden_source"
                        | "golden_source_app_id"
                )
            {
                return false;
            }
            // Drop empty suggestions
            if s.suggested_value.is_empty() {
                return false;
            }
            // Drop suggestions for fields that already have values
            // (AI may ignore the prompt instruction to skip populated fields)
            if existing_fields.contains(&s.field_name) {
                tracing::debug!(
                    field = %s.field_name,
                    "dropping AI suggestion for already-populated field"
                );
                return false;
            }
            true
        })
        .collect();

    // Store suggestions in the database
    let mut suggestion_responses = Vec::new();

    for raw in &filtered_suggestions {
        let row = sqlx::query_as::<_, AiSuggestion>(
            r#"
            INSERT INTO ai_suggestions (
                entity_type, entity_id, field_name, suggested_value,
                confidence, rationale, source, model, status
            )
            VALUES ($1, $2, $3, $4, $5::FLOAT8::NUMERIC(3,2), $6, $7, $8, 'PENDING')
            RETURNING
                suggestion_id, entity_type, entity_id, field_name,
                suggested_value, confidence::FLOAT8 AS confidence,
                rationale, source, model, status,
                accepted_by, accepted_at, created_at
            "#,
        )
        .bind(&body.entity_type)
        .bind(body.entity_id)
        .bind(&raw.field_name)
        .bind(&raw.suggested_value)
        .bind(raw.confidence)
        .bind(&raw.rationale)
        .bind(&result.provider)
        .bind(&result.model)
        .fetch_one(&state.pool)
        .await?;

        suggestion_responses.push(AiSuggestionResponse {
            suggestion_id: row.suggestion_id,
            field_name: row.field_name,
            suggested_value: row.suggested_value,
            confidence: row.confidence.unwrap_or(0.0),
            rationale: row.rationale.unwrap_or_default(),
            source: row.source,
            model: row.model,
            status: row.status,
            created_at: row.created_at,
        });
    }

    // Log the enrichment action
    tracing::info!(
        entity_type = %body.entity_type,
        entity_id = %body.entity_id,
        user_id = %claims.sub,
        provider = %result.provider,
        suggestion_count = suggestion_responses.len(),
        "AI enrichment completed"
    );

    Ok(Json(AiEnrichResponse {
        entity_type: body.entity_type,
        entity_id: body.entity_id,
        suggestions: suggestion_responses,
        provider: result.provider,
        model: result.model,
    }))
}

// ---------------------------------------------------------------------------
// list_suggestions — GET /api/v1/ai/suggestions/:entity_type/:entity_id
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/ai/suggestions/{entity_type}/{entity_id}",
    params(
        ("entity_type" = String, Path, description = "Entity type (glossary_term, data_element)"),
        ("entity_id" = Uuid, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "List of suggestions for entity", body = Vec<AiSuggestionResponse>)
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn list_suggestions(
    State(state): State<AppState>,
    Path((entity_type, entity_id)): Path<(String, Uuid)>,
) -> AppResult<Json<Vec<AiSuggestionResponse>>> {
    let rows = sqlx::query_as::<_, AiSuggestion>(
        r#"
        SELECT
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        FROM ai_suggestions
        WHERE entity_type = $1 AND entity_id = $2
        ORDER BY created_at DESC
        "#,
    )
    .bind(&entity_type)
    .bind(entity_id)
    .fetch_all(&state.pool)
    .await?;

    let responses: Vec<AiSuggestionResponse> = rows
        .into_iter()
        .map(|row| AiSuggestionResponse {
            suggestion_id: row.suggestion_id,
            field_name: row.field_name,
            suggested_value: row.suggested_value,
            confidence: row.confidence.unwrap_or(0.0),
            rationale: row.rationale.unwrap_or_default(),
            source: row.source,
            model: row.model,
            status: row.status,
            created_at: row.created_at,
        })
        .collect();

    Ok(Json(responses))
}

// ---------------------------------------------------------------------------
// accept_suggestion — POST /api/v1/ai/suggestions/:suggestion_id/accept
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/suggestions/{suggestion_id}/accept",
    params(("suggestion_id" = Uuid, Path, description = "Suggestion ID")),
    request_body = AcceptSuggestionRequest,
    responses(
        (status = 200, description = "Suggestion accepted and applied", body = AiSuggestionResponse),
        (status = 404, description = "Suggestion not found")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn accept_suggestion(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
    Json(body): Json<AcceptSuggestionRequest>,
) -> AppResult<Json<AiSuggestionResponse>> {
    // SEC-025: Input length validation
    if let Some(ref val) = body.modified_value
        && val.len() > 4000
    {
        return Err(AppError::Validation(
            "modified_value exceeds 4000 characters".into(),
        ));
    }

    // Fetch the suggestion
    let suggestion = sqlx::query_as::<_, AiSuggestion>(
        r#"
        SELECT
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        FROM ai_suggestions
        WHERE suggestion_id = $1
        "#,
    )
    .bind(suggestion_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("suggestion not found: {suggestion_id}")))?;

    if suggestion.status != "PENDING" {
        return Err(AppError::Validation(format!(
            "suggestion is already {}, cannot accept",
            suggestion.status
        )));
    }

    // Determine the value to apply
    let final_value = body
        .modified_value
        .as_deref()
        .unwrap_or(&suggestion.suggested_value);
    let new_status = if body.modified_value.is_some() {
        "MODIFIED"
    } else {
        "ACCEPTED"
    };

    // Apply the value to the entity
    apply_suggestion_to_entity(
        &state.pool,
        &suggestion.entity_type,
        suggestion.entity_id,
        &suggestion.field_name,
        final_value,
        claims.sub,
    )
    .await?;

    // Update the suggestion status
    let updated = sqlx::query_as::<_, AiSuggestion>(
        r#"
        UPDATE ai_suggestions
        SET status = $1,
            suggested_value = $2,
            accepted_by = $3,
            accepted_at = CURRENT_TIMESTAMP
        WHERE suggestion_id = $4
        RETURNING
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        "#,
    )
    .bind(new_status)
    .bind(final_value)
    .bind(claims.sub)
    .bind(suggestion_id)
    .fetch_one(&state.pool)
    .await?;

    tracing::info!(
        suggestion_id = %suggestion_id,
        status = new_status,
        user_id = %claims.sub,
        "AI suggestion accepted"
    );

    Ok(Json(AiSuggestionResponse {
        suggestion_id: updated.suggestion_id,
        field_name: updated.field_name,
        suggested_value: updated.suggested_value,
        confidence: updated.confidence.unwrap_or(0.0),
        rationale: updated.rationale.unwrap_or_default(),
        source: updated.source,
        model: updated.model,
        status: updated.status,
        created_at: updated.created_at,
    }))
}

// ---------------------------------------------------------------------------
// reject_suggestion — POST /api/v1/ai/suggestions/:suggestion_id/reject
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/suggestions/{suggestion_id}/reject",
    params(("suggestion_id" = Uuid, Path, description = "Suggestion ID")),
    responses(
        (status = 200, description = "Suggestion rejected", body = AiSuggestionResponse),
        (status = 404, description = "Suggestion not found")
    ),
    security(("bearer_auth" = [])),
    tag = "glossary"
)]
pub async fn reject_suggestion(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
) -> AppResult<Json<AiSuggestionResponse>> {
    // Verify and update in one query
    let updated = sqlx::query_as::<_, AiSuggestion>(
        r#"
        UPDATE ai_suggestions
        SET status = 'REJECTED',
            accepted_by = $1,
            accepted_at = CURRENT_TIMESTAMP
        WHERE suggestion_id = $2 AND status = 'PENDING'
        RETURNING
            suggestion_id, entity_type, entity_id, field_name,
            suggested_value, confidence::FLOAT8 AS confidence,
            rationale, source, model, status,
            accepted_by, accepted_at, created_at
        "#,
    )
    .bind(claims.sub)
    .bind(suggestion_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("pending suggestion not found: {suggestion_id}")))?;

    tracing::info!(
        suggestion_id = %suggestion_id,
        user_id = %claims.sub,
        "AI suggestion rejected"
    );

    Ok(Json(AiSuggestionResponse {
        suggestion_id: updated.suggestion_id,
        field_name: updated.field_name,
        suggested_value: updated.suggested_value,
        confidence: updated.confidence.unwrap_or(0.0),
        rationale: updated.rationale.unwrap_or_default(),
        source: updated.source,
        model: updated.model,
        status: updated.status,
        created_at: updated.created_at,
    }))
}

// ---------------------------------------------------------------------------
// submit_feedback — POST /api/v1/ai/suggestions/:suggestion_id/feedback
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/ai/suggestions/{suggestion_id}/feedback",
    params(("suggestion_id" = Uuid, Path, description = "Suggestion ID")),
    request_body = FeedbackRequest,
    responses(
        (status = 200, description = "Feedback recorded", body = FeedbackResponse),
        (status = 404, description = "Suggestion not found")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn submit_feedback(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(suggestion_id): Path<Uuid>,
    Json(body): Json<FeedbackRequest>,
) -> AppResult<Json<FeedbackResponse>> {
    // Validate rating if provided
    if let Some(rating) = body.rating
        && !(1..=5).contains(&rating)
    {
        return Err(AppError::Validation(
            "rating must be between 1 and 5".into(),
        ));
    }

    // SEC-025: Input length validation
    if let Some(ref text) = body.feedback_text
        && text.len() > 4000
    {
        return Err(AppError::Validation(
            "feedback_text exceeds 4000 characters".into(),
        ));
    }

    // Verify suggestion exists
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ai_suggestions WHERE suggestion_id = $1)",
    )
    .bind(suggestion_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "suggestion not found: {suggestion_id}"
        )));
    }

    // Insert feedback
    let feedback_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_feedback (suggestion_id, user_id, rating, feedback_text)
        VALUES ($1, $2, $3, $4)
        RETURNING feedback_id
        "#,
    )
    .bind(suggestion_id)
    .bind(claims.sub)
    .bind(body.rating)
    .bind(body.feedback_text.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(FeedbackResponse {
        feedback_id,
        suggestion_id,
        message: "feedback recorded successfully".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// suggest_quality_rules — POST /api/v1/ai/suggest-quality-rules
// ---------------------------------------------------------------------------

/// Ask AI to suggest quality rules for a data element across the 6 quality dimensions.
/// Returns suggestions directly (not stored in ai_suggestions) for the user to accept/reject.
#[utoipa::path(
    post,
    path = "/api/v1/ai/suggest-quality-rules",
    request_body = AiSuggestRulesRequest,
    responses(
        (status = 200, description = "AI-suggested quality rules", body = AiSuggestRulesResponse),
        (status = 404, description = "Data element not found"),
        (status = 502, description = "AI service error")
    ),
    security(("bearer_auth" = [])),
    tag = "ai"
)]
pub async fn suggest_quality_rules(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<AiSuggestRulesRequest>,
) -> AppResult<Json<AiSuggestRulesResponse>> {
    // Fetch the data element
    let row = sqlx::query_as::<_, crate::domain::data_dictionary::DataElement>(
        r#"
        SELECT
            element_id, element_name, element_code, description,
            business_definition, business_rules, data_type,
            max_length, numeric_precision, numeric_scale,
            format_pattern, allowed_values, default_value,
            is_nullable, is_cde, cde_rationale, cde_designated_at,
            glossary_term_id, domain_id, classification_id,
            status_id, owner_user_id,
            steward_user_id, approver_user_id, organisational_unit,
            review_frequency_id, next_review_date, approved_at,
            is_pii, version_number, is_current_version,
            previous_version_id,
            created_by, updated_by,
            created_at, updated_at
        FROM data_elements
        WHERE element_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(body.element_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("data element not found: {}", body.element_id)))?;

    // Build prompt for quality rule suggestions
    let element_name = &row.element_name;
    let data_type = row.data_type.as_deref().unwrap_or("unknown");
    let description = &row.description;
    let business_definition = row.business_definition.as_deref().unwrap_or("not specified");

    // Sanitize inputs for prompt injection safety (SEC-013)
    let safe_name = crate::ai::sanitize_for_prompt(element_name);
    let safe_type = crate::ai::sanitize_for_prompt(data_type);
    let safe_desc = crate::ai::sanitize_for_prompt(description);
    let safe_biz_def = crate::ai::sanitize_for_prompt(business_definition);

    let prompt = format!(
        r#"You are a data quality expert for financial institutions. Given the following data element, suggest quality rules for each applicable quality dimension.

Data element:
- Name: {safe_name}
- Data Type: {safe_type}
- Description: {safe_desc}
- Business Definition: {safe_biz_def}

Quality Dimensions:
1. COMPLETENESS — data values are present where expected
2. UNIQUENESS — no unintended duplicates
3. VALIDITY — values conform to defined rules/formats
4. ACCURACY — values correctly represent real-world truth
5. TIMELINESS — data is available when needed
6. CONSISTENCY — same data in different places agrees

For each applicable dimension, suggest a quality rule. Not all dimensions apply to every element. Only suggest rules that make practical sense.

Return ONLY a JSON array. Each rule must have:
- dimension: one of COMPLETENESS, UNIQUENESS, VALIDITY, ACCURACY, TIMELINESS, CONSISTENCY
- rule_name: short descriptive name
- description: what the rule checks
- comparison_type: one of NOT_NULL, UNIQUE, GREATER_THAN, LESS_THAN, BETWEEN, EQUAL, NOT_EQUAL, REGEX, IN_LIST, CUSTOM_SQL (or null if not applicable)
- comparison_value: the value to compare against (or null)
- threshold_percentage: pass rate threshold (0-100)
- severity: CRITICAL, HIGH, MEDIUM, or LOW
- rationale: why this rule, citing standards where applicable
- confidence: 0.0-1.0

Example:
[
  {{
    "dimension": "COMPLETENESS",
    "rule_name": "Interest Income Not Null",
    "description": "Interest Income value must be present",
    "comparison_type": "NOT_NULL",
    "comparison_value": null,
    "threshold_percentage": 100.0,
    "severity": "CRITICAL",
    "rationale": "Per BCBS 239, all financial metrics must have complete data",
    "confidence": 0.95
  }}
]"#,
    );

    // Verify at least one AI provider is configured
    if state.config.ai.anthropic_api_key.is_none() && state.config.ai.openai_api_key.is_none() {
        return Err(AppError::AiService(
            "no AI provider configured — set ANTHROPIC_API_KEY or OPENAI_API_KEY".into(),
        ));
    }

    // Call AI (Claude primary, OpenAI fallback)
    let (text, model, provider) = if state.config.ai.anthropic_api_key.is_some() {
        match crate::ai::call_claude_public(&state.config.ai, &prompt).await {
            Ok((text, model)) => (text, model, "CLAUDE".to_string()),
            Err(e) => {
                tracing::warn!(error = %e, "Claude API failed for quality rules, attempting OpenAI fallback");
                if state.config.ai.openai_api_key.is_some() {
                    let (text, model) =
                        crate::ai::call_openai_public(&state.config.ai, &prompt).await?;
                    (text, model, "OPENAI".to_string())
                } else {
                    return Err(e);
                }
            }
        }
    } else {
        let (text, model) = crate::ai::call_openai_public(&state.config.ai, &prompt).await?;
        (text, model, "OPENAI".to_string())
    };

    // Parse the AI response
    let suggestions = parse_rule_suggestions(&text)?;

    tracing::info!(
        element_id = %body.element_id,
        user_id = %claims.sub,
        provider = %provider,
        suggestion_count = suggestions.len(),
        "AI quality rule suggestions generated"
    );

    Ok(Json(AiSuggestRulesResponse {
        element_id: body.element_id,
        element_name: row.element_name.clone(),
        suggestions,
        provider,
        model,
    }))
}

/// Parse AI response text into a Vec<AiRuleSuggestion>.
/// Similar to parse_suggestions but for the quality rules schema.
fn parse_rule_suggestions(text: &str) -> Result<Vec<AiRuleSuggestion>, AppError> {
    let trimmed = text.trim();
    let json_str = if trimmed.starts_with("```") {
        let without_opening = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_opening
            .strip_suffix("```")
            .unwrap_or(without_opening)
            .trim()
    } else {
        trimmed
    };

    let json_to_parse = if json_str.starts_with('[') {
        json_str.to_string()
    } else if let Some(start) = json_str.find('[') {
        if let Some(end) = json_str.rfind(']') {
            json_str[start..=end].to_string()
        } else {
            return Err(AppError::AiService(
                "AI response does not contain a valid JSON array".into(),
            ));
        }
    } else {
        return Err(AppError::AiService(
            "AI response does not contain a JSON array".into(),
        ));
    };

    // Parse as raw JSON first, then convert — handles AI returning numbers where strings expected
    let raw_values: Vec<serde_json::Value> = serde_json::from_str(&json_to_parse)
        .map_err(|e| AppError::AiService(format!("failed to parse AI rule suggestions: {e}")))?;

    let suggestions: Vec<AiRuleSuggestion> = raw_values
        .into_iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            Some(AiRuleSuggestion {
                dimension: obj.get("dimension")?.as_str()?.to_string(),
                rule_name: obj.get("rule_name")?.as_str()?.to_string(),
                description: obj.get("description")?.as_str()?.to_string(),
                comparison_type: obj.get("comparison_type").and_then(|v| {
                    if v.is_null() { None } else { Some(v.as_str().unwrap_or("").to_string()) }
                }).filter(|s| !s.is_empty()),
                comparison_value: obj.get("comparison_value").and_then(|v| {
                    if v.is_null() { None }
                    else if v.is_string() { Some(v.as_str().unwrap().to_string()) }
                    else { Some(v.to_string()) } // handles numbers, booleans
                }),
                threshold_percentage: obj.get("threshold_percentage").and_then(|v| v.as_f64()).unwrap_or(100.0),
                severity: obj.get("severity").and_then(|v| v.as_str()).unwrap_or("MEDIUM").to_string(),
                rationale: obj.get("rationale").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                confidence: obj.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5),
            })
        })
        .collect();

    // Validate and clean each suggestion
    let valid_dimensions = [
        "COMPLETENESS",
        "UNIQUENESS",
        "VALIDITY",
        "ACCURACY",
        "TIMELINESS",
        "CONSISTENCY",
    ];
    let valid_severities = ["LOW", "MEDIUM", "HIGH", "CRITICAL"];

    let suggestions = suggestions
        .into_iter()
        .filter_map(|mut s| {
            // Validate dimension
            if !valid_dimensions.contains(&s.dimension.as_str()) {
                tracing::warn!(dimension = %s.dimension, "dropping rule suggestion with invalid dimension");
                return None;
            }
            // Validate severity
            if !valid_severities.contains(&s.severity.as_str()) {
                s.severity = "MEDIUM".to_string();
            }
            // Clamp confidence and threshold
            s.confidence = s.confidence.clamp(0.0, 1.0);
            s.threshold_percentage = s.threshold_percentage.clamp(0.0, 100.0);
            // Drop suggestions with empty names
            if s.rule_name.trim().is_empty() {
                return None;
            }
            Some(s)
        })
        .collect();

    Ok(suggestions)
}
