use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::applications::*;
use crate::error::AppResult;

#[utoipa::path(
    get,
    path = "/api/v1/applications",
    responses(
        (status = 200, description = "List applications", body = Vec<Application>)
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn list_applications(
    State(_state): State<AppState>,
) -> AppResult<Json<Vec<Application>>> {
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/applications/{app_id}",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    responses(
        (status = 200, description = "Application details", body = ApplicationFullView),
        (status = 404, description = "Application not found")
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn get_application(
    State(_state): State<AppState>,
    Path(_app_id): Path<Uuid>,
) -> AppResult<Json<ApplicationFullView>> {
    Err(crate::error::AppError::NotFound("Application not found".into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/applications",
    request_body = CreateApplicationRequest,
    responses(
        (status = 201, description = "Application created", body = Application)
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn create_application(
    State(_state): State<AppState>,
    Json(_body): Json<CreateApplicationRequest>,
) -> AppResult<Json<Application>> {
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    put,
    path = "/api/v1/applications/{app_id}",
    params(("app_id" = Uuid, Path, description = "Application ID")),
    request_body = UpdateApplicationRequest,
    responses(
        (status = 200, description = "Application updated", body = Application)
    ),
    security(("bearer_auth" = [])),
    tag = "applications"
)]
pub async fn update_application(
    State(_state): State<AppState>,
    Path(_app_id): Path<Uuid>,
    Json(_body): Json<UpdateApplicationRequest>,
) -> AppResult<Json<Application>> {
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}
