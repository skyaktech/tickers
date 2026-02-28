use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::Internal(msg) => {
                tracing::error!(error = %msg, "Internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };
        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}
