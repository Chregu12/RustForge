//! # rf-validation - Validation & Forms
//!
//! Production-ready validation for web applications built on top of the
//! `validator` crate with Axum integration.
//!
//! ## Features
//!
//! - **Declarative Validation**: Use `#[derive(Validate)]` from validator crate
//! - **30+ Built-in Rules**: Email, URL, length, range, regex, and more
//! - **Axum Integration**: ValidatedJson extractor with automatic validation
//! - **Field-Level Errors**: Detailed error messages per field
//! - **Type-Safe**: Compile-time validation rule checking
//! - **RFC 7807 Compatible**: Standard error responses
//!
//! ## Quick Start
//!
//! ```ignore
//! use rf_validation::{ValidatedJson, Validate};
//! use serde::Deserialize;
//! use axum::{routing::post, Router};
//!
//! #[derive(Debug, Deserialize, Validate)]
//! struct CreateUser {
//!     #[validate(email)]
//!     email: String,
//!
//!     #[validate(length(min = 8, max = 128))]
//!     password: String,
//!
//!     #[validate(length(min = 2, max = 100))]
//!     name: String,
//! }
//!
//! async fn create_user(
//!     ValidatedJson(user): ValidatedJson<CreateUser>,
//! ) -> String {
//!     format!("Created user: {}", user.email)
//! }
//!
//! # async fn example() {
//! let app = Router::new().route("/users", post(create_user));
//! # }
//! ```
//!
//! ## Validation Rules
//!
//! All rules from the `validator` crate are supported:
//!
//! - **email**: Valid email address
//! - **url**: Valid URL
//! - **length(min, max)**: String/collection length
//! - **range(min, max)**: Numeric range
//! - **regex**: Custom regex pattern
//! - **contains**: String contains substring
//! - **custom**: Custom validation function
//! - And many more!
//!
//! ## Error Responses
//!
//! Validation errors are returned as RFC 7807-compatible JSON:
//!
//! ```json
//! {
//!   "type": "validation-failed",
//!   "title": "Validation Failed",
//!   "status": 422,
//!   "detail": "One or more fields failed validation",
//!   "errors": {
//!     "email": [
//!       {
//!         "code": "email",
//!         "message": "Invalid email address"
//!       }
//!     ]
//!   }
//! }
//! ```

pub mod error;
pub mod extractor;

// Re-export main types
pub use error::{FieldError, ValidationErrors};
pub use extractor::{ValidatedJson, ValidationRejection};

// Re-export validator traits and derive macro
pub use validator::Validate;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        error::{FieldError, ValidationErrors},
        extractor::{ValidatedJson, ValidationRejection},
    };
    pub use validator::Validate;
}
