use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LineageGraph {
    pub graph_id: Uuid,
    pub graph_name: String,
    pub graph_type: String, // BUSINESS or TECHNICAL
    pub description: Option<String>,
    pub scope_type: Option<String>,
    pub scope_entity_id: Option<Uuid>,
    pub version_number: i32,
    pub is_current: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LineageNode {
    pub node_id: Uuid,
    pub graph_id: Uuid,
    pub node_type_id: Uuid,
    pub node_name: String,
    pub node_label: Option<String>,
    pub description: Option<String>,
    pub system_id: Option<Uuid>,
    pub table_id: Option<Uuid>,
    pub element_id: Option<Uuid>,
    pub application_id: Option<Uuid>,
    pub process_id: Option<Uuid>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LineageEdge {
    pub edge_id: Uuid,
    pub graph_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub transformation_logic: Option<String>,
    pub description: Option<String>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLineageGraphRequest {
    pub graph_name: String,
    pub graph_type: String,
    pub description: Option<String>,
    pub scope_type: Option<String>,
    pub scope_entity_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddLineageNodeRequest {
    pub node_type_id: Uuid,
    pub node_name: String,
    pub node_label: Option<String>,
    pub description: Option<String>,
    pub system_id: Option<Uuid>,
    pub table_id: Option<Uuid>,
    pub element_id: Option<Uuid>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddLineageEdgeRequest {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub transformation_logic: Option<String>,
    pub description: Option<String>,
}

/// Full graph structure for visualization
#[derive(Debug, Serialize, ToSchema)]
pub struct LineageGraphView {
    #[serde(flatten)]
    pub graph: LineageGraph,
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

/// Impact analysis result
#[derive(Debug, Serialize, ToSchema)]
pub struct ImpactAnalysis {
    pub source_node_id: Uuid,
    pub direction: String, // UPSTREAM or DOWNSTREAM
    pub impacted_nodes: Vec<LineageNode>,
    pub impacted_edges: Vec<LineageEdge>,
    pub depth: i32,
}
