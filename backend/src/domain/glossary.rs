use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Paginated response wrapper (generic, reusable across all domains)
// ---------------------------------------------------------------------------

/// Paginated API response wrapper.
/// Since utoipa does not support generic schemas directly, each domain
/// defines a concrete alias. See `PaginatedGlossaryTerms` below.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedGlossaryTerms {
    pub data: Vec<GlossaryTermListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Full glossary term (single-record detail view — all 45 columns)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryTerm {
    // Section 1: Core Identity
    pub term_id: Uuid,
    pub term_name: String,
    pub term_code: Option<String>,
    pub definition: String,
    pub abbreviation: Option<String>,

    // Section 2: Definition & Semantics
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub definition_notes: Option<String>,
    pub counter_examples: Option<String>,
    pub formula: Option<String>,
    pub unit_of_measure_id: Option<Uuid>,

    // Section 3: Classification
    pub term_type_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,

    // Section 4: Ownership
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub domain_owner_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,

    // Section 5: Lifecycle
    pub status_id: Uuid,
    pub version_number: i32,
    pub is_current_version: bool,
    pub approved_at: Option<DateTime<Utc>>,
    pub review_frequency_id: Option<Uuid>,
    pub next_review_date: Option<NaiveDate>,

    // Section 6: Relationships
    pub parent_term_id: Option<Uuid>,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,

    // Section 7: Usage & Context
    pub used_in_reports: Option<String>,
    pub used_in_policies: Option<String>,
    pub regulatory_reporting_usage: Option<String>,

    // Section 8: Quality
    pub is_cbt: bool,
    pub golden_source: Option<String>,
    pub golden_source_app_id: Option<Uuid>,
    pub confidence_level_id: Option<Uuid>,

    // Section 9: Discoverability
    pub visibility_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
    pub external_reference: Option<String>,

    // Versioning
    pub previous_version_id: Option<Uuid>,

    // Audit
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Detail view — flat struct with all columns, resolved names, and junction data
// (ADR-0006 Pattern 1: No #[serde(flatten)], explicit fields only)
// ---------------------------------------------------------------------------

/// Internal row type for the single JOIN query that fetches entity columns
/// plus all resolved FK lookup names. Used by the `get_term` handler.
#[derive(Debug, Clone, FromRow)]
pub struct GlossaryTermDetailRow {
    // === Entity columns ===
    pub term_id: Uuid,
    pub term_name: String,
    pub term_code: Option<String>,
    pub definition: String,
    pub abbreviation: Option<String>,
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub definition_notes: Option<String>,
    pub counter_examples: Option<String>,
    pub formula: Option<String>,
    pub unit_of_measure_id: Option<Uuid>,
    pub term_type_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub domain_owner_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,
    pub status_id: Uuid,
    pub version_number: i32,
    pub is_current_version: bool,
    pub approved_at: Option<DateTime<Utc>>,
    pub review_frequency_id: Option<Uuid>,
    pub next_review_date: Option<NaiveDate>,
    pub parent_term_id: Option<Uuid>,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,
    pub used_in_reports: Option<String>,
    pub used_in_policies: Option<String>,
    pub regulatory_reporting_usage: Option<String>,
    pub is_cbt: bool,
    pub golden_source: Option<String>,
    pub golden_source_app_id: Option<Uuid>,
    pub confidence_level_id: Option<Uuid>,
    pub visibility_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
    pub external_reference: Option<String>,
    pub previous_version_id: Option<Uuid>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // === Resolved lookup names (from LEFT JOINs) ===
    pub domain_name: Option<String>,
    pub category_name: Option<String>,
    pub term_type_name: Option<String>,
    pub unit_of_measure_name: Option<String>,
    pub classification_name: Option<String>,
    pub review_frequency_name: Option<String>,
    pub confidence_level_name: Option<String>,
    pub visibility_name: Option<String>,
    pub language_name: Option<String>,
    pub parent_term_name: Option<String>,
    pub golden_source_app_name: Option<String>,
    pub owner_name: Option<String>,
    pub steward_name: Option<String>,
    pub domain_owner_name: Option<String>,
    pub approver_name: Option<String>,
    pub status_code: Option<String>,
    pub status_name: Option<String>,
}

/// Complete glossary term detail view with resolved lookup names and junction data.
/// All fields are at the root level — no nesting, no `#[serde(flatten)]` (ADR-0006 Pattern 1).
/// Constructed from a single JOIN query (→ `GlossaryTermDetailRow`) + separate junction queries.
#[derive(Debug, Serialize, ToSchema)]
pub struct GlossaryTermDetail {
    // === Entity columns ===
    pub term_id: Uuid,
    pub term_name: String,
    pub term_code: Option<String>,
    pub definition: String,
    pub abbreviation: Option<String>,
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub definition_notes: Option<String>,
    pub counter_examples: Option<String>,
    pub formula: Option<String>,
    pub unit_of_measure_id: Option<Uuid>,
    pub term_type_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub domain_owner_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,
    pub status_id: Uuid,
    pub version_number: i32,
    pub is_current_version: bool,
    pub approved_at: Option<DateTime<Utc>>,
    pub review_frequency_id: Option<Uuid>,
    pub next_review_date: Option<NaiveDate>,
    pub parent_term_id: Option<Uuid>,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,
    pub used_in_reports: Option<String>,
    pub used_in_policies: Option<String>,
    pub regulatory_reporting_usage: Option<String>,
    pub is_cbt: bool,
    pub golden_source: Option<String>,
    pub golden_source_app_id: Option<Uuid>,
    pub confidence_level_id: Option<Uuid>,
    pub visibility_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
    pub external_reference: Option<String>,
    pub previous_version_id: Option<Uuid>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // === Resolved lookup names (from JOINs) ===
    pub domain_name: Option<String>,
    pub category_name: Option<String>,
    pub term_type_name: Option<String>,
    pub unit_of_measure_name: Option<String>,
    pub classification_name: Option<String>,
    pub review_frequency_name: Option<String>,
    pub confidence_level_name: Option<String>,
    pub visibility_name: Option<String>,
    pub language_name: Option<String>,
    pub parent_term_name: Option<String>,
    pub golden_source_app_name: Option<String>,
    pub owner_name: Option<String>,
    pub steward_name: Option<String>,
    pub domain_owner_name: Option<String>,
    pub approver_name: Option<String>,
    pub status_code: Option<String>,
    pub status_name: Option<String>,
    // === Junction data (from separate queries) ===
    pub regulatory_tags: Vec<GlossaryRegulatoryTagItem>,
    pub subject_areas: Vec<GlossarySubjectAreaItem>,
    pub tags: Vec<GlossaryTagItem>,
    pub linked_processes: Vec<GlossaryLinkedProcess>,
    pub aliases: Vec<GlossaryAliasItem>,
    pub child_terms: Vec<ChildTermRef>,
}

/// Organisational unit for ownership assignment dropdown
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OrganisationalUnit {
    pub unit_id: Uuid,
    pub unit_code: String,
    pub unit_name: String,
    pub description: Option<String>,
}

/// Alias/synonym attached to a term (from glossary_term_aliases)
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossaryAliasItem {
    pub alias_id: Uuid,
    pub alias_name: String,
    pub alias_type: Option<String>,
}

/// Child term reference (terms where parent_term_id = this term)
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ChildTermRef {
    pub term_id: Uuid,
    pub term_name: String,
}

impl GlossaryTermDetail {
    /// Construct from a `GlossaryTermDetailRow` (JOIN query result) and junction data.
    pub fn from_row_and_junctions(
        row: GlossaryTermDetailRow,
        regulatory_tags: Vec<GlossaryRegulatoryTagItem>,
        subject_areas: Vec<GlossarySubjectAreaItem>,
        tags: Vec<GlossaryTagItem>,
        linked_processes: Vec<GlossaryLinkedProcess>,
        aliases: Vec<GlossaryAliasItem>,
        child_terms: Vec<ChildTermRef>,
    ) -> Self {
        Self {
            term_id: row.term_id,
            term_name: row.term_name,
            term_code: row.term_code,
            definition: row.definition,
            abbreviation: row.abbreviation,
            business_context: row.business_context,
            examples: row.examples,
            definition_notes: row.definition_notes,
            counter_examples: row.counter_examples,
            formula: row.formula,
            unit_of_measure_id: row.unit_of_measure_id,
            term_type_id: row.term_type_id,
            domain_id: row.domain_id,
            category_id: row.category_id,
            classification_id: row.classification_id,
            owner_user_id: row.owner_user_id,
            steward_user_id: row.steward_user_id,
            domain_owner_user_id: row.domain_owner_user_id,
            approver_user_id: row.approver_user_id,
            organisational_unit: row.organisational_unit,
            status_id: row.status_id,
            version_number: row.version_number,
            is_current_version: row.is_current_version,
            approved_at: row.approved_at,
            review_frequency_id: row.review_frequency_id,
            next_review_date: row.next_review_date,
            parent_term_id: row.parent_term_id,
            source_reference: row.source_reference,
            regulatory_reference: row.regulatory_reference,
            used_in_reports: row.used_in_reports,
            used_in_policies: row.used_in_policies,
            regulatory_reporting_usage: row.regulatory_reporting_usage,
            is_cbt: row.is_cbt,
            golden_source: row.golden_source,
            golden_source_app_id: row.golden_source_app_id,
            confidence_level_id: row.confidence_level_id,
            visibility_id: row.visibility_id,
            language_id: row.language_id,
            external_reference: row.external_reference,
            previous_version_id: row.previous_version_id,
            created_by: row.created_by,
            updated_by: row.updated_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            domain_name: row.domain_name,
            category_name: row.category_name,
            term_type_name: row.term_type_name,
            unit_of_measure_name: row.unit_of_measure_name,
            classification_name: row.classification_name,
            review_frequency_name: row.review_frequency_name,
            confidence_level_name: row.confidence_level_name,
            visibility_name: row.visibility_name,
            language_name: row.language_name,
            parent_term_name: row.parent_term_name,
            golden_source_app_name: row.golden_source_app_name,
            owner_name: row.owner_name,
            steward_name: row.steward_name,
            domain_owner_name: row.domain_owner_name,
            approver_name: row.approver_name,
            status_code: row.status_code,
            status_name: row.status_name,
            regulatory_tags,
            subject_areas,
            tags,
            linked_processes,
            aliases,
            child_terms,
        }
    }
}

/// Regulatory tag attached to a term (from junction query)
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossaryRegulatoryTagItem {
    pub tag_id: Uuid,
    pub tag_code: String,
    pub tag_name: String,
    pub description: Option<String>,
}

/// Subject area attached to a term (from junction query)
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossarySubjectAreaItem {
    pub subject_area_id: Uuid,
    pub area_code: String,
    pub area_name: String,
    pub is_primary: bool,
}

/// Tag attached to a term (from junction query)
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossaryTagItem {
    pub tag_id: Uuid,
    pub tag_name: String,
}

/// Business process linked to a term (from junction query)
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossaryLinkedProcess {
    pub process_id: Uuid,
    pub process_name: String,
    pub usage_context: Option<String>,
}

// ---------------------------------------------------------------------------
// List view (joined fields for display in tables/lists)
// ---------------------------------------------------------------------------

/// List view of a glossary term with joined fields for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossaryTermListItem {
    pub term_id: Uuid,
    pub term_name: String,
    pub term_code: Option<String>,
    pub definition: String,
    pub abbreviation: Option<String>,
    pub domain_name: Option<String>,
    pub category_name: Option<String>,
    pub term_type_name: Option<String>,
    pub status_code: String,
    pub status_name: String,
    pub owner_name: Option<String>,
    pub steward_name: Option<String>,
    pub is_cbt: bool,
    pub version_number: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Create request — minimal for AI-first flow (term_name + definition required)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGlossaryTermRequest {
    pub term_name: String,
    pub definition: String,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
}

/// Update request — all fields optional for edit form
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateGlossaryTermRequest {
    // Core identity
    pub term_name: Option<String>,
    pub definition: Option<String>,
    pub abbreviation: Option<String>,

    // Definition & semantics
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub definition_notes: Option<String>,
    pub counter_examples: Option<String>,
    pub formula: Option<String>,
    pub unit_of_measure_id: Option<Uuid>,

    // Classification
    pub term_type_id: Option<Uuid>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub classification_id: Option<Uuid>,

    // Ownership
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub domain_owner_user_id: Option<Uuid>,
    pub approver_user_id: Option<Uuid>,
    pub organisational_unit: Option<String>,

    // Lifecycle
    pub approved_at: Option<DateTime<Utc>>,
    pub review_frequency_id: Option<Uuid>,

    // Relationships
    pub parent_term_id: Option<Uuid>,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,

    // Usage & context
    pub used_in_reports: Option<String>,
    pub used_in_policies: Option<String>,
    pub regulatory_reporting_usage: Option<String>,

    // Quality
    pub is_cbt: Option<bool>,
    pub golden_source: Option<String>,
    pub golden_source_app_id: Option<Uuid>,
    pub confidence_level_id: Option<Uuid>,

    // Discoverability
    pub visibility_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SearchGlossaryTermsRequest {
    pub query: Option<String>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub status: Option<String>,
    pub term_type_id: Option<Uuid>,
    pub is_cbt: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Junction management request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttachRegulatoryTagRequest {
    pub tag_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttachSubjectAreaRequest {
    pub area_id: Uuid,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AttachTagRequest {
    pub tag_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddAliasRequest {
    pub alias_name: String,
    pub alias_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Lookup types (existing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryDomain {
    pub domain_id: Uuid,
    pub domain_name: String,
    pub description: Option<String>,
    pub parent_domain_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryCategory {
    pub category_id: Uuid,
    pub category_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryTermRelationship {
    pub relationship_id: Uuid,
    pub source_term_id: Uuid,
    pub target_term_id: Uuid,
    pub relationship_type_id: Uuid,
    pub relationship_description: Option<String>,
}

// ---------------------------------------------------------------------------
// New lookup types (from migration 017)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryTermType {
    pub term_type_id: Uuid,
    pub type_code: String,
    pub type_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryReviewFrequency {
    pub frequency_id: Uuid,
    pub frequency_code: String,
    pub frequency_name: String,
    pub months_interval: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryConfidenceLevel {
    pub confidence_id: Uuid,
    pub level_code: String,
    pub level_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryVisibilityLevel {
    pub visibility_id: Uuid,
    pub visibility_code: String,
    pub visibility_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryUnitOfMeasure {
    pub unit_id: Uuid,
    pub unit_code: String,
    pub unit_name: String,
    pub unit_symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryRegulatoryTag {
    pub tag_id: Uuid,
    pub tag_code: String,
    pub tag_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossarySubjectArea {
    pub subject_area_id: Uuid,
    pub area_code: String,
    pub area_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryLanguage {
    pub language_id: Uuid,
    pub language_code: String,
    pub language_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryTag {
    pub tag_id: Uuid,
    pub tag_name: String,
}

// ---------------------------------------------------------------------------
// Bulk upload types
// ---------------------------------------------------------------------------

/// Result of a bulk upload operation — returned to the frontend.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUploadResult {
    /// Total number of data rows found in the file.
    pub total_rows: usize,
    /// Number of rows successfully inserted.
    pub successful: usize,
    /// Number of rows that failed validation or insertion.
    pub failed: usize,
    /// Per-row error details.
    pub errors: Vec<BulkUploadError>,
    /// UUIDs of the terms that were created successfully.
    pub created_term_ids: Vec<Uuid>,
}

/// A single error encountered while processing a bulk upload row.
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkUploadError {
    /// 1-based row number in the Excel file (header = row 1, first data = row 2).
    pub row: usize,
    /// Column/field name that caused the error, if applicable.
    pub field: Option<String>,
    /// Human-readable error message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Dashboard statistics
// ---------------------------------------------------------------------------

/// Dashboard statistics across all domains
#[derive(Debug, Serialize, ToSchema)]
pub struct DashboardStats {
    pub glossary_terms: i64,
    pub data_elements: i64,
    pub critical_data_elements: i64,
    pub quality_rules: i64,
    pub applications: i64,
    pub business_processes: i64,
    pub pending_tasks: i64,
}
