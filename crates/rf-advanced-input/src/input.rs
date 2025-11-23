use crate::array::parse_array;
use crate::error::{ValidationError, ValidationResult};
use crate::validator::{ValidationRule, Validator};
use std::collections::HashMap;

/// Input container for command-line arguments and options
#[derive(Debug, Clone, Default)]
pub struct Input {
    fields: HashMap<String, String>,
    arrays: HashMap<String, Vec<String>>,
}

impl Input {
    /// Create a new empty input
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a field value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.fields.insert(key.into(), value.into());
    }

    /// Set an array field from a CSV string
    pub fn set_array_from_csv(&mut self, key: impl Into<String>, csv: &str) {
        let values = parse_array(csv).into_iter().map(String::from).collect();
        self.arrays.insert(key.into(), values);
    }

    /// Set an array field directly
    pub fn set_array(&mut self, key: impl Into<String>, values: Vec<impl Into<String>>) {
        let values = values.into_iter().map(|v| v.into()).collect();
        self.arrays.insert(key.into(), values);
    }

    /// Get a field value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    /// Get an array field
    pub fn get_array(&self, key: &str) -> Option<&[String]> {
        self.arrays.get(key).map(|v| v.as_slice())
    }

    /// Check if a field exists
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key) || self.arrays.contains_key(key)
    }

    /// Check if a field is an array
    pub fn is_array(&self, key: &str) -> bool {
        self.arrays.contains_key(key)
    }

    /// Get all field keys
    pub fn keys(&self) -> Vec<&str> {
        self.fields
            .keys()
            .chain(self.arrays.keys())
            .map(|s| s.as_str())
            .collect()
    }

    /// Validate the input against the given rules
    ///
    /// # Examples
    ///
    /// ```
    /// use foundry_advanced_input::{Input, ValidationRule};
    ///
    /// let mut input = Input::new();
    /// input.set("email", "user@example.com");
    /// input.set("age", "25");
    ///
    /// let rules = vec![
    ///     ("email", vec![ValidationRule::Required, ValidationRule::Email]),
    ///     ("age", vec![ValidationRule::Required, ValidationRule::Numeric]),
    /// ];
    ///
    /// let result = input.validate(&rules);
    /// assert!(result.is_ok());
    /// ```
    pub fn validate(&self, rules: &[(&str, Vec<ValidationRule>)]) -> ValidationResult<ValidatedInput> {
        let mut errors: HashMap<String, Vec<String>> = HashMap::new();

        for (field, field_rules) in rules {
            let validator = Validator::new(field_rules.clone());

            // Check if it's an array field
            if self.is_array(field) {
                // Validate array-specific rules
                for rule in field_rules {
                    match rule {
                        ValidationRule::Array => continue,
                        ValidationRule::MinLength(min) => {
                            if let Some(arr) = self.get_array(field) {
                                if arr.len() < *min {
                                    errors.entry(field.to_string()).or_default().push(
                                        format!("Must have at least {} item(s)", min),
                                    );
                                }
                            }
                        }
                        ValidationRule::MaxLength(max) => {
                            if let Some(arr) = self.get_array(field) {
                                if arr.len() > *max {
                                    errors.entry(field.to_string()).or_default().push(
                                        format!("Must have at most {} item(s)", max),
                                    );
                                }
                            }
                        }
                        ValidationRule::Required => {
                            if let Some(arr) = self.get_array(field) {
                                if arr.is_empty() {
                                    errors
                                        .entry(field.to_string())
                                        .or_default()
                                        .push("This field is required".to_string());
                                }
                            } else {
                                errors
                                    .entry(field.to_string())
                                    .or_default()
                                    .push("This field is required".to_string());
                            }
                        }
                        _ => {
                            // For other rules, validate each array element
                            if let Some(arr) = self.get_array(field) {
                                for (i, item) in arr.iter().enumerate() {
                                    if let Err(e) = rule.validate(field, Some(item)) {
                                        errors.entry(field.to_string()).or_default().push(
                                            format!("Item {}: {}", i, e.field_errors(field).join(", ")),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Regular field validation
                let value = self.get(field);
                if let Err(e) = validator.validate(field, value) {
                    errors
                        .entry(field.to_string())
                        .or_default()
                        .extend(e.field_errors(field));
                }
            }
        }

        if !errors.is_empty() {
            return Err(ValidationError::multiple(errors));
        }

        Ok(ValidatedInput {
            fields: self.fields.clone(),
            arrays: self.arrays.clone(),
        })
    }
}

/// Validated input that has passed all validation rules
#[derive(Debug, Clone)]
pub struct ValidatedInput {
    fields: HashMap<String, String>,
    arrays: HashMap<String, Vec<String>>,
}

impl ValidatedInput {
    /// Get a field value (guaranteed to exist if validated with Required)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    /// Get an array field
    pub fn get_array(&self, key: &str) -> Option<&[String]> {
        self.arrays.get(key).map(|v| v.as_slice())
    }

    /// Get a field value or a default
    pub fn get_or(&self, key: &str, default: &str) -> &str {
        self.get(key).unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_set_get() {
        let mut input = Input::new();
        input.set("name", "Alice");
        assert_eq!(input.get("name"), Some("Alice"));
    }

    #[test]
    fn test_input_array_from_csv() {
        let mut input = Input::new();
        input.set_array_from_csv("tags", "rust,web,cli");
        assert_eq!(input.get_array("tags"), Some(["rust", "web", "cli"].as_ref()));
    }

    #[test]
    fn test_input_array_direct() {
        let mut input = Input::new();
        input.set_array("ids", vec!["1", "2", "3"]);
        assert_eq!(input.get_array("ids"), Some(["1", "2", "3"].as_ref()));
    }

    #[test]
    fn test_input_has() {
        let mut input = Input::new();
        input.set("name", "Alice");
        assert!(input.has("name"));
        assert!(!input.has("age"));
    }

    #[test]
    fn test_input_is_array() {
        let mut input = Input::new();
        input.set("name", "Alice");
        input.set_array("tags", vec!["rust"]);
        assert!(!input.is_array("name"));
        assert!(input.is_array("tags"));
    }

    #[test]
    fn test_validate_success() {
        let mut input = Input::new();
        input.set("email", "user@example.com");
        input.set("age", "25");

        let rules = vec![
            ("email", vec![ValidationRule::Required, ValidationRule::Email]),
            ("age", vec![ValidationRule::Required, ValidationRule::Numeric]),
        ];

        let result = input.validate(&rules);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_failure() {
        let mut input = Input::new();
        input.set("email", "invalid");

        let rules = vec![("email", vec![ValidationRule::Email])];

        let result = input.validate(&rules);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_array_min_length() {
        let mut input = Input::new();
        input.set_array("tags", vec!["rust"]);

        let rules = vec![("tags", vec![ValidationRule::Array, ValidationRule::MinLength(2)])];

        let result = input.validate(&rules);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_array_max_length() {
        let mut input = Input::new();
        input.set_array("tags", vec!["rust", "web", "cli", "framework"]);

        let rules = vec![("tags", vec![ValidationRule::Array, ValidationRule::MaxLength(3)])];

        let result = input.validate(&rules);
        assert!(result.is_err());
    }

    #[test]
    fn test_validated_input_get() {
        let mut input = Input::new();
        input.set("name", "Alice");

        let validated = input.validate(&[]).unwrap();
        assert_eq!(validated.get("name"), Some("Alice"));
    }

    #[test]
    fn test_validated_input_get_or() {
        let mut input = Input::new();
        input.set("name", "Alice");

        let validated = input.validate(&[]).unwrap();
        assert_eq!(validated.get_or("name", "Bob"), "Alice");
        assert_eq!(validated.get_or("missing", "default"), "default");
    }
}
