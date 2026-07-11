//! Form Request Validation
//!
//! Provides Laravel-like form requests with automatic validation and authorization.

use crate::error::ValidationErrors;
use crate::validator::Rule;
use async_trait::async_trait;
use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

/// Result type for form request validation
pub type FormRequestResult<T> = Result<T, FormRequestError>;

/// Errors that can occur during form request validation
#[derive(Debug)]
pub enum FormRequestError {
    /// Validation failed
    ValidationFailed(ValidationErrors),
    /// Authorization failed
    Unauthorized,
    /// Failed to parse request body
    InvalidBody(String),
    /// Missing required data
    MissingData(String),
}

impl IntoResponse for FormRequestError {
    fn into_response(self) -> Response {
        match self {
            FormRequestError::ValidationFailed(errors) => {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(errors)).into_response()
            }
            FormRequestError::Unauthorized => {
                (StatusCode::FORBIDDEN, "Unauthorized").into_response()
            }
            FormRequestError::InvalidBody(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            FormRequestError::MissingData(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
        }
    }
}

/// Validation rules for form requests
pub type ValidationRules = HashMap<&'static str, Vec<Box<dyn Rule>>>;

/// Custom validation messages
pub type ValidationMessages = HashMap<&'static str, &'static str>;

/// Trait for form requests with validation and authorization
#[async_trait]
pub trait FormRequest: DeserializeOwned + Send + Sized {
    /// The validated data type (usually Self)
    type Validated: Send;

    /// Define validation rules
    fn rules(&self) -> ValidationRules;

    /// Custom validation messages (optional)
    fn messages(&self) -> ValidationMessages {
        HashMap::new()
    }

    /// Authorize the request (optional)
    fn authorize(&self) -> bool {
        true
    }

    /// Perform the validation
    async fn validate(self) -> FormRequestResult<Self::Validated>;

    /// Prepare for validation (hook for modifications)
    fn prepare_for_validation(&mut self) {}

    /// Pass validation (hook after successful validation)
    fn pass_validation(&mut self) {}

    /// Fail validation (hook for custom error handling)
    fn fail_validation(&self, errors: ValidationErrors) -> FormRequestError {
        FormRequestError::ValidationFailed(errors)
    }
}

/// Validated form request extractor
///
/// Automatically validates and authorizes form requests.
///
/// # Example
///
/// ```ignore
/// use rf_validation::{FormRequest, Validated};
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct CreateUserRequest {
///     email: String,
///     password: String,
///     name: String,
/// }
///
/// impl FormRequest for CreateUserRequest {
///     type Validated = Self;
///
///     fn rules(&self) -> ValidationRules {
///         // Define validation rules
///         HashMap::new()
///     }
///
///     async fn validate(self) -> FormRequestResult<Self::Validated> {
///         Ok(self)
///     }
/// }
///
/// async fn create_user(
///     Validated(request): Validated<CreateUserRequest>,
/// ) -> String {
///     format!("Created user: {}", request.email)
/// }
/// ```
pub struct Validated<T: FormRequest>(pub T::Validated);

impl<T, S> FromRequest<S> for Validated<T>
where
    T: FormRequest + 'static,
    S: Send + Sync,
{
    type Rejection = FormRequestError;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract JSON body
        let bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
            .await
            .map_err(|e| FormRequestError::InvalidBody(e.to_string()))?;

        // Deserialize to form request type
        let mut form_request: T = serde_json::from_slice(&bytes)
            .map_err(|e| FormRequestError::InvalidBody(e.to_string()))?;

        // Check authorization
        if !form_request.authorize() {
            return Err(FormRequestError::Unauthorized);
        }

        // Prepare for validation
        form_request.prepare_for_validation();

        // Validate
        let validated = form_request.validate().await?;

        Ok(Validated(validated))
    }
}

/// Helper to build validation rules fluently
pub struct RulesBuilder {
    rules: ValidationRules,
}

impl RulesBuilder {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn add(mut self, field: &'static str, rules: Vec<Box<dyn Rule>>) -> Self {
        self.rules.insert(field, rules);
        self
    }

    pub fn build(self) -> ValidationRules {
        self.rules
    }
}

impl Default for RulesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to build custom messages fluently
pub struct MessagesBuilder {
    messages: ValidationMessages,
}

impl MessagesBuilder {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
        }
    }

    pub fn add(mut self, key: &'static str, message: &'static str) -> Self {
        self.messages.insert(key, message);
        self
    }

    pub fn build(self) -> ValidationMessages {
        self.messages
    }
}

impl Default for MessagesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RequiredRule;

    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)] // deserialize-only fixture; fields are populated, not read
    struct TestRequest {
        email: String,
        name: String,
    }

    #[async_trait]
    impl FormRequest for TestRequest {
        type Validated = Self;

        fn rules(&self) -> ValidationRules {
            let mut rules = HashMap::new();
            rules.insert("email", vec![Box::new(RequiredRule) as Box<dyn Rule>]);
            rules
        }

        async fn validate(self) -> FormRequestResult<Self::Validated> {
            // Simple pass-through for testing
            Ok(self)
        }
    }

    #[test]
    fn test_rules_builder() {
        let rules = RulesBuilder::new()
            .add("email", vec![Box::new(RequiredRule) as Box<dyn Rule>])
            .build();

        assert_eq!(rules.len(), 1);
        assert!(rules.contains_key("email"));
    }

    #[test]
    fn test_messages_builder() {
        let messages = MessagesBuilder::new()
            .add("email.required", "Email is required")
            .add("name.required", "Name is required")
            .build();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages.get("email.required"), Some(&"Email is required"));
    }

    #[tokio::test]
    async fn test_form_request_validation() {
        let request = TestRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        let result = request.validate().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_form_request_authorization() {
        let request = TestRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        assert!(request.authorize());
    }
}
