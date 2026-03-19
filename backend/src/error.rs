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
        let (status, code, message) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", self.to_string()),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string()),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT", self.to_string()),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", self.to_string()),
            AppError::NamingViolation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "NAMING_VIOLATION", self.to_string()),
            AppError::Workflow(_) => (StatusCode::UNPROCESSABLE_ENTITY, "WORKFLOW_ERROR", self.to_string()),
            AppError::AiService(_) => (StatusCode::BAD_GATEWAY, "AI_SERVICE_ERROR", self.to_string()),
            // SEC-012: Log full error server-side, return generic message to client
            AppError::Database(e) => {
                tracing::error!(error = %e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "a database error occurred".to_string())
            }
            AppError::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "an internal error occurred".to_string())
            }
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code,
                message,
            },
        };

        (status, axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
