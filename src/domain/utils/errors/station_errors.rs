use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use tracing;

#[derive(Debug, Error)]
pub enum StationError {
    #[error("Invalid")]
    WrongCredentials(String),

    #[error("Station already exists")]
    AlreadyExists,

    // ➡️ FIX: Must carry a value to be specific, matching the correct IntoResponse logic
    #[error("Resource not found.")]
    NotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

// Helper struct for consistent JSON error responses
#[derive(Serialize)]
struct ApiError {
    message: String,
}

impl IntoResponse for StationError {
    fn into_response(self) -> Response {
        let (status_code, client_message) = match self {
            // ✅ FIX: Binds the identifier (id_or_email) to be specific
            StationError::NotFound(id_or_email) => (
                StatusCode::NOT_FOUND,
                format!("Station with identifier {} not found.", id_or_email),
            ),
            StationError::AlreadyExists => {
                (StatusCode::CONFLICT, "Station already exists.".to_string())
            }
            StationError::WrongCredentials(message) => {
                (StatusCode::UNAUTHORIZED, format!("Invalid: {message} "))
            }
            // 🔒 SECURITY FIX: Log internal error but return generic message
            StationError::DatabaseError(err) => {
                tracing::error!("Database Error Occurred: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error.".to_string(),
                )
            }
        };

        // Return the structured JSON error response
        (
            status_code,
            Json(ApiError {
                message: client_message,
            }),
        )
            .into_response()
    }
}
