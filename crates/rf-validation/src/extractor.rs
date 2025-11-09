//! Axum extractors for automatic validation
//!
//! Provides ValidatedJson and ValidatedForm extractors that automatically
//! validate request bodies before passing them to handlers.

use crate::error::ValidationErrors;
use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

/// JSON extractor with automatic validation
///
/// # Example
///
/// ```ignore
/// use rf_validation::{ValidatedJson, Validate};
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, Validate)]
/// struct CreateUser {
///     #[validate(email)]
///     email: String,
///
///     #[validate(length(min = 8))]
///     password: String,
/// }
///
/// async fn create_user(
///     ValidatedJson(user): ValidatedJson<CreateUser>,
/// ) -> String {
///     format!("Created user: {}", user.email)
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ValidationRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Extract JSON body
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|err| ValidationRejection::JsonError(err.to_string()))?;

        // Validate
        value
            .validate()
            .map_err(|e| ValidationRejection::ValidationError(e.into()))?;

        Ok(ValidatedJson(value))
    }
}

/// Validation rejection type
#[derive(Debug)]
pub enum ValidationRejection {
    /// JSON deserialization error
    JsonError(String),
    /// Validation error
    ValidationError(ValidationErrors),
}

impl IntoResponse for ValidationRejection {
    fn into_response(self) -> Response {
        match self {
            ValidationRejection::JsonError(msg) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid JSON",
                    "message": msg,
                })),
            )
                .into_response(),

            ValidationRejection::ValidationError(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "type": "validation-failed",
                    "title": "Validation Failed",
                    "status": 422,
                    "detail": "One or more fields failed validation",
                    "errors": errors.field_errors(),
                })),
            )
                .into_response(),
        }
    }
}

// Integration tests moved to examples/validation-demo for easier testing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_rejection_debug() {
        let rejection = ValidationRejection::JsonError("test error".to_string());
        assert!(format!("{:?}", rejection).contains("JsonError"));
    }
}
