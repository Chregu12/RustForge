use std::collections::HashMap;
use thiserror::Error;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validation error that can contain multiple field errors
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// Single field validation error
    #[error("Validation failed for field '{field}': {message}")]
    FieldError { field: String, message: String },

    /// Multiple field validation errors
    #[error("Validation failed with {0} error(s)")]
    MultipleErrors(HashMap<String, Vec<String>>),

    /// Custom validation error
    #[error("{0}")]
    Custom(String),
}

impl ValidationError {
    /// Create a new field error
    pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::FieldError {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a multiple errors instance
    pub fn multiple(errors: HashMap<String, Vec<String>>) -> Self {
        Self::MultipleErrors(errors)
    }

    /// Create a custom error
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }

    /// Get all error messages as a map
    pub fn errors(&self) -> HashMap<String, Vec<String>> {
        match self {
            Self::FieldError { field, message } => {
                let mut map = HashMap::new();
                map.insert(field.clone(), vec![message.clone()]);
                map
            }
            Self::MultipleErrors(errors) => errors.clone(),
            Self::Custom(msg) => {
                let mut map = HashMap::new();
                map.insert("_error".to_string(), vec![msg.clone()]);
                map
            }
        }
    }

    /// Check if this error contains errors for a specific field
    pub fn has_field(&self, field: &str) -> bool {
        self.errors().contains_key(field)
    }

    /// Get error messages for a specific field
    pub fn field_errors(&self, field: &str) -> Vec<String> {
        self.errors().get(field).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_error() {
        let error = ValidationError::field("email", "Invalid email format");
        assert!(error.has_field("email"));
        assert_eq!(error.field_errors("email"), vec!["Invalid email format"]);
    }

    #[test]
    fn test_multiple_errors() {
        let mut errors = HashMap::new();
        errors.insert("email".to_string(), vec!["Required".to_string()]);
        errors.insert("age".to_string(), vec!["Must be numeric".to_string()]);

        let error = ValidationError::multiple(errors);
        assert!(error.has_field("email"));
        assert!(error.has_field("age"));
        assert_eq!(error.field_errors("email"), vec!["Required"]);
    }

    #[test]
    fn test_custom_error() {
        let error = ValidationError::custom("Something went wrong");
        assert!(error.has_field("_error"));
    }
}
