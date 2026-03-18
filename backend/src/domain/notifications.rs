use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// In-app notification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct InAppNotification {
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub link_url: Option<String>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Notification queue entry (email)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NotificationQueueEntry {
    pub notification_id: Uuid,
    pub template_id: Option<Uuid>,
    pub recipient_user_id: Uuid,
    pub recipient_email: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub status: String,
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Notification template
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NotificationTemplate {
    pub template_id: Uuid,
    pub template_code: String,
    pub template_name: String,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Notification preferences
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct NotificationPreference {
    pub preference_id: Uuid,
    pub user_id: Uuid,
    pub event_type: String,
    pub email_enabled: bool,
    pub in_app_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListNotificationsParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferenceRequest {
    pub event_type: String,
    pub email_enabled: bool,
    pub in_app_enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferencesRequest {
    pub preferences: Vec<UpdatePreferenceRequest>,
}

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedInAppNotifications {
    pub data: Vec<InAppNotification>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}
