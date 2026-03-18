use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::glossary::PaginatedResponse;
use crate::domain::notifications::*;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// list_notifications — GET /api/v1/notifications
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/notifications",
    params(ListNotificationsParams),
    responses(
        (status = 200, description = "Paginated list of in-app notifications",
         body = PaginatedInAppNotifications)
    ),
    security(("bearer_auth" = [])),
    tag = "notifications"
)]
pub async fn list_notifications(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ListNotificationsParams>,
) -> AppResult<Json<PaginatedResponse<InAppNotification>>> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM in_app_notifications WHERE user_id = $1",
    )
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    let items = sqlx::query_as::<_, InAppNotification>(
        r#"
        SELECT notification_id, user_id, title, message, link_url,
               is_read, read_at, entity_type, entity_id, created_at
        FROM in_app_notifications
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        OFFSET $3
        "#,
    )
    .bind(claims.sub)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PaginatedResponse {
        data: items,
        total_count,
        page,
        page_size,
    }))
}

// ---------------------------------------------------------------------------
// mark_read — POST /api/v1/notifications/{id}/read
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/notifications/{notification_id}/read",
    params(("notification_id" = Uuid, Path, description = "Notification ID")),
    responses(
        (status = 200, description = "Notification marked as read"),
        (status = 404, description = "Notification not found")
    ),
    security(("bearer_auth" = [])),
    tag = "notifications"
)]
pub async fn mark_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(notification_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let rows_affected = sqlx::query(
        "UPDATE in_app_notifications
         SET is_read = TRUE, read_at = CURRENT_TIMESTAMP
         WHERE notification_id = $1 AND user_id = $2 AND is_read = FALSE",
    )
    .bind(notification_id)
    .bind(claims.sub)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        // Either not found or already read — check existence
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM in_app_notifications WHERE notification_id = $1 AND user_id = $2)",
        )
        .bind(notification_id)
        .bind(claims.sub)
        .fetch_one(&state.pool)
        .await?;

        if !exists {
            return Err(AppError::NotFound(format!(
                "notification not found: {notification_id}"
            )));
        }
    }

    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// mark_all_read — POST /api/v1/notifications/read-all
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/notifications/read-all",
    responses(
        (status = 200, description = "All notifications marked as read")
    ),
    security(("bearer_auth" = [])),
    tag = "notifications"
)]
pub async fn mark_all_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<StatusCode> {
    sqlx::query(
        "UPDATE in_app_notifications
         SET is_read = TRUE, read_at = CURRENT_TIMESTAMP
         WHERE user_id = $1 AND is_read = FALSE",
    )
    .bind(claims.sub)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// unread_count — GET /api/v1/notifications/unread-count
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/notifications/unread-count",
    responses(
        (status = 200, description = "Unread notification count", body = UnreadCountResponse)
    ),
    security(("bearer_auth" = [])),
    tag = "notifications"
)]
pub async fn unread_count(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<UnreadCountResponse>> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM in_app_notifications WHERE user_id = $1 AND is_read = FALSE",
    )
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(UnreadCountResponse { count }))
}

// ---------------------------------------------------------------------------
// get_preferences — GET /api/v1/notifications/preferences
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/notifications/preferences",
    responses(
        (status = 200, description = "User notification preferences",
         body = Vec<NotificationPreference>)
    ),
    security(("bearer_auth" = [])),
    tag = "notifications"
)]
pub async fn get_preferences(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<Vec<NotificationPreference>>> {
    // Return existing preferences. If none exist yet, the frontend can
    // display defaults and the user can save them via update_preferences.
    let prefs = sqlx::query_as::<_, NotificationPreference>(
        r#"
        SELECT preference_id, user_id, event_type, email_enabled,
               in_app_enabled, created_at, updated_at
        FROM notification_preferences
        WHERE user_id = $1
        ORDER BY event_type ASC
        "#,
    )
    .bind(claims.sub)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(prefs))
}

// ---------------------------------------------------------------------------
// update_preferences — PUT /api/v1/notifications/preferences
// ---------------------------------------------------------------------------

#[utoipa::path(
    put,
    path = "/api/v1/notifications/preferences",
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "Preferences updated",
         body = Vec<NotificationPreference>)
    ),
    security(("bearer_auth" = [])),
    tag = "notifications"
)]
pub async fn update_preferences(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdatePreferencesRequest>,
) -> AppResult<Json<Vec<NotificationPreference>>> {
    for pref in &body.preferences {
        sqlx::query(
            r#"
            INSERT INTO notification_preferences (user_id, event_type, email_enabled, in_app_enabled)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, event_type)
            DO UPDATE SET email_enabled = $3, in_app_enabled = $4, updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(claims.sub)
        .bind(&pref.event_type)
        .bind(pref.email_enabled)
        .bind(pref.in_app_enabled)
        .execute(&state.pool)
        .await?;
    }

    // Return the updated preferences
    let prefs = sqlx::query_as::<_, NotificationPreference>(
        r#"
        SELECT preference_id, user_id, event_type, email_enabled,
               in_app_enabled, created_at, updated_at
        FROM notification_preferences
        WHERE user_id = $1
        ORDER BY event_type ASC
        "#,
    )
    .bind(claims.sub)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(prefs))
}
