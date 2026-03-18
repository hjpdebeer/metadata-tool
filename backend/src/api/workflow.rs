use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::workflow::*;
use crate::error::AppResult;
use crate::workflow::service;

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
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<Vec<PendingTaskView>>> {
    let tasks = service::get_pending_tasks(&state.pool, claims.sub, &claims.roles).await?;
    Ok(Json(tasks))
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
    State(state): State<AppState>,
    Path(instance_id): Path<Uuid>,
) -> AppResult<Json<WorkflowInstanceView>> {
    let view = service::get_workflow_instance(&state.pool, instance_id).await?;
    Ok(Json(view))
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
    State(state): State<AppState>,
    Path(instance_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<WorkflowTransitionRequest>,
) -> AppResult<Json<WorkflowInstance>> {
    let instance = service::transition_workflow(
        &state.pool,
        instance_id,
        &body.action,
        claims.sub,
        body.comments.as_deref(),
    )
    .await?;

    Ok(Json(instance))
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
    State(state): State<AppState>,
    Path(task_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CompleteTaskRequest>,
) -> AppResult<Json<WorkflowTask>> {
    let task = service::complete_task(
        &state.pool,
        task_id,
        claims.sub,
        &body.decision,
        body.comments.as_deref(),
    )
    .await?;

    Ok(Json(task))
}
