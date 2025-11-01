//! API response wrappers

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Serialize, Deserialize};

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            data: Some(data),
            message: None,
            meta: None,
        }
    }

    pub fn with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            data: Some(data),
            message: Some(message.into()),
            meta: None,
        }
    }

    pub fn with_meta(data: T, meta: serde_json::Value) -> Self {
        Self {
            data: Some(data),
            message: None,
            meta: Some(meta),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Standard API error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ApiError {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: None,
            details: None,
            code: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn not_found() -> Self {
        Self::new("Not Found").with_message("The requested resource was not found")
    }

    pub fn validation_error(details: serde_json::Value) -> Self {
        Self::new("Validation Error")
            .with_message("The given data was invalid")
            .with_details(details)
    }

    pub fn unauthorized() -> Self {
        Self::new("Unauthorized").with_message("You are not authorized to access this resource")
    }

    pub fn forbidden() -> Self {
        Self::new("Forbidden").with_message("You do not have permission to access this resource")
    }

    pub fn internal_error() -> Self {
        Self::new("Internal Server Error").with_message("An unexpected error occurred")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.error.as_str() {
            "Not Found" => StatusCode::NOT_FOUND,
            "Validation Error" => StatusCode::UNPROCESSABLE_ENTITY,
            "Unauthorized" => StatusCode::UNAUTHORIZED,
            "Forbidden" => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self)).into_response()
    }
}
