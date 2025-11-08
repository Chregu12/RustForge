//! FormRequest trait - Laravel-style form validation
//!
//! This module provides the FormRequest pattern for automatic validation
//! and authorization of incoming requests.
//!
//! # Example
//!
//! ```
//! use foundry_forms::form_request::{FormRequest, FormRequestValidator};
//! use foundry_forms::validation::{ValidationData, ValidationErrors, required, email, min_length};
//! use std::collections::HashMap;
//!
//! struct CreateUserRequest {
//!     name: String,
//!     email: String,
//!     password: String,
//! }
//!
//! impl FormRequest for CreateUserRequest {
//!     fn rules(&self) -> FormRequestValidator {
//!         FormRequestValidator::new()
//!             .rule("name", vec![required(), min_length(3)])
//!             .rule("email", vec![required(), email()])
//!             .rule("password", vec![required(), min_length(8)])
//!     }
//!
//!     fn authorize(&self) -> bool {
//!         // Add authorization logic here
//!         true
//!     }
//!
//!     fn messages(&self) -> HashMap<String, String> {
//!         let mut messages = HashMap::new();
//!         messages.insert("email".to_string(), "Please provide a valid email address".to_string());
//!         messages
//!     }
//! }
//! ```

use crate::validation::{ValidationData, ValidationErrors, ValidationRuleTrait, Validator};
use std::collections::HashMap;

// ============================================================================
// FormRequest Trait
// ============================================================================

/// Trait for form requests with validation and authorization
pub trait FormRequest {
    /// Define validation rules for the request
    fn rules(&self) -> FormRequestValidator;

    /// Authorize the request (default: true)
    fn authorize(&self) -> bool {
        true
    }

    /// Custom error messages for validation rules
    fn messages(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Validate the request data
    fn validate(&self, data: ValidationData) -> Result<ValidationData, FormRequestError> {
        // Check authorization first
        if !self.authorize() {
            return Err(FormRequestError::Unauthorized);
        }

        // Build validator with rules
        let validator_rules = self.rules();
        let messages = self.messages();

        let mut validator = Validator::new(data);

        // Add rules
        for (field, rules) in validator_rules.rules {
            validator = validator.rule(field, rules);
        }

        // Add custom messages
        for (field, message) in messages {
            validator = validator.message(field, message);
        }

        // Validate
        validator.validate().map_err(FormRequestError::Validation)
    }
}

// ============================================================================
// FormRequestValidator (Builder for rules)
// ============================================================================

/// Builder for defining validation rules in FormRequest
pub struct FormRequestValidator {
    pub rules: HashMap<String, Vec<Box<dyn ValidationRuleTrait>>>,
}

impl FormRequestValidator {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn rule(mut self, field: impl Into<String>, rules: Vec<Box<dyn ValidationRuleTrait>>) -> Self {
        self.rules.insert(field.into(), rules);
        self
    }
}

impl Default for FormRequestValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FormRequest Error
// ============================================================================

/// Errors that can occur during form request processing
#[derive(Debug)]
pub enum FormRequestError {
    /// Validation failed
    Validation(ValidationErrors),
    /// Authorization failed
    Unauthorized,
}

impl std::fmt::Display for FormRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormRequestError::Validation(errors) => write!(f, "Validation failed: {}", errors),
            FormRequestError::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}

impl std::error::Error for FormRequestError {}

// ============================================================================
// Macro for easy FormRequest definition
// ============================================================================

/// Macro to define a FormRequest with validation rules
#[macro_export]
macro_rules! form_request {
    (
        $name:ident {
            $(
                $field:ident: $field_type:ty => [ $($rule:expr),* $(,)? ]
            ),* $(,)?
        }
    ) => {
        pub struct $name {
            $(
                pub $field: $field_type,
            )*
        }

        impl $crate::form_request::FormRequest for $name {
            fn rules(&self) -> $crate::form_request::FormRequestValidator {
                $crate::form_request::FormRequestValidator::new()
                    $(
                        .rule(stringify!($field), vec![$($rule),*])
                    )*
            }
        }
    };
}

// ============================================================================
// Helper for extracting validated data into structs
// ============================================================================

/// Trait for types that can be created from validated data
pub trait FromValidatedData: Sized {
    fn from_validated(data: &ValidationData) -> Result<Self, String>;
}

// ============================================================================
// Async FormRequest Support
// ============================================================================

#[cfg(feature = "async")]
use async_trait::async_trait;

#[cfg(feature = "async")]
#[async_trait]
pub trait AsyncFormRequest {
    /// Define validation rules for the request
    fn rules(&self) -> FormRequestValidator;

    /// Authorize the request (async version)
    async fn authorize(&self) -> bool {
        true
    }

    /// Custom error messages for validation rules
    fn messages(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Validate the request data (async)
    async fn validate(&self, data: ValidationData) -> Result<ValidationData, FormRequestError> {
        // Check authorization first
        if !self.authorize().await {
            return Err(FormRequestError::Unauthorized);
        }

        // Build validator with rules
        let validator_rules = self.rules();
        let messages = self.messages();

        let mut validator = Validator::new(data);

        // Add rules
        for (field, rules) in validator_rules.rules {
            validator = validator.rule(field, rules);
        }

        // Add custom messages
        for (field, message) in messages {
            validator = validator.message(field, message);
        }

        // Validate
        validator.validate().map_err(FormRequestError::Validation)
    }
}
