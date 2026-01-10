use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;
use tracing;

#[derive(Debug, Error)]
pub enum CommodityError {
    #[error("Invalid")]
    WrongCredentials(String),

    #[error("Commodity already exists")]
    AlreadyExists,

    // ➡️ FIX: Must carry a value to be specific, matching the correct IntoResponse logic
    #[error("Resource not found.")]
    NotFound(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl IntoResponse for CommodityError {
    fn into_response(self) -> Response {
        let (status_code, client_message) = match self {
            // ✅ FIX: Binds the identifier (id_or_email) to be specific
            CommodityError::NotFound(name) => (
                StatusCode::NOT_FOUND,
                format!("Station with identifier {} not found.", name),
            ),
            CommodityError::WrongCredentials(message) => {
                (StatusCode::UNAUTHORIZED, format!("{message}"))
            }
            CommodityError::AlreadyExists => (
                StatusCode::CONFLICT,
                "Commodity already exists.".to_string(),
            ),

            // 🔒 SECURITY FIX: Log internal error but return generic message
            CommodityError::DatabaseError(err) => {
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

// Helper struct for consistent JSON error responses
#[derive(Serialize)]
struct ApiError {
    message: String,
}
