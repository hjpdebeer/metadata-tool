use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("naming standard violation: {0}")]
    NamingViolation(String),

    #[error("workflow error: {0}")]
    Workflow(String),

    #[error("ai service error: {0}")]
    AiService(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR"),
            AppError::NamingViolation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "NAMING_VIOLATION"),
            AppError::Workflow(_) => (StatusCode::UNPROCESSABLE_ENTITY, "WORKFLOW_ERROR"),
            AppError::AiService(_) => (StatusCode::BAD_GATEWAY, "AI_SERVICE_ERROR"),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR"),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code,
                message: self.to_string(),
            },
        };

        (status, axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
