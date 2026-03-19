use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::auth::Claims;
use crate::db::AppState;
use crate::domain::lineage::*;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Query parameter types
// ---------------------------------------------------------------------------

/// Query parameters for impact analysis traversal direction and depth.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ImpactAnalysisQuery {
    /// Direction: UPSTREAM or DOWNSTREAM
    pub direction: Option<String>,
    /// Maximum traversal depth (default 10, max 50)
    pub max_depth: Option<i32>,
}

/// Query parameters for filtering lineage graphs by type.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListGraphsQuery {
    /// Filter by graph type: BUSINESS or TECHNICAL
    pub graph_type: Option<String>,
}

// ---------------------------------------------------------------------------
// list_graphs -- GET /api/v1/lineage/graphs
// ---------------------------------------------------------------------------

/// List lineage graphs with node/edge counts, optionally filtered by graph type.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/lineage/graphs",
    params(ListGraphsQuery),
    responses(
        (status = 200, description = "List lineage graphs with counts",
         body = Vec<LineageGraphListItem>)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn list_graphs(
    State(state): State<AppState>,
    Query(params): Query<ListGraphsQuery>,
) -> AppResult<Json<Vec<LineageGraphListItem>>> {
    let graphs = sqlx::query_as::<_, LineageGraphListItem>(
        r#"
        SELECT
            g.graph_id,
            g.graph_name,
            g.graph_type,
            g.description,
            g.is_current,
            (SELECT COUNT(*) FROM lineage_nodes n WHERE n.graph_id = g.graph_id)  AS node_count,
            (SELECT COUNT(*) FROM lineage_edges e WHERE e.graph_id = g.graph_id)  AS edge_count,
            u.display_name AS created_by_name,
            g.created_at,
            g.updated_at
        FROM lineage_graphs g
        LEFT JOIN users u ON u.user_id = g.created_by
        WHERE g.deleted_at IS NULL
          AND ($1::TEXT IS NULL OR g.graph_type = $1)
        ORDER BY g.graph_name ASC
        "#,
    )
    .bind(params.graph_type.as_deref())
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(graphs))
}

// ---------------------------------------------------------------------------
// get_graph -- GET /api/v1/lineage/graphs/{graph_id}
// ---------------------------------------------------------------------------

/// Retrieve a full lineage graph with all nodes and edges for React Flow visualization.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/lineage/graphs/{graph_id}",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    responses(
        (status = 200, description = "Full graph for visualization", body = LineageGraphView),
        (status = 404, description = "Graph not found")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn get_graph(
    State(state): State<AppState>,
    Path(graph_id): Path<Uuid>,
) -> AppResult<Json<LineageGraphView>> {
    // Fetch the graph record
    let graph = sqlx::query_as::<_, LineageGraph>(
        r#"
        SELECT
            graph_id, graph_name, graph_type, description,
            scope_type, scope_entity_id, version_number, is_current,
            created_by, created_at, updated_at
        FROM lineage_graphs
        WHERE graph_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(graph_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("lineage graph not found: {graph_id}")))?;

    // Fetch all nodes with type info for React Flow
    let nodes = sqlx::query_as::<_, LineageNodeView>(
        r#"
        SELECT
            n.node_id,
            n.graph_id,
            n.node_type_id,
            n.node_name,
            n.node_label,
            n.description,
            n.system_id,
            n.table_id,
            n.element_id,
            n.application_id,
            n.process_id,
            n.position_x,
            n.position_y,
            n.properties,
            nt.type_code  AS node_type_code,
            nt.type_name  AS node_type_name,
            nt.icon_name
        FROM lineage_nodes n
        JOIN lineage_node_types nt ON nt.node_type_id = n.node_type_id
        WHERE n.graph_id = $1
        ORDER BY n.node_name ASC
        "#,
    )
    .bind(graph_id)
    .fetch_all(&state.pool)
    .await?;

    // Fetch all edges for the graph
    let edges = sqlx::query_as::<_, LineageEdge>(
        r#"
        SELECT
            edge_id, graph_id, source_node_id, target_node_id,
            edge_type, transformation_logic, description, properties
        FROM lineage_edges
        WHERE graph_id = $1
        "#,
    )
    .bind(graph_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(LineageGraphView {
        graph,
        nodes,
        edges,
    }))
}

// ---------------------------------------------------------------------------
// create_graph -- POST /api/v1/lineage/graphs
// ---------------------------------------------------------------------------

/// Create a new lineage graph (BUSINESS or TECHNICAL type).
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/lineage/graphs",
    request_body = CreateLineageGraphRequest,
    responses(
        (status = 201, description = "Graph created", body = LineageGraph),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn create_graph(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateLineageGraphRequest>,
) -> AppResult<(StatusCode, Json<LineageGraph>)> {
    // Validate required fields
    let graph_name = body.graph_name.trim().to_string();
    if graph_name.is_empty() {
        return Err(AppError::Validation("graph_name is required".into()));
    }

    // Validate graph_type
    let graph_type = body.graph_type.trim().to_string();
    if !["BUSINESS", "TECHNICAL"].contains(&graph_type.as_str()) {
        return Err(AppError::Validation(
            "graph_type must be BUSINESS or TECHNICAL".into(),
        ));
    }

    let graph = sqlx::query_as::<_, LineageGraph>(
        r#"
        INSERT INTO lineage_graphs (
            graph_name, graph_type, description,
            scope_type, scope_entity_id,
            version_number, is_current, created_by
        )
        VALUES ($1, $2, $3, $4, $5, 1, TRUE, $6)
        RETURNING
            graph_id, graph_name, graph_type, description,
            scope_type, scope_entity_id, version_number, is_current,
            created_by, created_at, updated_at
        "#,
    )
    .bind(&graph_name)
    .bind(&graph_type)
    .bind(body.description.as_deref())
    .bind(body.scope_type.as_deref())
    .bind(body.scope_entity_id)
    .bind(claims.sub)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(graph)))
}

// ---------------------------------------------------------------------------
// update_graph -- PUT /api/v1/lineage/graphs/{graph_id}
// ---------------------------------------------------------------------------

/// Update a lineage graph's name or description.
/// Requires authentication.
#[utoipa::path(
    put,
    path = "/api/v1/lineage/graphs/{graph_id}",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    request_body = UpdateLineageGraphRequest,
    responses(
        (status = 200, description = "Graph updated", body = LineageGraph),
        (status = 404, description = "Graph not found")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn update_graph(
    State(state): State<AppState>,
    Path(graph_id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateLineageGraphRequest>,
) -> AppResult<Json<LineageGraph>> {
    // Verify the graph exists and is not deleted
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM lineage_graphs WHERE graph_id = $1 AND deleted_at IS NULL)",
    )
    .bind(graph_id)
    .fetch_one(&state.pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound(format!(
            "lineage graph not found: {graph_id}"
        )));
    }

    let graph = sqlx::query_as::<_, LineageGraph>(
        r#"
        UPDATE lineage_graphs
        SET graph_name  = COALESCE($1, graph_name),
            description = COALESCE($2, description),
            updated_by  = $3,
            updated_at  = CURRENT_TIMESTAMP
        WHERE graph_id = $4 AND deleted_at IS NULL
        RETURNING
            graph_id, graph_name, graph_type, description,
            scope_type, scope_entity_id, version_number, is_current,
            created_by, created_at, updated_at
        "#,
    )
    .bind(body.graph_name.as_deref())
    .bind(body.description.as_deref())
    .bind(claims.sub)
    .bind(graph_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(graph))
}

// ---------------------------------------------------------------------------
// add_node -- POST /api/v1/lineage/graphs/{graph_id}/nodes
// ---------------------------------------------------------------------------

/// Add a new node to a lineage graph.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/lineage/graphs/{graph_id}/nodes",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    request_body = AddLineageNodeRequest,
    responses(
        (status = 201, description = "Node added", body = LineageNode),
        (status = 404, description = "Graph not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn add_node(
    State(state): State<AppState>,
    Path(graph_id): Path<Uuid>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<AddLineageNodeRequest>,
) -> AppResult<(StatusCode, Json<LineageNode>)> {
    // Verify graph exists
    let graph_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM lineage_graphs WHERE graph_id = $1 AND deleted_at IS NULL)",
    )
    .bind(graph_id)
    .fetch_one(&state.pool)
    .await?;

    if !graph_exists {
        return Err(AppError::NotFound(format!(
            "lineage graph not found: {graph_id}"
        )));
    }

    // Validate required fields
    let node_name = body.node_name.trim().to_string();
    if node_name.is_empty() {
        return Err(AppError::Validation("node_name is required".into()));
    }

    let node = sqlx::query_as::<_, LineageNode>(
        r#"
        INSERT INTO lineage_nodes (
            graph_id, node_type_id, node_name, node_label, description,
            system_id, table_id, element_id,
            position_x, position_y, properties
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING
            node_id, graph_id, node_type_id, node_name, node_label, description,
            system_id, table_id, element_id, application_id, process_id,
            position_x, position_y, properties
        "#,
    )
    .bind(graph_id)
    .bind(body.node_type_id)
    .bind(&node_name)
    .bind(body.node_label.as_deref())
    .bind(body.description.as_deref())
    .bind(body.system_id)
    .bind(body.table_id)
    .bind(body.element_id)
    .bind(body.position_x)
    .bind(body.position_y)
    .bind(&body.properties)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(node)))
}

// ---------------------------------------------------------------------------
// update_node_position -- PUT /api/v1/lineage/nodes/{node_id}/position
// ---------------------------------------------------------------------------

/// Update a node's X/Y position for the React Flow canvas.
/// Requires authentication.
#[utoipa::path(
    put,
    path = "/api/v1/lineage/nodes/{node_id}/position",
    params(("node_id" = Uuid, Path, description = "Node ID")),
    request_body = UpdateNodePositionRequest,
    responses(
        (status = 200, description = "Node position updated"),
        (status = 404, description = "Node not found")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn update_node_position(
    State(state): State<AppState>,
    Path(node_id): Path<Uuid>,
    Json(body): Json<UpdateNodePositionRequest>,
) -> AppResult<StatusCode> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE lineage_nodes
        SET position_x  = $1,
            position_y  = $2,
            updated_at  = CURRENT_TIMESTAMP
        WHERE node_id = $3
        "#,
    )
    .bind(body.position_x)
    .bind(body.position_y)
    .bind(node_id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "lineage node not found: {node_id}"
        )));
    }

    Ok(StatusCode::OK)
}

// ---------------------------------------------------------------------------
// delete_node -- DELETE /api/v1/lineage/nodes/{node_id}
// ---------------------------------------------------------------------------

/// Delete a lineage node. Connected edges are removed via CASCADE.
/// Requires authentication.
#[utoipa::path(
    delete,
    path = "/api/v1/lineage/nodes/{node_id}",
    params(("node_id" = Uuid, Path, description = "Node ID")),
    responses(
        (status = 204, description = "Node deleted (edges cascade)"),
        (status = 404, description = "Node not found")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn delete_node(
    State(state): State<AppState>,
    Path(node_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Delete node; connected edges are removed via ON DELETE CASCADE
    let rows_affected = sqlx::query("DELETE FROM lineage_nodes WHERE node_id = $1")
        .bind(node_id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "lineage node not found: {node_id}"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// add_edge -- POST /api/v1/lineage/graphs/{graph_id}/edges
// ---------------------------------------------------------------------------

/// Add a directed edge between two nodes in a lineage graph.
/// Requires authentication.
#[utoipa::path(
    post,
    path = "/api/v1/lineage/graphs/{graph_id}/edges",
    params(("graph_id" = Uuid, Path, description = "Graph ID")),
    request_body = AddLineageEdgeRequest,
    responses(
        (status = 201, description = "Edge added", body = LineageEdge),
        (status = 404, description = "Graph or node not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn add_edge(
    State(state): State<AppState>,
    Path(graph_id): Path<Uuid>,
    Extension(_claims): Extension<Claims>,
    Json(body): Json<AddLineageEdgeRequest>,
) -> AppResult<(StatusCode, Json<LineageEdge>)> {
    // Verify graph exists
    let graph_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM lineage_graphs WHERE graph_id = $1 AND deleted_at IS NULL)",
    )
    .bind(graph_id)
    .fetch_one(&state.pool)
    .await?;

    if !graph_exists {
        return Err(AppError::NotFound(format!(
            "lineage graph not found: {graph_id}"
        )));
    }

    // Verify source node exists and belongs to this graph
    let source_belongs = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM lineage_nodes WHERE node_id = $1 AND graph_id = $2)",
    )
    .bind(body.source_node_id)
    .bind(graph_id)
    .fetch_one(&state.pool)
    .await?;

    if !source_belongs {
        return Err(AppError::NotFound(format!(
            "source node {} not found in graph {graph_id}",
            body.source_node_id
        )));
    }

    // Verify target node exists and belongs to this graph
    let target_belongs = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM lineage_nodes WHERE node_id = $1 AND graph_id = $2)",
    )
    .bind(body.target_node_id)
    .bind(graph_id)
    .fetch_one(&state.pool)
    .await?;

    if !target_belongs {
        return Err(AppError::NotFound(format!(
            "target node {} not found in graph {graph_id}",
            body.target_node_id
        )));
    }

    // Self-loops are prevented by the CHECK constraint in the schema
    if body.source_node_id == body.target_node_id {
        return Err(AppError::Validation(
            "source_node_id and target_node_id must be different".into(),
        ));
    }

    let edge = sqlx::query_as::<_, LineageEdge>(
        r#"
        INSERT INTO lineage_edges (
            graph_id, source_node_id, target_node_id,
            edge_type, transformation_logic, description
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            edge_id, graph_id, source_node_id, target_node_id,
            edge_type, transformation_logic, description, properties
        "#,
    )
    .bind(graph_id)
    .bind(body.source_node_id)
    .bind(body.target_node_id)
    .bind(&body.edge_type)
    .bind(body.transformation_logic.as_deref())
    .bind(body.description.as_deref())
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(edge)))
}

// ---------------------------------------------------------------------------
// delete_edge -- DELETE /api/v1/lineage/edges/{edge_id}
// ---------------------------------------------------------------------------

/// Delete a lineage edge.
/// Requires authentication.
#[utoipa::path(
    delete,
    path = "/api/v1/lineage/edges/{edge_id}",
    params(("edge_id" = Uuid, Path, description = "Edge ID")),
    responses(
        (status = 204, description = "Edge deleted"),
        (status = 404, description = "Edge not found")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn delete_edge(
    State(state): State<AppState>,
    Path(edge_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let rows_affected = sqlx::query("DELETE FROM lineage_edges WHERE edge_id = $1")
        .bind(edge_id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "lineage edge not found: {edge_id}"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// list_node_types -- GET /api/v1/lineage/node-types
// ---------------------------------------------------------------------------

/// List available lineage node types (seeded in migration 006).
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/lineage/node-types",
    responses(
        (status = 200, description = "List lineage node types",
         body = Vec<LineageNodeType>)
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn list_node_types(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<LineageNodeType>>> {
    let node_types = sqlx::query_as::<_, LineageNodeType>(
        r#"
        SELECT node_type_id, type_code, type_name, description, icon_name
        FROM lineage_node_types
        ORDER BY type_name ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(node_types))
}

// ---------------------------------------------------------------------------
// impact_analysis -- GET /api/v1/lineage/impact/{node_id}
// ---------------------------------------------------------------------------

/// Perform upstream or downstream impact analysis from a starting node using recursive CTE traversal.
/// Requires authentication.
#[utoipa::path(
    get,
    path = "/api/v1/lineage/impact/{node_id}",
    params(
        ("node_id" = Uuid, Path, description = "Starting node ID"),
        ImpactAnalysisQuery
    ),
    responses(
        (status = 200, description = "Impact analysis result", body = ImpactAnalysis),
        (status = 404, description = "Node not found"),
        (status = 422, description = "Validation error")
    ),
    security(("bearer_auth" = [])),
    tag = "lineage"
)]
pub async fn impact_analysis(
    State(state): State<AppState>,
    Path(node_id): Path<Uuid>,
    Query(params): Query<ImpactAnalysisQuery>,
) -> AppResult<Json<ImpactAnalysis>> {
    // Validate and default parameters
    let direction = params
        .direction
        .as_deref()
        .unwrap_or("DOWNSTREAM")
        .to_uppercase();
    if !["UPSTREAM", "DOWNSTREAM"].contains(&direction.as_str()) {
        return Err(AppError::Validation(
            "direction must be UPSTREAM or DOWNSTREAM".into(),
        ));
    }

    let max_depth = params.max_depth.unwrap_or(10).clamp(1, 50);

    // Verify the starting node exists
    let node_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM lineage_nodes WHERE node_id = $1)",
    )
    .bind(node_id)
    .fetch_one(&state.pool)
    .await?;

    if !node_exists {
        return Err(AppError::NotFound(format!(
            "lineage node not found: {node_id}"
        )));
    }

    // Run recursive CTE based on direction
    let (impacted_nodes, impacted_edges) = if direction == "DOWNSTREAM" {
        let nodes = sqlx::query_as::<_, ImpactedNode>(
            r#"
            WITH RECURSIVE traversal AS (
                -- Base case: edges originating from the start node
                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    1 AS depth
                FROM lineage_edges e
                WHERE e.source_node_id = $1

                UNION ALL

                -- Recursive case: follow edges forward
                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    t.depth + 1
                FROM lineage_edges e
                JOIN traversal t ON e.source_node_id = t.target_node_id
                WHERE t.depth < $2
            )
            SELECT DISTINCT
                n.node_id,
                n.graph_id,
                n.node_type_id,
                n.node_name,
                n.node_label,
                n.description,
                nt.type_code  AS node_type_code,
                nt.type_name  AS node_type_name,
                nt.icon_name,
                t.min_depth   AS depth
            FROM (
                SELECT target_node_id, MIN(depth) AS min_depth
                FROM traversal
                GROUP BY target_node_id
            ) t
            JOIN lineage_nodes n ON n.node_id = t.target_node_id
            JOIN lineage_node_types nt ON nt.node_type_id = n.node_type_id
            ORDER BY t.min_depth ASC, n.node_name ASC
            "#,
        )
        .bind(node_id)
        .bind(max_depth)
        .fetch_all(&state.pool)
        .await?;

        let edges = sqlx::query_as::<_, ImpactedEdge>(
            r#"
            WITH RECURSIVE traversal AS (
                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    e.graph_id,
                    e.edge_type,
                    e.transformation_logic,
                    e.description,
                    1 AS depth
                FROM lineage_edges e
                WHERE e.source_node_id = $1

                UNION ALL

                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    e.graph_id,
                    e.edge_type,
                    e.transformation_logic,
                    e.description,
                    t.depth + 1
                FROM lineage_edges e
                JOIN traversal t ON e.source_node_id = t.target_node_id
                WHERE t.depth < $2
            )
            SELECT DISTINCT ON (edge_id)
                edge_id,
                graph_id,
                source_node_id,
                target_node_id,
                edge_type,
                transformation_logic,
                description,
                depth
            FROM traversal
            ORDER BY edge_id, depth ASC
            "#,
        )
        .bind(node_id)
        .bind(max_depth)
        .fetch_all(&state.pool)
        .await?;

        (nodes, edges)
    } else {
        // UPSTREAM: follow edges backwards (target -> source)
        let nodes = sqlx::query_as::<_, ImpactedNode>(
            r#"
            WITH RECURSIVE traversal AS (
                -- Base case: edges pointing to the start node
                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    1 AS depth
                FROM lineage_edges e
                WHERE e.target_node_id = $1

                UNION ALL

                -- Recursive case: follow edges backward
                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    t.depth + 1
                FROM lineage_edges e
                JOIN traversal t ON e.target_node_id = t.source_node_id
                WHERE t.depth < $2
            )
            SELECT DISTINCT
                n.node_id,
                n.graph_id,
                n.node_type_id,
                n.node_name,
                n.node_label,
                n.description,
                nt.type_code  AS node_type_code,
                nt.type_name  AS node_type_name,
                nt.icon_name,
                t.min_depth   AS depth
            FROM (
                SELECT source_node_id, MIN(depth) AS min_depth
                FROM traversal
                GROUP BY source_node_id
            ) t
            JOIN lineage_nodes n ON n.node_id = t.source_node_id
            JOIN lineage_node_types nt ON nt.node_type_id = n.node_type_id
            ORDER BY t.min_depth ASC, n.node_name ASC
            "#,
        )
        .bind(node_id)
        .bind(max_depth)
        .fetch_all(&state.pool)
        .await?;

        let edges = sqlx::query_as::<_, ImpactedEdge>(
            r#"
            WITH RECURSIVE traversal AS (
                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    e.graph_id,
                    e.edge_type,
                    e.transformation_logic,
                    e.description,
                    1 AS depth
                FROM lineage_edges e
                WHERE e.target_node_id = $1

                UNION ALL

                SELECT
                    e.source_node_id,
                    e.target_node_id,
                    e.edge_id,
                    e.graph_id,
                    e.edge_type,
                    e.transformation_logic,
                    e.description,
                    t.depth + 1
                FROM lineage_edges e
                JOIN traversal t ON e.target_node_id = t.source_node_id
                WHERE t.depth < $2
            )
            SELECT DISTINCT ON (edge_id)
                edge_id,
                graph_id,
                source_node_id,
                target_node_id,
                edge_type,
                transformation_logic,
                description,
                depth
            FROM traversal
            ORDER BY edge_id, depth ASC
            "#,
        )
        .bind(node_id)
        .bind(max_depth)
        .fetch_all(&state.pool)
        .await?;

        (nodes, edges)
    };

    let max_depth_reached = impacted_nodes
        .iter()
        .map(|n| n.depth)
        .max()
        .unwrap_or(0);

    Ok(Json(ImpactAnalysis {
        source_node_id: node_id,
        direction,
        impacted_nodes,
        impacted_edges,
        max_depth_reached,
    }))
}
