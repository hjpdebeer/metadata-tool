use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::processes::*;
use crate::error::AppResult;

#[utoipa::path(
    get,
    path = "/api/v1/processes",
    responses(
        (status = 200, description = "List business processes", body = Vec<BusinessProcess>)
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_processes(
    State(_state): State<AppState>,
) -> AppResult<Json<Vec<BusinessProcess>>> {
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/processes/{process_id}",
    params(("process_id" = Uuid, Path, description = "Process ID")),
    responses(
        (status = 200, description = "Process details", body = BusinessProcessFullView),
        (status = 404, description = "Process not found")
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn get_process(
    State(_state): State<AppState>,
    Path(_process_id): Path<Uuid>,
) -> AppResult<Json<BusinessProcessFullView>> {
    Err(crate::error::AppError::NotFound("Process not found".into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/processes",
    request_body = CreateBusinessProcessRequest,
    responses(
        (status = 201, description = "Process created", body = BusinessProcess)
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn create_process(
    State(_state): State<AppState>,
    Json(_body): Json<CreateBusinessProcessRequest>,
) -> AppResult<Json<BusinessProcess>> {
    // TODO: Create process, if is_critical then auto-designate linked elements as CDEs
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/processes/critical",
    responses(
        (status = 200, description = "List critical business processes", body = Vec<BusinessProcess>)
    ),
    security(("bearer_auth" = [])),
    tag = "processes"
)]
pub async fn list_critical_processes(
    State(_state): State<AppState>,
) -> AppResult<Json<Vec<BusinessProcess>>> {
    Ok(Json(vec![]))
}
