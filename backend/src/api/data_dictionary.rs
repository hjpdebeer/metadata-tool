use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::db::AppState;
use crate::domain::data_dictionary::*;
use crate::error::AppResult;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct DataElementQuery {
    pub query: Option<String>,
    pub domain_id: Option<Uuid>,
    pub is_cde: Option<bool>,
    pub status: Option<String>,
    pub glossary_term_id: Option<Uuid>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements",
    params(DataElementQuery),
    responses(
        (status = 200, description = "List data elements", body = Vec<DataElement>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_elements(
    State(_state): State<AppState>,
    axum::extract::Query(_params): axum::extract::Query<DataElementQuery>,
) -> AppResult<Json<Vec<DataElement>>> {
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements/{element_id}",
    params(("element_id" = Uuid, Path, description = "Element ID")),
    responses(
        (status = 200, description = "Full data element view", body = DataElementFullView),
        (status = 404, description = "Element not found")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn get_element(
    State(_state): State<AppState>,
    Path(_element_id): Path<Uuid>,
) -> AppResult<Json<DataElementFullView>> {
    Err(crate::error::AppError::NotFound(
        "Element not found".into(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/data-dictionary/elements",
    request_body = CreateDataElementRequest,
    responses(
        (status = 201, description = "Data element created", body = DataElement),
        (status = 422, description = "Validation or naming standard violation")
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn create_element(
    State(_state): State<AppState>,
    Json(_body): Json<CreateDataElementRequest>,
) -> AppResult<Json<DataElement>> {
    // TODO: Validate naming standards, create element, trigger AI enrichment
    Err(crate::error::AppError::Internal(anyhow::anyhow!(
        "Not implemented yet"
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/elements/cde",
    responses(
        (status = 200, description = "List critical data elements", body = Vec<DataElement>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_cde(State(_state): State<AppState>) -> AppResult<Json<Vec<DataElement>>> {
    // TODO: Return all CDEs with their rationale
    Ok(Json(vec![]))
}

#[utoipa::path(
    get,
    path = "/api/v1/data-dictionary/source-systems",
    responses(
        (status = 200, description = "List source systems", body = Vec<SourceSystem>)
    ),
    security(("bearer_auth" = [])),
    tag = "data_dictionary"
)]
pub async fn list_source_systems(
    State(_state): State<AppState>,
) -> AppResult<Json<Vec<SourceSystem>>> {
    Ok(Json(vec![]))
}
