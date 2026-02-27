use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use super::models::ValidationErrorResponse;

#[derive(Debug)]
pub enum AuthError {
    Validation {
        field: &'static str,
        message: &'static str,
    },
    Conflict {
        message: &'static str,
    },
    InvalidCredentials,
    Unauthorized,
    RateLimited,
    BadRequest {
        message: &'static str,
    },
    Internal,
    ServiceUnavailable,
    HttpsRequired,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            Self::Validation { field, message } => (
                StatusCode::BAD_REQUEST,
                Json(ValidationErrorResponse {
                    field: field.to_string(),
                    message: message.to_string(),
                }),
            )
                .into_response(),
            Self::Conflict { message } => {
                (StatusCode::CONFLICT, Json(json!({ "message": message }))).into_response()
            }
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "message": "Email or password is incorrect" })),
            )
                .into_response(),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "message": "Session expired. Please login again" })),
            )
                .into_response(),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({ "message": "Too many login attempts. Please try again later." })),
            )
                .into_response(),
            Self::BadRequest { message } => {
                (StatusCode::BAD_REQUEST, Json(json!({ "message": message }))).into_response()
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "An internal server error occurred" })),
            )
                .into_response(),
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "message": "Authentication service unavailable" })),
            )
                .into_response(),
            Self::HttpsRequired => (
                StatusCode::FORBIDDEN,
                Json(json!({ "message": "HTTPS is required" })),
            )
                .into_response(),
        }
    }
}
