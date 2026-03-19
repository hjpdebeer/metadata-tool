use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Core entities (map directly to database tables)
// ---------------------------------------------------------------------------

/// A data lineage graph containing nodes and edges for tracing data flow. Maps to the `lineage_graphs` table.
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

/// A node in a lineage graph representing a data source, transformation, or destination. Maps to the `lineage_nodes` table.
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

/// A directed edge connecting two nodes in a lineage graph, representing data flow. Maps to the `lineage_edges` table.
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

// ---------------------------------------------------------------------------
// List item (for graph listing with aggregate counts)
// ---------------------------------------------------------------------------

/// List view of a lineage graph with node/edge counts and creator name
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct LineageGraphListItem {
    pub graph_id: Uuid,
    pub graph_name: String,
    pub graph_type: String,
    pub description: Option<String>,
    pub is_current: bool,
    pub node_count: i64,
    pub edge_count: i64,
    pub created_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Paginated response
// ---------------------------------------------------------------------------

/// Concrete paginated type for OpenAPI schema generation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedLineageGraphs {
    pub data: Vec<LineageGraphListItem>,
    pub total_count: i64,
    pub page: i64,
    pub page_size: i64,
}

// ---------------------------------------------------------------------------
// Node type (seeded in migration 006)
// ---------------------------------------------------------------------------

/// A classification for lineage nodes (e.g. Database, Application, File). Maps to the `lineage_node_types` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LineageNodeType {
    pub node_type_id: Uuid,
    pub type_code: String,
    pub type_name: String,
    pub description: Option<String>,
    pub icon_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Node view (enriched with node type info for React Flow)
// ---------------------------------------------------------------------------

/// Enriched node view including type info joined from lineage_node_types
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct LineageNodeView {
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
    pub node_type_code: String,
    pub node_type_name: String,
    pub icon_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for creating a new lineage graph.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateLineageGraphRequest {
    pub graph_name: String,
    pub graph_type: String,
    pub description: Option<String>,
    pub scope_type: Option<String>,
    pub scope_entity_id: Option<Uuid>,
}

/// Request body for partially updating a lineage graph.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateLineageGraphRequest {
    pub graph_name: Option<String>,
    pub description: Option<String>,
}

/// Request body for adding a new node to a lineage graph.
#[derive(Debug, Clone, Deserialize, ToSchema)]
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

/// Request body for adding a directed edge between two nodes in a lineage graph.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddLineageEdgeRequest {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub transformation_logic: Option<String>,
    pub description: Option<String>,
}

/// Request body for updating a node's position in the React Flow canvas.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateNodePositionRequest {
    pub node_id: Uuid,
    pub position_x: f64,
    pub position_y: f64,
}

/// Query parameters for searching and filtering lineage graphs with pagination.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct SearchGraphsRequest {
    pub query: Option<String>,
    pub graph_type: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// ---------------------------------------------------------------------------
// Full graph structure for visualization (React Flow)
// ---------------------------------------------------------------------------

/// Full graph structure for visualization
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LineageGraphView {
    #[serde(flatten)]
    pub graph: LineageGraph,
    pub nodes: Vec<LineageNodeView>,
    pub edges: Vec<LineageEdge>,
}

// ---------------------------------------------------------------------------
// Impact analysis
// ---------------------------------------------------------------------------

/// A node in the impact analysis result with its traversal depth
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ImpactedNode {
    pub node_id: Uuid,
    pub graph_id: Uuid,
    pub node_type_id: Uuid,
    pub node_name: String,
    pub node_label: Option<String>,
    pub description: Option<String>,
    pub node_type_code: String,
    pub node_type_name: String,
    pub icon_name: Option<String>,
    pub depth: i32,
}

/// An edge in the impact analysis result with its traversal depth
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ImpactedEdge {
    pub edge_id: Uuid,
    pub graph_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub transformation_logic: Option<String>,
    pub description: Option<String>,
    pub depth: i32,
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ImpactAnalysis {
    pub source_node_id: Uuid,
    pub direction: String, // UPSTREAM or DOWNSTREAM
    pub impacted_nodes: Vec<ImpactedNode>,
    pub impacted_edges: Vec<ImpactedEdge>,
    pub max_depth_reached: i32,
}
