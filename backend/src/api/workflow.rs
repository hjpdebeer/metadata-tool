use axum::extract::{Path, State};
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::workflow::*;
use crate::error::AppResult;
use crate::workflow::service;

/// List pending workflow tasks assigned to the current user or their roles.
/// Requires authentication.
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

/// Retrieve a workflow instance with its current state, tasks, and transition history.
/// Requires authentication.
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

/// Look up the workflow instance for a given entity (e.g. glossary term) by its entity_id.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/workflow/instances/by-entity/{entity_id}",
    params(("entity_id" = Uuid, Path, description = "Entity ID (e.g. term_id)")),
    responses(
        (status = 200, description = "Workflow instance for entity", body = WorkflowInstanceView),
        (status = 404, description = "No workflow instance found for entity")
    ),
    security(("bearer_auth" = [])),
    tag = "workflow"
)]
pub async fn get_instance_by_entity(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> AppResult<Json<WorkflowInstanceView>> {
    let view = service::get_workflow_instance_by_entity(&state.pool, entity_id).await?;
    Ok(Json(view))
}

/// Perform a workflow state transition (e.g. SUBMIT, APPROVE, REJECT) on an instance (Principle 5).
/// Requires authentication.
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

/// Complete a workflow task with a decision (APPROVE, REJECT, or REVISE).
/// Requires authentication.
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
