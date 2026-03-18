use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryDomain {
    pub domain_id: Uuid,
    pub domain_name: String,
    pub description: Option<String>,
    pub parent_domain_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GlossaryTermRelationship {
    pub relationship_id: Uuid,
    pub source_term_id: Uuid,
    pub target_term_id: Uuid,
    pub relationship_type_id: Uuid,
    pub relationship_description: Option<String>,
}
