//! Validation error types

use rf_core::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation errors container
///
/// Contains validation errors grouped by field name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationErrors {
    /// Field errors mapped by field name
    pub errors: HashMap<String, Vec<FieldError>>,
}

impl ValidationErrors {
    /// Create a new validation errors container
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    /// Add an error for a specific field
    pub fn add(&mut self, field: impl Into<String>, error: FieldError) {
        self.errors
            .entry(field.into())
            .or_insert_with(Vec::new)
            .push(error);
    }

    /// Check if there are any errors
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get errors for a specific field
    pub fn get(&self, field: &str) -> Option<&Vec<FieldError>> {
        self.errors.get(field)
    }

    /// Get all field errors
    pub fn field_errors(&self) -> &HashMap<String, Vec<FieldError>> {
        &self.errors
    }
}

impl Default for ValidationErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// Single field validation error
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldError {
    /// Error code (e.g., "email", "length", "required")
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Optional parameters (e.g., min/max values)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

impl FieldError {
    /// Create a new field error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            params: None,
        }
    }

    /// Add a parameter to the error
    pub fn with_param(
        mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> Self {
        if self.params.is_none() {
            self.params = Some(HashMap::new());
        }

        if let Ok(json_value) = serde_json::to_value(value) {
            self.params.as_mut().unwrap().insert(key.into(), json_value);
        }

        self
    }
}

/// Convert validator::ValidationErrors to our ValidationErrors
impl From<validator::ValidationErrors> for ValidationErrors {
    fn from(errors: validator::ValidationErrors) -> Self {
        let mut validation_errors = ValidationErrors::new();

        for (field, field_errors) in errors.field_errors() {
            for error in field_errors {
                let mut field_error = FieldError::new(
                    error.code.to_string(),
                    error.message.as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| format!("Validation failed for field '{}'", field)),
                );

                // validator's params are HashMap<Cow<str>, Value>, convert to HashMap<String, Value>
                if !error.params.is_empty() {
                    let converted_params: HashMap<String, serde_json::Value> = error
                        .params
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect();
                    field_error.params = Some(converted_params);
                }

                validation_errors.add(field, field_error);
            }
        }

        validation_errors
    }
}

/// Convert ValidationErrors to AppError for HTTP responses
impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        // validator crate has a ValidationErrors type, but we can't use it directly
        // since we've converted to our own type. We'll use BadRequest for now.

        // Serialize to JSON for the error message
        let json = serde_json::to_string_pretty(&errors)
            .unwrap_or_else(|_| "Validation failed".to_string());

        AppError::BadRequest { message: json }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_errors_creation() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());

        errors.add(
            "email",
            FieldError::new("email", "Invalid email address"),
        );
        assert!(!errors.is_empty());
        assert_eq!(errors.errors.len(), 1);
    }

    #[test]
    fn test_field_error_with_params() {
        let error = FieldError::new("length", "Field too short")
            .with_param("min", 8)
            .with_param("actual", 5);

        assert_eq!(error.code, "length");
        assert_eq!(error.message, "Field too short");
        assert!(error.params.is_some());

        let params = error.params.unwrap();
        assert_eq!(params.get("min").unwrap(), &serde_json::json!(8));
        assert_eq!(params.get("actual").unwrap(), &serde_json::json!(5));
    }

    #[test]
    fn test_serialization() {
        let mut errors = ValidationErrors::new();
        errors.add(
            "email",
            FieldError::new("email", "Invalid email").with_param("value", "not-an-email"),
        );

        let json = serde_json::to_string(&errors).unwrap();
        assert!(json.contains("email"));
        assert!(json.contains("Invalid email"));
    }
}
