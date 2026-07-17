//! Custom validation rules and helpers.

use std::collections::HashMap;

/// Custom validator trait.
pub trait Validator<T> {
    /// Validate the value.
    fn validate(&self, value: &T) -> ValidationResult;
}

/// Validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Valid,
    Invalid(String),
}

impl ValidationResult {
    /// Check if validation passed.
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// Check if validation failed.
    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }

    /// Get error message if invalid.
    pub fn error_message(&self) -> Option<&str> {
        match self {
            ValidationResult::Invalid(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Email validator.
pub struct EmailValidator;

impl Validator<String> for EmailValidator {
    fn validate(&self, value: &String) -> ValidationResult {
        if value.contains('@') && value.contains('.') {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid("Invalid email format".to_string())
        }
    }
}

/// Length validator.
pub struct LengthValidator {
    min: Option<usize>,
    max: Option<usize>,
}

impl LengthValidator {
    /// Create a new length validator.
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
        }
    }

    /// Set minimum length.
    pub fn min(mut self, min: usize) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum length.
    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }
}

impl Default for LengthValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator<String> for LengthValidator {
    fn validate(&self, value: &String) -> ValidationResult {
        let len = value.len();

        if let Some(min) = self.min {
            if len < min {
                return ValidationResult::Invalid(format!(
                    "Length must be at least {} characters",
                    min
                ));
            }
        }

        if let Some(max) = self.max {
            if len > max {
                return ValidationResult::Invalid(format!(
                    "Length must be at most {} characters",
                    max
                ));
            }
        }

        ValidationResult::Valid
    }
}

/// URL validator.
pub struct UrlValidator;

impl Validator<String> for UrlValidator {
    fn validate(&self, value: &String) -> ValidationResult {
        if value.starts_with("http://") || value.starts_with("https://") {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid("Invalid URL format".to_string())
        }
    }
}

/// Numeric validator.
pub struct NumericValidator;

impl Validator<String> for NumericValidator {
    fn validate(&self, value: &String) -> ValidationResult {
        if value.parse::<f64>().is_ok() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid("Value must be numeric".to_string())
        }
    }
}

/// Custom error messages.
pub struct CustomMessages {
    messages: HashMap<String, String>,
}

impl CustomMessages {
    /// Create new custom messages.
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
        }
    }

    /// Add a custom message.
    pub fn add(mut self, field: &str, message: &str) -> Self {
        self.messages.insert(field.to_string(), message.to_string());
        self
    }

    /// Get message for field.
    pub fn get(&self, field: &str) -> Option<&str> {
        self.messages.get(field).map(|s| s.as_str())
    }
}

impl Default for CustomMessages {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validator() {
        let validator = EmailValidator;

        let valid_email = "test@example.com".to_string();
        assert!(validator.validate(&valid_email).is_valid());

        let invalid_email = "invalid-email".to_string();
        assert!(validator.validate(&invalid_email).is_invalid());
    }

    #[test]
    fn test_length_validator() {
        let validator = LengthValidator::new().min(3).max(10);

        let valid = "hello".to_string();
        assert!(validator.validate(&valid).is_valid());

        let too_short = "hi".to_string();
        assert!(validator.validate(&too_short).is_invalid());

        let too_long = "hello world!".to_string();
        assert!(validator.validate(&too_long).is_invalid());
    }

    #[test]
    fn test_url_validator() {
        let validator = UrlValidator;

        let valid_url = "https://example.com".to_string();
        assert!(validator.validate(&valid_url).is_valid());

        let invalid_url = "example.com".to_string();
        assert!(validator.validate(&invalid_url).is_invalid());
    }

    #[test]
    fn test_numeric_validator() {
        let validator = NumericValidator;

        let valid_number = "123.45".to_string();
        assert!(validator.validate(&valid_number).is_valid());

        let invalid_number = "not-a-number".to_string();
        assert!(validator.validate(&invalid_number).is_invalid());
    }

    #[test]
    fn test_custom_messages() {
        let messages = CustomMessages::new()
            .add("email", "Please provide a valid email address")
            .add("password", "Password must be at least 8 characters");

        assert_eq!(
            messages.get("email"),
            Some("Please provide a valid email address")
        );
        assert_eq!(
            messages.get("password"),
            Some("Password must be at least 8 characters")
        );
        assert_eq!(messages.get("username"), None);
    }
}
