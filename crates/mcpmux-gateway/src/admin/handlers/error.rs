//! Admin API error helpers.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// HTTP error wrapper that always serializes as `{ "error": "<message>" }`.
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    /// Converts bridge errors to HTTP JSON while preserving sentinel strings
    /// like `PORT_IN_USE:<port>:<source>` in the message field.
    pub fn from_bridge(error: anyhow::Error) -> Self {
        Self::internal(error.to_string())
    }
}

/// Shared formatter used by tests to assert sentinel message preservation.
pub fn format_bridge_error_message(error: anyhow::Error) -> String {
    ApiError::from_bridge(error).message
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
