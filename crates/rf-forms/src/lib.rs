//! Foundry Forms - HTML Form Builder & Validation
//!
//! Provides:
//! - Fluent form builder API
//! - HTML form field generation
//! - Built-in validation rules
//! - CSRF protection
//! - Error display helpers
//! - Multiple themes (Bootstrap, Tailwind)
//!
//! # Example
//!
//! ```no_run
//! use foundry_forms::{Form, Field, Theme, FormMethod};
//!
//! let form = Form::new("user_form")
//!     .action("/users")
//!     .method(FormMethod::Post)
//!     .field(Field::text("name").label("Name").required().build())
//!     .field(Field::email("email").label("Email").required().build())
//!     .field(Field::password("password").label("Password").min_length(8).build())
//!     .submit("Create User")
//!     .build();
//!
//! let html = form.render(Theme::Tailwind)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod builder;
pub mod csrf;
pub mod field;
pub mod form_request;
pub mod renderer;
pub mod theme;
pub mod validation;

pub use builder::{Form, FormBuilder, FormMethod};
pub use csrf::{CsrfProtection, CsrfToken};
pub use field::{Field, FieldType, InputType};
pub use form_request::{FormRequest, FormRequestError, FormRequestValidator, FromValidatedData};
pub use renderer::FormRenderer;
pub use theme::Theme;
pub use validation::{
    ValidationData, ValidationError, ValidationErrors, ValidationRule, ValidationRuleTrait,
    Validator,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Form data after submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormData {
    pub fields: HashMap<String, String>,
    pub files: HashMap<String, Vec<u8>>,
}

impl FormData {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            files: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.fields.get(key)
    }

    pub fn get_file(&self, key: &str) -> Option<&Vec<u8>> {
        self.files.get(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.fields.insert(key.into(), value.into());
    }
}

impl Default for FormData {
    fn default() -> Self {
        Self::new()
    }
}

/// Form errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormErrors {
    pub errors: HashMap<String, Vec<String>>,
}

impl FormErrors {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    pub fn add(&mut self, field: impl Into<String>, error: impl Into<String>) {
        self.errors
            .entry(field.into())
            .or_default()
            .push(error.into());
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn get(&self, field: &str) -> Option<&Vec<String>> {
        self.errors.get(field)
    }
}

impl Default for FormErrors {
    fn default() -> Self {
        Self::new()
    }
}
