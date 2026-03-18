use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::workflow::*;
use crate::error::AppResult;

#[utoipa::path(
    get,
    path = "/api/v1/workflow/tasks/pending",
    responses(
        (status = 200, description = "Pending tasks for current user", body = Vec<PendingTaskView>)
    ),
    security(("bearer_auth" = [])),
    tag = "workflow"
)]
pub async fn my_pending_tasks(
    State(_state): State<AppState>,
) -> AppResult<Json<Vec<PendingTaskView>>> {
    // TODO: Get tasks assigned to current user or their roles
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/workflow/instances/{instance_id}",
    params(("instance_id" = Uuid, Path, description = "Workflow instance ID")),
    responses(
        (status = 200, description = "Workflow instance details", body = WorkflowInstanceView)
    ),
    security(("bearer_auth" = [])),
    tag = "workflow"
)]
pub async fn get_instance(
    State(_state): State<AppState>,
    Path(_instance_id): Path<Uuid>,
) -> AppResult<Json<WorkflowInstanceView>> {
    Err(crate::error::AppError::NotFound("Instance not found".into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workflow/instances/{instance_id}/transition",
    params(("instance_id" = Uuid, Path, description = "Workflow instance ID")),
    request_body = WorkflowTransitionRequest,
    responses(
        (status = 200, description = "Transition successful", body = WorkflowInstance),
        (status = 422, description = "Invalid transition")
    ),
    security(("bearer_auth" = [])),
    tag = "workflow"
)]
pub async fn transition(
    State(_state): State<AppState>,
    Path(_instance_id): Path<Uuid>,
    Json(_body): Json<WorkflowTransitionRequest>,
) -> AppResult<Json<WorkflowInstance>> {
    // TODO: Validate transition, update state, create tasks, send notifications
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/workflow/tasks/{task_id}/complete",
    params(("task_id" = Uuid, Path, description = "Task ID")),
    request_body = CompleteTaskRequest,
    responses(
        (status = 200, description = "Task completed", body = WorkflowTask)
    ),
    security(("bearer_auth" = [])),
    tag = "workflow"
)]
pub async fn complete_task(
    State(_state): State<AppState>,
    Path(_task_id): Path<Uuid>,
    Json(_body): Json<CompleteTaskRequest>,
) -> AppResult<Json<WorkflowTask>> {
    // TODO: Complete task, check if all tasks done, auto-transition if so
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}
