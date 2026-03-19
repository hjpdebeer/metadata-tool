use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// In-app notification
// ---------------------------------------------------------------------------

/// An in-app notification displayed to a user. Maps to the `in_app_notifications` table.
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

/// A queued email notification pending delivery via Microsoft Graph API. Maps to the `notification_queue` table.
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

/// A reusable email notification template with subject and body. Maps to the `notification_templates` table.
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

/// A user's notification delivery preferences per event type. Maps to the `notification_preferences` table.
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

/// Query parameters for paginating in-app notifications.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListNotificationsParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Response containing the count of unread notifications for the current user.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: i64,
}

/// A single notification preference setting for one event type.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferenceRequest {
    pub event_type: String,
    pub email_enabled: bool,
    pub in_app_enabled: bool,
}

/// Request body for bulk-updating notification preferences.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferencesRequest {
    pub preferences: Vec<UpdatePreferenceRequest>,
}

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedInAppNotifications {
    pub data: Vec<InAppNotification>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}
