use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkflowDefinition {
    pub workflow_def_id: Uuid,
    pub entity_type_id: Uuid,
    pub workflow_name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub review_sla_hours: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkflowState {
    pub state_id: Uuid,
    pub state_code: String,
    pub state_name: String,
    pub description: Option<String>,
    pub is_initial: bool,
    pub is_terminal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkflowInstance {
    pub instance_id: Uuid,
    pub workflow_def_id: Uuid,
    pub entity_type_id: Uuid,
    pub entity_id: Uuid,
    pub current_state_id: Uuid,
    pub initiated_by: Uuid,
    pub initiated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completion_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkflowTask {
    pub task_id: Uuid,
    pub instance_id: Uuid,
    pub task_type: String,
    pub task_name: String,
    pub description: Option<String>,
    pub assigned_to_user_id: Option<Uuid>,
    pub assigned_to_role_id: Option<Uuid>,
    pub status: String,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<Uuid>,
    pub decision: Option<String>,
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WorkflowTransitionRequest {
    pub action: String, // SUBMIT, APPROVE, REJECT, REVISE, WITHDRAW
    pub comments: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteTaskRequest {
    pub decision: String, // APPROVE, REJECT, REVISE
    pub comments: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkflowInstanceView {
    #[serde(flatten)]
    pub instance: WorkflowInstance,
    pub current_state_name: String,
    pub entity_type_name: String,
    pub initiated_by_name: String,
    pub tasks: Vec<WorkflowTask>,
    pub history: Vec<WorkflowHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkflowHistoryEntry {
    pub history_id: Uuid,
    pub instance_id: Uuid,
    pub from_state_id: Uuid,
    pub to_state_id: Uuid,
    pub action: String,
    pub performed_by: Uuid,
    pub performed_at: DateTime<Utc>,
    pub comments: Option<String>,
}

/// Pending tasks for the current user's dashboard
#[derive(Debug, Serialize, ToSchema)]
pub struct PendingTaskView {
    pub task: WorkflowTask,
    pub entity_type: String,
    pub entity_name: String,
    pub entity_id: Uuid,
    pub workflow_name: String,
    pub submitted_by: String,
    pub submitted_at: DateTime<Utc>,
}
