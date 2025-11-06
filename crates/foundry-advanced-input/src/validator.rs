use crate::error::{ValidationError, ValidationResult};
use regex::Regex;
use std::sync::OnceLock;

/// Validation rule for input fields
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// Field must be present and non-empty
    Required,
    /// Field must be a valid email
    Email,
    /// Field must be a valid URL
    Url,
    /// Field must contain only numeric characters
    Numeric,
    /// Field must be at least n characters/items long
    MinLength(usize),
    /// Field must be at most n characters/items long
    MaxLength(usize),
    /// Field must match the given regex pattern
    Regex(String),
    /// Field must be one of the given values
    InList(Vec<String>),
    /// Field must be an array
    Array,
    /// Custom validation with a function
    Custom(fn(&str) -> ValidationResult<()>),
}

impl ValidationRule {
    /// Validate a value against this rule
    pub fn validate(&self, field: &str, value: Option<&str>) -> ValidationResult<()> {
        match self {
            Self::Required => {
                if value.is_none() || value.unwrap().trim().is_empty() {
                    return Err(ValidationError::field(field, "This field is required"));
                }
                Ok(())
            }
            Self::Email => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Email is required"))?;
                static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
                let regex = EMAIL_REGEX.get_or_init(|| {
                    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
                });

                if !regex.is_match(val) {
                    return Err(ValidationError::field(field, "Invalid email format"));
                }
                Ok(())
            }
            Self::Url => {
                let val = value.ok_or_else(|| ValidationError::field(field, "URL is required"))?;
                static URL_REGEX: OnceLock<Regex> = OnceLock::new();
                let regex = URL_REGEX.get_or_init(|| {
                    Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap()
                });

                if !regex.is_match(val) {
                    return Err(ValidationError::field(field, "Invalid URL format"));
                }
                Ok(())
            }
            Self::Numeric => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Numeric value is required"))?;
                if !val.chars().all(|c| c.is_numeric() || c == '.' || c == '-') {
                    return Err(ValidationError::field(field, "Must be numeric"));
                }
                Ok(())
            }
            Self::MinLength(min) => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Value is required"))?;
                if val.len() < *min {
                    return Err(ValidationError::field(
                        field,
                        format!("Must be at least {} characters", min),
                    ));
                }
                Ok(())
            }
            Self::MaxLength(max) => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Value is required"))?;
                if val.len() > *max {
                    return Err(ValidationError::field(
                        field,
                        format!("Must be at most {} characters", max),
                    ));
                }
                Ok(())
            }
            Self::Regex(pattern) => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Value is required"))?;
                let regex = Regex::new(pattern)
                    .map_err(|e| ValidationError::custom(format!("Invalid regex pattern: {}", e)))?;

                if !regex.is_match(val) {
                    return Err(ValidationError::field(field, "Does not match required pattern"));
                }
                Ok(())
            }
            Self::InList(list) => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Value is required"))?;
                if !list.contains(&val.to_string()) {
                    return Err(ValidationError::field(
                        field,
                        format!("Must be one of: {}", list.join(", ")),
                    ));
                }
                Ok(())
            }
            Self::Array => {
                // This is validated in the Input struct
                Ok(())
            }
            Self::Custom(f) => {
                let val = value.ok_or_else(|| ValidationError::field(field, "Value is required"))?;
                f(val)
            }
        }
    }
}

/// Validator for input values
pub struct Validator {
    rules: Vec<ValidationRule>,
}

impl Validator {
    /// Create a new validator with the given rules
    pub fn new(rules: Vec<ValidationRule>) -> Self {
        Self { rules }
    }

    /// Validate a value against all rules
    pub fn validate(&self, field: &str, value: Option<&str>) -> ValidationResult<()> {
        for rule in &self.rules {
            rule.validate(field, value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_rule() {
        let rule = ValidationRule::Required;
        assert!(rule.validate("field", Some("value")).is_ok());
        assert!(rule.validate("field", Some("")).is_err());
        assert!(rule.validate("field", None).is_err());
    }

    #[test]
    fn test_email_rule() {
        let rule = ValidationRule::Email;
        assert!(rule.validate("email", Some("user@example.com")).is_ok());
        assert!(rule.validate("email", Some("invalid")).is_err());
        assert!(rule.validate("email", Some("@example.com")).is_err());
        assert!(rule.validate("email", Some("user@")).is_err());
    }

    #[test]
    fn test_url_rule() {
        let rule = ValidationRule::Url;
        assert!(rule.validate("url", Some("https://example.com")).is_ok());
        assert!(rule.validate("url", Some("http://example.com/path")).is_ok());
        assert!(rule.validate("url", Some("invalid")).is_err());
        assert!(rule.validate("url", Some("ftp://example.com")).is_err());
    }

    #[test]
    fn test_numeric_rule() {
        let rule = ValidationRule::Numeric;
        assert!(rule.validate("num", Some("123")).is_ok());
        assert!(rule.validate("num", Some("123.45")).is_ok());
        assert!(rule.validate("num", Some("-123")).is_ok());
        assert!(rule.validate("num", Some("abc")).is_err());
    }

    #[test]
    fn test_min_length_rule() {
        let rule = ValidationRule::MinLength(5);
        assert!(rule.validate("field", Some("12345")).is_ok());
        assert!(rule.validate("field", Some("123456")).is_ok());
        assert!(rule.validate("field", Some("1234")).is_err());
    }

    #[test]
    fn test_max_length_rule() {
        let rule = ValidationRule::MaxLength(5);
        assert!(rule.validate("field", Some("12345")).is_ok());
        assert!(rule.validate("field", Some("1234")).is_ok());
        assert!(rule.validate("field", Some("123456")).is_err());
    }

    #[test]
    fn test_regex_rule() {
        let rule = ValidationRule::Regex(r"^\d{3}-\d{3}-\d{4}$".to_string());
        assert!(rule.validate("phone", Some("123-456-7890")).is_ok());
        assert!(rule.validate("phone", Some("1234567890")).is_err());
    }

    #[test]
    fn test_in_list_rule() {
        let rule = ValidationRule::InList(vec!["red".to_string(), "green".to_string(), "blue".to_string()]);
        assert!(rule.validate("color", Some("red")).is_ok());
        assert!(rule.validate("color", Some("green")).is_ok());
        assert!(rule.validate("color", Some("yellow")).is_err());
    }

    #[test]
    fn test_validator_multiple_rules() {
        let validator = Validator::new(vec![
            ValidationRule::Required,
            ValidationRule::MinLength(3),
            ValidationRule::MaxLength(10),
        ]);

        assert!(validator.validate("field", Some("hello")).is_ok());
        assert!(validator.validate("field", Some("hi")).is_err()); // too short
        assert!(validator.validate("field", Some("hello world!!!")).is_err()); // too long
        assert!(validator.validate("field", None).is_err()); // required
    }
}
