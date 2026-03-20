use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::workflow::{
    PendingTaskView, WorkflowHistoryEntry, WorkflowInstance, WorkflowInstanceView, WorkflowTask,
};
use crate::error::{AppError, AppResult};
use crate::notifications;

use super::{
    ACTION_APPROVE, ACTION_REJECT, ACTION_REVISE, STATE_DRAFT, STATE_UNDER_REVIEW,
    STATE_PENDING_APPROVAL, TASK_STATUS_CANCELLED, TASK_STATUS_COMPLETED, TASK_STATUS_PENDING,
};

// ---------------------------------------------------------------------------
// Internal row types for queries that don't map to domain structs directly
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct WorkflowDefRow {
    workflow_def_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct StateRow {
    state_id: Uuid,
    state_code: String,
    is_terminal: bool,
}

#[derive(sqlx::FromRow)]
struct TransitionRow {
    to_state_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct EntityTypeRow {
    entity_type_id: Uuid,
    table_name: String,
}

#[derive(sqlx::FromRow)]
struct PendingTaskRow {
    // Task fields
    task_id: Uuid,
    instance_id: Uuid,
    task_type: String,
    task_name: String,
    description: Option<String>,
    assigned_to_user_id: Option<Uuid>,
    assigned_to_role_id: Option<Uuid>,
    status: String,
    due_date: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    completed_by: Option<Uuid>,
    decision: Option<String>,
    comments: Option<String>,
    // Joined fields
    entity_type: String,
    entity_id: Uuid,
    workflow_name: String,
    submitted_by: String,
    submitted_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct InstanceViewRow {
    // Instance fields
    instance_id: Uuid,
    workflow_def_id: Uuid,
    entity_type_id: Uuid,
    entity_id: Uuid,
    current_state_id: Uuid,
    initiated_by: Uuid,
    initiated_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    completion_notes: Option<String>,
    // Joined fields
    current_state_name: String,
    entity_type_name: String,
    initiated_by_name: String,
}

// ---------------------------------------------------------------------------
// 1. initiate_workflow
// ---------------------------------------------------------------------------

/// Create a new workflow instance in DRAFT state for the given entity.
///
/// Looks up the active workflow definition for `entity_type_code`, resolves
/// the DRAFT initial state, and inserts a new `workflow_instances` row.
pub async fn initiate_workflow(
    pool: &PgPool,
    entity_type_code: &str,
    entity_id: Uuid,
    initiated_by: Uuid,
) -> AppResult<WorkflowInstance> {
    // Look up the entity type
    let entity_type = sqlx::query_as::<_, EntityTypeRow>(
        "SELECT entity_type_id, table_name
         FROM workflow_entity_types
         WHERE type_code = $1",
    )
    .bind(entity_type_code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!(
            "workflow entity type not found: {entity_type_code}"
        ))
    })?;

    // Look up the active workflow definition for this entity type
    let wf_def = sqlx::query_as::<_, WorkflowDefRow>(
        "SELECT workflow_def_id
         FROM workflow_definitions
         WHERE entity_type_id = $1 AND is_active = TRUE
         LIMIT 1",
    )
    .bind(entity_type.entity_type_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::Workflow(format!(
            "no active workflow definition for entity type: {entity_type_code}"
        ))
    })?;

    // Look up the DRAFT (initial) state
    let draft_state = sqlx::query_as::<_, StateRow>(
        "SELECT state_id, state_code, is_terminal
         FROM workflow_states
         WHERE state_code = $1 AND is_initial = TRUE",
    )
    .bind(STATE_DRAFT)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Workflow("draft workflow state not found".into()))?;

    // Create the workflow instance
    let instance = sqlx::query_as::<_, WorkflowInstance>(
        "INSERT INTO workflow_instances
             (workflow_def_id, entity_type_id, entity_id, current_state_id, initiated_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING instance_id, workflow_def_id, entity_type_id, entity_id,
                   current_state_id, initiated_by, initiated_at,
                   completed_at, completion_notes",
    )
    .bind(wf_def.workflow_def_id)
    .bind(entity_type.entity_type_id)
    .bind(entity_id)
    .bind(draft_state.state_id)
    .bind(initiated_by)
    .fetch_one(pool)
    .await?;

    Ok(instance)
}

// ---------------------------------------------------------------------------
// 2. transition_workflow
// ---------------------------------------------------------------------------

/// Advance a workflow instance by performing the given action.
///
/// Validates that the transition is legal from the current state, records
/// history, optionally creates approval tasks (when entering UNDER_REVIEW),
/// marks terminal states, and updates the underlying entity's `status_id`.
pub async fn transition_workflow(
    pool: &PgPool,
    instance_id: Uuid,
    action_code: &str,
    performed_by: Uuid,
    comments: Option<&str>,
) -> AppResult<WorkflowInstance> {
    // Fetch the current instance
    let instance = sqlx::query_as::<_, WorkflowInstance>(
        "SELECT instance_id, workflow_def_id, entity_type_id, entity_id,
                current_state_id, initiated_by, initiated_at,
                completed_at, completion_notes
         FROM workflow_instances
         WHERE instance_id = $1",
    )
    .bind(instance_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!("workflow instance not found: {instance_id}"))
    })?;

    if instance.completed_at.is_some() {
        return Err(AppError::Workflow(
            "cannot transition a completed workflow instance".into(),
        ));
    }

    // Look up valid transition from current state with this action
    let transition = sqlx::query_as::<_, TransitionRow>(
        "SELECT to_state_id
         FROM workflow_transitions
         WHERE workflow_def_id = $1
           AND from_state_id = $2
           AND action_code = $3",
    )
    .bind(instance.workflow_def_id)
    .bind(instance.current_state_id)
    .bind(action_code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::Workflow(format!(
            "invalid transition: action '{action_code}' is not allowed from the current state"
        ))
    })?;

    let new_state_id = transition.to_state_id;

    // Governance validation: when submitting for review (SUBMIT action),
    // all mandatory ownership fields must be populated.
    if action_code == super::ACTION_SUBMIT {
        validate_ownership_before_submit(pool, &instance).await?;
    }

    // Look up the new state to check if it is terminal
    let new_state = sqlx::query_as::<_, StateRow>(
        "SELECT state_id, state_code, is_terminal
         FROM workflow_states
         WHERE state_id = $1",
    )
    .bind(new_state_id)
    .fetch_one(pool)
    .await?;

    // Record in workflow_history
    sqlx::query(
        "INSERT INTO workflow_history
             (instance_id, from_state_id, to_state_id, action, performed_by, comments)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(instance_id)
    .bind(instance.current_state_id)
    .bind(new_state_id)
    .bind(action_code)
    .bind(performed_by)
    .bind(comments)
    .execute(pool)
    .await?;

    // Update instance current state (and completed_at if terminal)
    let completed_at: Option<DateTime<Utc>> = if new_state.is_terminal {
        Some(Utc::now())
    } else {
        None
    };

    let updated_instance = sqlx::query_as::<_, WorkflowInstance>(
        "UPDATE workflow_instances
         SET current_state_id = $1,
             completed_at = COALESCE($2, completed_at),
             updated_at = CURRENT_TIMESTAMP
         WHERE instance_id = $3
         RETURNING instance_id, workflow_def_id, entity_type_id, entity_id,
                   current_state_id, initiated_by, initiated_at,
                   completed_at, completion_notes",
    )
    .bind(new_state_id)
    .bind(completed_at)
    .bind(instance_id)
    .fetch_one(pool)
    .await?;

    // Complete all existing PENDING tasks for this instance before creating new ones.
    // This ensures tasks from the previous state don't linger as duplicates.
    sqlx::query(
        "UPDATE workflow_tasks
         SET status = $1, completed_at = CURRENT_TIMESTAMP, completed_by = $2, updated_at = CURRENT_TIMESTAMP
         WHERE instance_id = $3 AND status = $4",
    )
    .bind(TASK_STATUS_COMPLETED)
    .bind(performed_by)
    .bind(instance_id)
    .bind(TASK_STATUS_PENDING)
    .execute(pool)
    .await?;

    // If entering UNDER_REVIEW, create review task for Data Steward
    if new_state.state_code == STATE_UNDER_REVIEW {
        create_steward_review_task(pool, &updated_instance).await?;
    }

    // If entering PENDING_APPROVAL, create final approval task for Owner
    if new_state.state_code == STATE_PENDING_APPROVAL {
        create_owner_approval_task(pool, &updated_instance).await?;
    }

    // If terminal (ACCEPTED, REJECTED, DEPRECATED), cancel any remaining PENDING tasks
    // and notify the initiator of the final outcome.
    if new_state.is_terminal {
        sqlx::query(
            "UPDATE workflow_tasks
             SET status = $1, updated_at = CURRENT_TIMESTAMP
             WHERE instance_id = $2 AND status = $3",
        )
        .bind(TASK_STATUS_CANCELLED)
        .bind(instance_id)
        .bind(TASK_STATUS_PENDING)
        .execute(pool)
        .await?;

        // Notify the workflow initiator about the terminal state
        let entity_type_name = sqlx::query_scalar::<_, String>(
            "SELECT type_name FROM workflow_entity_types WHERE entity_type_id = $1",
        )
        .bind(updated_instance.entity_type_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or_else(|| "Entity".to_string());

        let entity_name = resolve_entity_name(
            pool,
            &entity_type_name,
            updated_instance.entity_id,
        )
        .await?;

        // Resolve the old state name for the notification message
        let old_state_name = sqlx::query_scalar::<_, String>(
            "SELECT state_name FROM workflow_states WHERE state_id = $1",
        )
        .bind(instance.current_state_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or_else(|| "Unknown".to_string());

        if let Err(e) = notifications::queue_workflow_state_changed_notification(
            pool,
            updated_instance.initiated_by,
            &entity_type_name,
            &entity_name,
            updated_instance.entity_id,
            &old_state_name,
            &new_state.state_code,
            comments,
        )
        .await
        {
            tracing::warn!(
                initiator = %updated_instance.initiated_by,
                error = %e,
                "Failed to send workflow state change notification"
            );
        }
    }

    // Update the entity's status_id to match the new workflow state.
    // The entity_statuses table uses the same codes as workflow_states.
    update_entity_status(pool, &updated_instance, &new_state.state_code).await?;

    Ok(updated_instance)
}

// ---------------------------------------------------------------------------
// 3. get_pending_tasks
// ---------------------------------------------------------------------------

/// Retrieve pending workflow tasks assigned to a user (directly or via roles).
pub async fn get_pending_tasks(
    pool: &PgPool,
    user_id: Uuid,
    role_codes: &[String],
) -> AppResult<Vec<PendingTaskView>> {
    // Build the role IDs list from role codes
    let role_ids: Vec<Uuid> = if role_codes.is_empty() {
        vec![]
    } else {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT role_id FROM roles WHERE role_code = ANY($1)",
        )
        .bind(role_codes)
        .fetch_all(pool)
        .await?
    };

    let rows = sqlx::query_as::<_, PendingTaskRow>(
        "SELECT
             t.task_id, t.instance_id, t.task_type, t.task_name,
             t.description, t.assigned_to_user_id, t.assigned_to_role_id,
             t.status, t.due_date, t.completed_at, t.completed_by,
             t.decision, t.comments,
             wet.type_name  AS entity_type,
             wi.entity_id,
             wd.workflow_name,
             u.display_name AS submitted_by,
             wi.initiated_at AS submitted_at
         FROM workflow_tasks t
         JOIN workflow_instances wi ON wi.instance_id = t.instance_id
         JOIN workflow_definitions wd ON wd.workflow_def_id = wi.workflow_def_id
         JOIN workflow_entity_types wet ON wet.entity_type_id = wi.entity_type_id
         JOIN users u ON u.user_id = wi.initiated_by
         WHERE t.status = $1
           AND (t.assigned_to_user_id = $2 OR t.assigned_to_role_id = ANY($3))
         ORDER BY t.due_date ASC NULLS LAST, t.created_at ASC",
    )
    .bind(TASK_STATUS_PENDING)
    .bind(user_id)
    .bind(&role_ids)
    .fetch_all(pool)
    .await?;

    // Resolve entity names for each task
    let mut tasks = Vec::with_capacity(rows.len());
    for row in rows {
        let entity_name = resolve_entity_name(pool, &row.entity_type, row.entity_id).await?;

        tasks.push(PendingTaskView {
            task: WorkflowTask {
                task_id: row.task_id,
                instance_id: row.instance_id,
                task_type: row.task_type,
                task_name: row.task_name,
                description: row.description,
                assigned_to_user_id: row.assigned_to_user_id,
                assigned_to_role_id: row.assigned_to_role_id,
                status: row.status,
                due_date: row.due_date,
                completed_at: row.completed_at,
                completed_by: row.completed_by,
                decision: row.decision,
                comments: row.comments,
            },
            entity_type: row.entity_type,
            entity_name,
            entity_id: row.entity_id,
            workflow_name: row.workflow_name,
            submitted_by: row.submitted_by,
            submitted_at: row.submitted_at,
        });
    }

    Ok(tasks)
}

// ---------------------------------------------------------------------------
// 4. complete_task
// ---------------------------------------------------------------------------

/// Complete a workflow task with a decision (APPROVE, REJECT, or REVISE).
///
/// Marks the task as completed, then triggers the corresponding workflow
/// transition based on the decision.
pub async fn complete_task(
    pool: &PgPool,
    task_id: Uuid,
    completed_by: Uuid,
    decision: &str,
    comments: Option<&str>,
) -> AppResult<WorkflowTask> {
    // Look up the task
    let task = sqlx::query_as::<_, WorkflowTask>(
        "SELECT task_id, instance_id, task_type, task_name, description,
                assigned_to_user_id, assigned_to_role_id, status,
                due_date, completed_at, completed_by, decision, comments
         FROM workflow_tasks
         WHERE task_id = $1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("workflow task not found: {task_id}")))?;

    if task.status != TASK_STATUS_PENDING {
        return Err(AppError::Workflow(format!(
            "task is not pending (current status: {})",
            task.status
        )));
    }

    // Validate decision — use workflow constants (CS-003)
    let action_code = match decision {
        ACTION_APPROVE => ACTION_APPROVE,
        ACTION_REJECT => ACTION_REJECT,
        ACTION_REVISE => ACTION_REVISE,
        _ => {
            return Err(AppError::Validation(format!(
                "invalid decision: '{decision}'. Must be {ACTION_APPROVE}, {ACTION_REJECT}, or {ACTION_REVISE}"
            )));
        }
    };

    // Mark task as completed
    let updated_task = sqlx::query_as::<_, WorkflowTask>(
        "UPDATE workflow_tasks
         SET status = $1,
             completed_at = CURRENT_TIMESTAMP,
             completed_by = $2,
             decision = $3,
             comments = $4,
             updated_at = CURRENT_TIMESTAMP
         WHERE task_id = $5
         RETURNING task_id, instance_id, task_type, task_name, description,
                   assigned_to_user_id, assigned_to_role_id, status,
                   due_date, completed_at, completed_by, decision, comments",
    )
    .bind(TASK_STATUS_COMPLETED)
    .bind(completed_by)
    .bind(decision)
    .bind(comments)
    .bind(task_id)
    .fetch_one(pool)
    .await?;

    // Advance the workflow based on the decision
    transition_workflow(
        pool,
        task.instance_id,
        action_code,
        completed_by,
        comments,
    )
    .await?;

    Ok(updated_task)
}

// ---------------------------------------------------------------------------
// 5. get_workflow_instance
// ---------------------------------------------------------------------------

/// Fetch a workflow instance with its current state name, entity type,
/// initiator name, associated tasks, and full history.
pub async fn get_workflow_instance(
    pool: &PgPool,
    instance_id: Uuid,
) -> AppResult<WorkflowInstanceView> {
    let row = sqlx::query_as::<_, InstanceViewRow>(
        "SELECT
             wi.instance_id, wi.workflow_def_id, wi.entity_type_id,
             wi.entity_id, wi.current_state_id, wi.initiated_by,
             wi.initiated_at, wi.completed_at, wi.completion_notes,
             ws.state_name   AS current_state_name,
             wet.type_name   AS entity_type_name,
             u.display_name  AS initiated_by_name
         FROM workflow_instances wi
         JOIN workflow_states ws ON ws.state_id = wi.current_state_id
         JOIN workflow_entity_types wet ON wet.entity_type_id = wi.entity_type_id
         JOIN users u ON u.user_id = wi.initiated_by
         WHERE wi.instance_id = $1",
    )
    .bind(instance_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!("workflow instance not found: {instance_id}"))
    })?;

    // Fetch tasks for this instance
    let tasks = sqlx::query_as::<_, WorkflowTask>(
        "SELECT task_id, instance_id, task_type, task_name, description,
                assigned_to_user_id, assigned_to_role_id, status,
                due_date, completed_at, completed_by, decision, comments
         FROM workflow_tasks
         WHERE instance_id = $1
         ORDER BY created_at ASC",
    )
    .bind(instance_id)
    .fetch_all(pool)
    .await?;

    // Fetch history for this instance
    let history = sqlx::query_as::<_, WorkflowHistoryEntry>(
        "SELECT history_id, instance_id, from_state_id, to_state_id,
                action, performed_by, performed_at, comments
         FROM workflow_history
         WHERE instance_id = $1
         ORDER BY performed_at ASC",
    )
    .bind(instance_id)
    .fetch_all(pool)
    .await?;

    Ok(WorkflowInstanceView {
        instance_id: row.instance_id,
        workflow_def_id: row.workflow_def_id,
        entity_type_id: row.entity_type_id,
        entity_id: row.entity_id,
        current_state_id: row.current_state_id,
        initiated_by: row.initiated_by,
        initiated_at: row.initiated_at,
        completed_at: row.completed_at,
        completion_notes: row.completion_notes,
        current_state_name: row.current_state_name,
        entity_type_name: row.entity_type_name,
        initiated_by_name: row.initiated_by_name,
        tasks,
        history,
    })
}

// ---------------------------------------------------------------------------
// 6. get_workflow_instance_by_entity
// ---------------------------------------------------------------------------

/// Look up the workflow instance for a given entity_id.
/// Returns the full instance view (same as get_workflow_instance) or 404 if none exists.
pub async fn get_workflow_instance_by_entity(
    pool: &PgPool,
    entity_id: Uuid,
) -> AppResult<WorkflowInstanceView> {
    let instance_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT instance_id FROM workflow_instances WHERE entity_id = $1 ORDER BY initiated_at DESC LIMIT 1",
    )
    .bind(entity_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::NotFound(format!("no workflow instance found for entity: {entity_id}"))
    })?;

    get_workflow_instance(pool, instance_id).await
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Create a review task assigned to the entity's Data Steward.
/// Called when entering UNDER_REVIEW state.
async fn create_steward_review_task(
    pool: &PgPool,
    instance: &WorkflowInstance,
) -> AppResult<()> {
    create_task_for_entity_role(pool, instance, "steward_user_id", "REVIEW", "Review").await
}

/// Create a final approval task assigned to the entity's Business Term Owner.
/// Called when entering PENDING_APPROVAL state.
async fn create_owner_approval_task(
    pool: &PgPool,
    instance: &WorkflowInstance,
) -> AppResult<()> {
    create_task_for_entity_role(pool, instance, "owner_user_id", ACTION_APPROVE, "Final Approval").await
}

/// Generic helper: create a workflow task assigned to a specific user looked up
/// from the entity's ownership column (e.g. steward_user_id, owner_user_id).
async fn create_task_for_entity_role(
    pool: &PgPool,
    instance: &WorkflowInstance,
    user_column: &str,
    task_type: &str,
    task_label: &str,
) -> AppResult<()> {
    // Calculate due date from the workflow definition's SLA
    let sla_hours: i32 = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT review_sla_hours FROM workflow_definitions WHERE workflow_def_id = $1",
    )
    .bind(instance.workflow_def_id)
    .fetch_one(pool)
    .await?
    .unwrap_or(72);

    let due_date = Utc::now()
        + TimeDelta::try_hours(i64::from(sla_hours))
            .unwrap_or_else(|| TimeDelta::try_hours(72).expect("72 hours fits in TimeDelta"));

    // Resolve the entity type name and entity name for notifications
    let entity_type_name = sqlx::query_scalar::<_, String>(
        "SELECT type_name FROM workflow_entity_types WHERE entity_type_id = $1",
    )
    .bind(instance.entity_type_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "Entity".to_string());

    let entity_name = resolve_entity_name(pool, &entity_type_name, instance.entity_id).await?;

    // Look up the assigned user from the entity's ownership column.
    // Resolves the entity type to determine which table to query.
    // Uses a safe static match to avoid SQL injection (Principle 10).
    let entity_table = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM workflow_entity_types WHERE entity_type_id = $1",
    )
    .bind(instance.entity_type_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default();

    let assigned_user_id: Option<Uuid> = match (entity_table.as_str(), user_column) {
        ("glossary_terms", "steward_user_id") => {
            sqlx::query_scalar("SELECT steward_user_id FROM glossary_terms WHERE term_id = $1 AND deleted_at IS NULL")
                .bind(instance.entity_id)
                .fetch_optional(pool)
                .await?
                .flatten()
        }
        ("glossary_terms", "owner_user_id") => {
            sqlx::query_scalar("SELECT owner_user_id FROM glossary_terms WHERE term_id = $1 AND deleted_at IS NULL")
                .bind(instance.entity_id)
                .fetch_optional(pool)
                .await?
                .flatten()
        }
        ("applications", "steward_user_id") => {
            sqlx::query_scalar("SELECT steward_user_id FROM applications WHERE application_id = $1 AND deleted_at IS NULL")
                .bind(instance.entity_id)
                .fetch_optional(pool)
                .await?
                .flatten()
        }
        ("applications", "owner_user_id") => {
            sqlx::query_scalar("SELECT business_owner_id FROM applications WHERE application_id = $1 AND deleted_at IS NULL")
                .bind(instance.entity_id)
                .fetch_optional(pool)
                .await?
                .flatten()
        }
        _ => None,
    };

    let task_name = format!("{} — {}", task_label, entity_name);
    let description = format!(
        "{} for {} '{}'.",
        task_label, entity_type_name, entity_name,
    );
    let due_date_str = due_date.format("%Y-%m-%d %H:%M UTC").to_string();

    sqlx::query(
        "INSERT INTO workflow_tasks
             (instance_id, task_type, task_name, description,
              assigned_to_user_id, status, due_date)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(instance.instance_id)
    .bind(task_type)
    .bind(&task_name)
    .bind(&description)
    .bind(assigned_user_id)
    .bind(TASK_STATUS_PENDING)
    .bind(due_date)
    .execute(pool)
    .await?;

    // Notify the assigned user
    #[allow(clippy::collapsible_if)] // Intentionally separate: Some check vs Err check
    if let Some(user_id) = assigned_user_id {
        if let Err(e) = notifications::queue_workflow_task_notification(
            pool,
            user_id,
            &entity_type_name,
            &entity_name,
            instance.entity_id,
            Some(&due_date_str),
        )
        .await
        {
            tracing::warn!(
                user_id = %user_id,
                error = %e,
                "failed to send task assignment notification"
            );
        }
    }

    Ok(())
}

/// Update the underlying entity's `status_id` column to match the new
/// workflow state. Uses the `table_name` from `workflow_entity_types` to
/// determine which table to update and the `entity_statuses` table to
/// resolve the matching status row.
///
/// Each entity table has a `status_id` FK column and a primary key column
/// that follows the pattern `{singular}_id` (e.g. `term_id`, `element_id`).
/// We use a safe mapping from table name to PK column rather than dynamic
/// SQL to avoid any injection risk (Principle 10).
async fn update_entity_status(
    pool: &PgPool,
    instance: &WorkflowInstance,
    state_code: &str,
) -> AppResult<()> {
    // Resolve the entity_statuses row matching this workflow state code
    let status_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT status_id FROM entity_statuses WHERE status_code = $1",
    )
    .bind(state_code)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        AppError::Workflow(format!(
            "entity status not found for state code: {state_code}"
        ))
    })?;

    // Resolve the table name from the entity type
    let entity_type = sqlx::query_as::<_, EntityTypeRow>(
        "SELECT entity_type_id, table_name
         FROM workflow_entity_types
         WHERE entity_type_id = $1",
    )
    .bind(instance.entity_type_id)
    .fetch_one(pool)
    .await?;

    // Map table_name to the correct primary key column and execute update.
    // We use an explicit match to avoid dynamic SQL / string interpolation
    // in queries (Principle 10 — parameterized queries only).
    match entity_type.table_name.as_str() {
        "glossary_terms" => {
            if state_code == super::STATE_ACCEPTED {
                // Stamp approved_at, set is_current_version, recalculate next_review_date
                sqlx::query(
                    "UPDATE glossary_terms
                     SET status_id = $1,
                         approved_at = CURRENT_TIMESTAMP,
                         is_current_version = TRUE,
                         next_review_date = CURRENT_DATE + (
                             SELECT COALESCE(grf.months_interval, 12) * INTERVAL '1 month'
                             FROM glossary_review_frequencies grf
                             WHERE grf.frequency_id = glossary_terms.review_frequency_id
                         ),
                         updated_at = CURRENT_TIMESTAMP
                     WHERE term_id = $2",
                )
                .bind(status_id)
                .bind(instance.entity_id)
                .execute(pool)
                .await?;

                // Version swap: if this is an amendment (has previous_version_id),
                // mark the old version as no longer current.
                let previous_id = sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT previous_version_id FROM glossary_terms WHERE term_id = $1",
                )
                .bind(instance.entity_id)
                .fetch_one(pool)
                .await?;

                if let Some(old_term_id) = previous_id {
                    // Mark old version as SUPERSEDED (not just is_current_version = false)
                    let superseded_status_id = sqlx::query_scalar::<_, Uuid>(
                        "SELECT status_id FROM entity_statuses WHERE status_code = 'SUPERSEDED'",
                    )
                    .fetch_one(pool)
                    .await?;

                    sqlx::query(
                        "UPDATE glossary_terms
                         SET is_current_version = FALSE,
                             status_id = $1,
                             updated_at = CURRENT_TIMESTAMP
                         WHERE term_id = $2",
                    )
                    .bind(superseded_status_id)
                    .bind(old_term_id)
                    .execute(pool)
                    .await?;

                    // Also mark the old version's workflow instance as completed
                    sqlx::query(
                        "UPDATE workflow_instances
                         SET completed_at = CURRENT_TIMESTAMP,
                             completion_notes = 'Superseded by newer version',
                             updated_at = CURRENT_TIMESTAMP
                         WHERE entity_id = $1 AND completed_at IS NULL",
                    )
                    .bind(old_term_id)
                    .execute(pool)
                    .await?;

                    tracing::info!(
                        new_term_id = %instance.entity_id,
                        old_term_id = %old_term_id,
                        "version swap: amendment approved, old version superseded"
                    );
                }
            } else {
                sqlx::query(
                    "UPDATE glossary_terms SET status_id = $1, updated_at = CURRENT_TIMESTAMP WHERE term_id = $2",
                )
                .bind(status_id)
                .bind(instance.entity_id)
                .execute(pool)
                .await?;
            }
        }
        "data_elements" => {
            sqlx::query(
                "UPDATE data_elements SET status_id = $1, updated_at = CURRENT_TIMESTAMP WHERE element_id = $2",
            )
            .bind(status_id)
            .bind(instance.entity_id)
            .execute(pool)
            .await?;
        }
        "quality_rules" => {
            sqlx::query(
                "UPDATE quality_rules SET status_id = $1, updated_at = CURRENT_TIMESTAMP WHERE rule_id = $2",
            )
            .bind(status_id)
            .bind(instance.entity_id)
            .execute(pool)
            .await?;
        }
        "applications" => {
            if state_code == super::STATE_ACCEPTED {
                // Stamp approved_at, set is_current_version, recalculate next_review_date
                sqlx::query(
                    "UPDATE applications
                     SET status_id = $1,
                         approved_at = CURRENT_TIMESTAMP,
                         is_current_version = TRUE,
                         next_review_date = CURRENT_DATE + (
                             SELECT COALESCE(grf.months_interval, 12) * INTERVAL '1 month'
                             FROM glossary_review_frequencies grf
                             WHERE grf.frequency_id = applications.review_frequency_id
                         ),
                         updated_at = CURRENT_TIMESTAMP
                     WHERE application_id = $2",
                )
                .bind(status_id)
                .bind(instance.entity_id)
                .execute(pool)
                .await?;

                // Version swap: if this is an amendment (has previous_version_id),
                // mark the old version as no longer current.
                let previous_id = sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT previous_version_id FROM applications WHERE application_id = $1",
                )
                .bind(instance.entity_id)
                .fetch_one(pool)
                .await?;

                if let Some(old_app_id) = previous_id {
                    // Mark old version as SUPERSEDED
                    let superseded_status_id = sqlx::query_scalar::<_, Uuid>(
                        "SELECT status_id FROM entity_statuses WHERE status_code = 'SUPERSEDED'",
                    )
                    .fetch_one(pool)
                    .await?;

                    sqlx::query(
                        "UPDATE applications
                         SET is_current_version = FALSE,
                             status_id = $1,
                             updated_at = CURRENT_TIMESTAMP
                         WHERE application_id = $2",
                    )
                    .bind(superseded_status_id)
                    .bind(old_app_id)
                    .execute(pool)
                    .await?;

                    // Also mark the old version's workflow instance as completed
                    sqlx::query(
                        "UPDATE workflow_instances
                         SET completed_at = CURRENT_TIMESTAMP,
                             completion_notes = 'Superseded by newer version',
                             updated_at = CURRENT_TIMESTAMP
                         WHERE entity_id = $1 AND completed_at IS NULL",
                    )
                    .bind(old_app_id)
                    .execute(pool)
                    .await?;

                    tracing::info!(
                        new_app_id = %instance.entity_id,
                        old_app_id = %old_app_id,
                        "version swap: application amendment approved, old version superseded"
                    );
                }
            } else {
                sqlx::query(
                    "UPDATE applications SET status_id = $1, updated_at = CURRENT_TIMESTAMP WHERE application_id = $2",
                )
                .bind(status_id)
                .bind(instance.entity_id)
                .execute(pool)
                .await?;
            }
        }
        "business_processes" => {
            sqlx::query(
                "UPDATE business_processes SET status_id = $1, updated_at = CURRENT_TIMESTAMP WHERE process_id = $2",
            )
            .bind(status_id)
            .bind(instance.entity_id)
            .execute(pool)
            .await?;
        }
        unknown => {
            return Err(AppError::Workflow(format!(
                "unsupported entity table for status update: {unknown}"
            )));
        }
    }

    Ok(())
}

/// Resolve a human-readable name for an entity given its type name and ID.
/// Used to populate `PendingTaskView.entity_name`.
async fn resolve_entity_name(
    pool: &PgPool,
    entity_type_name: &str,
    entity_id: Uuid,
) -> AppResult<String> {
    let name = match entity_type_name {
        "Glossary Term" => {
            sqlx::query_scalar::<_, String>(
                "SELECT term_name FROM glossary_terms WHERE term_id = $1",
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
        }
        "Data Element" => {
            sqlx::query_scalar::<_, String>(
                "SELECT element_name FROM data_elements WHERE element_id = $1",
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
        }
        "Quality Rule" => {
            sqlx::query_scalar::<_, String>(
                "SELECT rule_name FROM quality_rules WHERE rule_id = $1",
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
        }
        "Application" => {
            sqlx::query_scalar::<_, String>(
                "SELECT application_name FROM applications WHERE application_id = $1",
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
        }
        "Business Process" => {
            sqlx::query_scalar::<_, String>(
                "SELECT process_name FROM business_processes WHERE process_id = $1",
            )
            .bind(entity_id)
            .fetch_optional(pool)
            .await?
        }
        _ => None,
    };

    Ok(name.unwrap_or_else(|| format!("(unknown entity {entity_id})")))
}

/// Validate that all mandatory ownership fields are populated before
/// a term can be submitted for review. This is a data governance
/// requirement — every term must have clear accountability before
/// entering the approval workflow.
async fn validate_ownership_before_submit(
    pool: &PgPool,
    instance: &WorkflowInstance,
) -> AppResult<()> {
    // Resolve the entity type
    let entity_type = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM workflow_entity_types WHERE entity_type_id = $1",
    )
    .bind(instance.entity_type_id)
    .fetch_one(pool)
    .await?;

    // Ownership checks per entity type (other types can be added as needed)
    if entity_type.as_str() == "glossary_terms" {
        #[derive(sqlx::FromRow)]
        struct OwnershipCheck {
            owner_user_id: Option<Uuid>,
            steward_user_id: Option<Uuid>,
            domain_owner_user_id: Option<Uuid>,
            approver_user_id: Option<Uuid>,
        }

        let row = sqlx::query_as::<_, OwnershipCheck>(
            "SELECT owner_user_id, steward_user_id, domain_owner_user_id, approver_user_id FROM glossary_terms WHERE term_id = $1 AND deleted_at IS NULL",
        )
        .bind(instance.entity_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("entity not found for ownership check".into()))?;

        let mut missing = Vec::new();
        if row.owner_user_id.is_none() {
            missing.push("Business Term Owner");
        }
        if row.steward_user_id.is_none() {
            missing.push("Data Steward");
        }
        if row.domain_owner_user_id.is_none() {
            missing.push("Data Domain Owner");
        }
        if row.approver_user_id.is_none() {
            missing.push("Approver");
        }

        if !missing.is_empty() {
            return Err(AppError::Validation(format!(
                "cannot submit for review — the following ownership fields must be assigned: {}",
                missing.join(", ")
            )));
        }
    } else if entity_type.as_str() == "applications" {
        #[derive(sqlx::FromRow)]
        struct AppOwnershipCheck {
            business_owner_id: Option<Uuid>,
            technical_owner_id: Option<Uuid>,
            steward_user_id: Option<Uuid>,
            approver_user_id: Option<Uuid>,
        }

        let row = sqlx::query_as::<_, AppOwnershipCheck>(
            "SELECT business_owner_id, technical_owner_id, steward_user_id, approver_user_id FROM applications WHERE application_id = $1 AND deleted_at IS NULL",
        )
        .bind(instance.entity_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("entity not found for ownership check".into()))?;

        let mut missing = Vec::new();
        if row.business_owner_id.is_none() {
            missing.push("Business Owner");
        }
        if row.technical_owner_id.is_none() {
            missing.push("Technical Owner");
        }
        if row.steward_user_id.is_none() {
            missing.push("Data Steward");
        }
        if row.approver_user_id.is_none() {
            missing.push("Approver");
        }

        if !missing.is_empty() {
            return Err(AppError::Validation(format!(
                "cannot submit for review — the following ownership fields must be assigned: {}",
                missing.join(", ")
            )));
        }
    }

    Ok(())
}
