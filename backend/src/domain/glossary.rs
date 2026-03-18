use chrono::{DateTime, Utc};
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
// Full glossary term (single-record detail view)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryTerm {
    pub term_id: Uuid,
    pub term_name: String,
    pub definition: String,
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub abbreviation: Option<String>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub status_id: Uuid,
    pub owner_user_id: Option<Uuid>,
    pub steward_user_id: Option<Uuid>,
    pub version_number: i32,
    pub is_current_version: bool,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// List view (joined fields for display in tables/lists)
// ---------------------------------------------------------------------------

/// List view of a glossary term with joined fields for display
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct GlossaryTermListItem {
    pub term_id: Uuid,
    pub term_name: String,
    pub definition: String,
    pub abbreviation: Option<String>,
    pub domain_name: Option<String>,
    pub category_name: Option<String>,
    pub status_code: String,
    pub status_name: String,
    pub owner_name: Option<String>,
    pub steward_name: Option<String>,
    pub version_number: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGlossaryTermRequest {
    pub term_name: String,
    pub definition: String,
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub abbreviation: Option<String>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateGlossaryTermRequest {
    pub term_name: Option<String>,
    pub definition: Option<String>,
    pub business_context: Option<String>,
    pub examples: Option<String>,
    pub abbreviation: Option<String>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub source_reference: Option<String>,
    pub regulatory_reference: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct SearchGlossaryTermsRequest {
    pub query: Option<String>,
    pub domain_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Lookup types
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
