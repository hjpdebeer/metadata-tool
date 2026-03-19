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
    pub is_cde: bool,
    pub golden_source: Option<String>,
    pub confidence_level_id: Option<Uuid>,

    // Section 9: Discoverability
    pub visibility_id: Option<Uuid>,
    pub language_id: Option<Uuid>,
    pub external_reference: Option<String>,

    // Audit
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Detail view — term plus junction data
// ---------------------------------------------------------------------------

/// Full detail view of a glossary term with all junction data
#[derive(Debug, Serialize, ToSchema)]
pub struct GlossaryTermDetailView {
    #[serde(flatten)]
    pub term: GlossaryTerm,
    pub regulatory_tags: Vec<GlossaryRegulatoryTagItem>,
    pub subject_areas: Vec<GlossarySubjectAreaItem>,
    pub tags: Vec<GlossaryTagItem>,
    pub linked_processes: Vec<GlossaryLinkedProcess>,
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
    pub is_cde: bool,
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
    pub is_cde: Option<bool>,
    pub golden_source: Option<String>,
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
    pub is_cde: Option<bool>,
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
