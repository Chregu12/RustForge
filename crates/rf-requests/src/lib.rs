//! # rf-requests
//!
//! Form request validation pattern for Rust web applications.
//!
//! ## Features
//!
//! - Form request validation
//! - Authorization in requests
//! - Custom validation rules
//! - After validation hooks
//! - Custom error messages
//!
//! ## Example
//!
//! ```rust
//! use rf_requests::{FormRequest, ValidationRulesBuilder, FormRequestResult};
//! use async_trait::async_trait;
//! use serde::Deserialize;
//! use std::collections::HashMap;
//!
//! #[derive(Debug, Deserialize)]
//! struct CreatePostRequest {
//!     title: String,
//!     content: String,
//! }
//!
//! #[async_trait]
//! impl FormRequest for CreatePostRequest {
//!     async fn authorize(&self) -> FormRequestResult<()> {
//!         // Check if user can create posts
//!         Ok(())
//!     }
//!
//!     fn rules(&self) -> HashMap<String, Vec<rf_requests::ValidationRule>> {
//!         ValidationRulesBuilder::new()
//!             .required("title")
//!             .min_length("title", 3)
//!             .required("content")
//!             .build()
//!     }
//! }
//! ```

pub mod form_request;
pub mod authorization;
pub mod validation;

pub use form_request::{
    FormRequest, FormRequestError, FormRequestResult, ValidationRule, ValidationRulesBuilder,
};
pub use authorization::{
    Authorizable, AuthorizationPolicy, AuthorizationChecker, AuthorizationResult,
};
pub use validation::{
    Validator, ValidationResult, EmailValidator, LengthValidator, UrlValidator, NumericValidator,
    CustomMessages,
};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize)]
    struct TestRequest {
        title: String,
        email: String,
    }

    #[async_trait]
    impl FormRequest for TestRequest {
        fn rules(&self) -> HashMap<String, Vec<ValidationRule>> {
            ValidationRulesBuilder::new()
                .required("title")
                .min_length("title", 3)
                .email("email")
                .build()
        }
    }

    #[tokio::test]
    async fn test_integration_form_request() {
        let request = TestRequest {
            title: "Test Post".to_string(),
            email: "test@example.com".to_string(),
        };

        let result = request.process().await;
        assert!(result.is_ok());
    }
}
