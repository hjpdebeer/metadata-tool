use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::lineage::*;
use crate::error::AppResult;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ImpactAnalysisQuery {
    pub direction: Option<String>,
    pub max_depth: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/graphs",
    responses(
        (status = 200, description = "List lineage graphs", body = Vec<LineageGraph>)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn list_graphs(State(_state): State<AppState>) -> AppResult<Json<Vec<LineageGraph>>> {
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/graphs/{graph_id}",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    responses(
        (status = 200, description = "Full graph for visualization", body = LineageGraphView)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn get_graph(
    State(_state): State<AppState>,
    Path(_graph_id): Path<Uuid>,
) -> AppResult<Json<LineageGraphView>> {
    Err(crate::error::AppError::NotFound("Graph not found".into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/lineage/graphs",
    request_body = CreateLineageGraphRequest,
    responses(
        (status = 201, description = "Graph created", body = LineageGraph)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn create_graph(
    State(_state): State<AppState>,
    Json(_body): Json<CreateLineageGraphRequest>,
) -> AppResult<Json<LineageGraph>> {
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/lineage/graphs/{graph_id}/nodes",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    request_body = AddLineageNodeRequest,
    responses(
        (status = 201, description = "Node added", body = LineageNode)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn add_node(
    State(_state): State<AppState>,
    Path(_graph_id): Path<Uuid>,
    Json(_body): Json<AddLineageNodeRequest>,
) -> AppResult<Json<LineageNode>> {
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/lineage/graphs/{graph_id}/edges",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    request_body = AddLineageEdgeRequest,
    responses(
        (status = 201, description = "Edge added", body = LineageEdge)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn add_edge(
    State(_state): State<AppState>,
    Path(_graph_id): Path<Uuid>,
    Json(_body): Json<AddLineageEdgeRequest>,
) -> AppResult<Json<LineageEdge>> {
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/lineage/impact/{node_id}",
    params(
        ("node_id" = Uuid, Path, description = "Node ID"),
        ImpactAnalysisQuery
    ),
    responses(
        (status = 200, description = "Impact analysis result", body = ImpactAnalysis)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn impact_analysis(
    State(_state): State<AppState>,
    Path(_node_id): Path<Uuid>,
    Query(_params): Query<ImpactAnalysisQuery>,
) -> AppResult<Json<ImpactAnalysis>> {
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}
