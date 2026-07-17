//! Form request validation pattern.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

/// Error type for form request validation.
#[derive(Debug, thiserror::Error)]
pub enum FormRequestError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}

/// Result type for form requests.
pub type FormRequestResult<T> = Result<T, FormRequestError>;

/// Trait for form request validation.
///
/// Implementors must be both [`DeserializeOwned`] (so they can be built from an
/// incoming request body) and [`Serialize`] (so the default [`validate`] impl can
/// read each field's value back out by name when running the rules engine).
///
/// [`validate`]: FormRequest::validate
#[async_trait]
pub trait FormRequest: DeserializeOwned + Serialize + Sized + Send + Sync {
    /// Authorize the request.
    async fn authorize(&self) -> FormRequestResult<()> {
        Ok(())
    }

    /// Get validation rules.
    fn rules(&self) -> HashMap<String, Vec<ValidationRule>> {
        HashMap::new()
    }

    /// Validate the request against the rules returned by [`rules`].
    ///
    /// Each rule is checked against the *value* of its field. The field's value is
    /// obtained by serializing `self` and looking the field up by name, so the rules
    /// engine works for any [`Serialize`] request type without per-field wiring.
    ///
    /// [`rules`]: FormRequest::rules
    async fn validate(&self) -> FormRequestResult<()> {
        let rules = self.rules();
        if rules.is_empty() {
            return Ok(());
        }

        let data = serde_json::to_value(self).map_err(|e| {
            FormRequestError::ValidationFailed(format!("could not read request data: {}", e))
        })?;

        let mut errors = Vec::new();

        for (field, field_rules) in rules.iter() {
            let value = field_value(&data, field);
            for rule in field_rules {
                if let Err(e) = rule.validate(&value) {
                    errors.push(format!("{}: {}", field, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(FormRequestError::ValidationFailed(errors.join(", ")));
        }

        Ok(())
    }

    /// Hook called after validation passes.
    async fn after_validation(&mut self) -> FormRequestResult<()> {
        Ok(())
    }

    /// Process the form request (authorize + validate).
    async fn process(mut self) -> FormRequestResult<Self> {
        self.authorize().await?;
        self.validate().await?;
        self.after_validation().await?;
        Ok(self)
    }
}

/// Extract a field's value from serialized request data as a string.
///
/// A missing field or an explicit `null` is treated as an empty string so that
/// rules such as [`ValidationRule::Required`] fail as expected. Strings are taken
/// verbatim; other scalar types (numbers, booleans) use their JSON representation.
fn field_value(data: &serde_json::Value, field: &str) -> String {
    match data.get(field) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Validation rule.
#[derive(Debug, Clone)]
pub enum ValidationRule {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Email,
    Url,
    Numeric,
    Custom(String),
}

impl ValidationRule {
    /// Validate a field's `value` against this rule.
    ///
    /// Returns `Ok(())` when the value satisfies the rule, or `Err(message)` with a
    /// human-readable reason when it does not.
    pub fn validate(&self, value: &str) -> Result<(), String> {
        match self {
            ValidationRule::Required => {
                if value.trim().is_empty() {
                    Err("This field is required".to_string())
                } else {
                    Ok(())
                }
            }
            ValidationRule::MinLength(min) => {
                if value.chars().count() < *min {
                    Err(format!("Must be at least {} characters", min))
                } else {
                    Ok(())
                }
            }
            ValidationRule::MaxLength(max) => {
                if value.chars().count() > *max {
                    Err(format!("Must be at most {} characters", max))
                } else {
                    Ok(())
                }
            }
            ValidationRule::Email => {
                if value.contains('@') && value.contains('.') {
                    Ok(())
                } else {
                    Err("Invalid email format".to_string())
                }
            }
            ValidationRule::Url => {
                if value.starts_with("http://") || value.starts_with("https://") {
                    Ok(())
                } else {
                    Err("Invalid URL format".to_string())
                }
            }
            ValidationRule::Numeric => {
                if value.parse::<f64>().is_ok() {
                    Ok(())
                } else {
                    Err("Value must be numeric".to_string())
                }
            }
            // `Custom` carries a caller-supplied message and has no built-in
            // predicate, so it cannot be auto-evaluated here; treat it as passing
            // and leave enforcement to the implementor's own `validate` override.
            ValidationRule::Custom(_) => Ok(()),
        }
    }
}

/// Builder for validation rules.
pub struct ValidationRulesBuilder {
    rules: HashMap<String, Vec<ValidationRule>>,
}

impl ValidationRulesBuilder {
    /// Create a new rules builder.
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Add a required rule.
    pub fn required(mut self, field: &str) -> Self {
        self.rules
            .entry(field.to_string())
            .or_default()
            .push(ValidationRule::Required);
        self
    }

    /// Add a min length rule.
    pub fn min_length(mut self, field: &str, min: usize) -> Self {
        self.rules
            .entry(field.to_string())
            .or_default()
            .push(ValidationRule::MinLength(min));
        self
    }

    /// Add a max length rule.
    pub fn max_length(mut self, field: &str, max: usize) -> Self {
        self.rules
            .entry(field.to_string())
            .or_default()
            .push(ValidationRule::MaxLength(max));
        self
    }

    /// Add an email rule.
    pub fn email(mut self, field: &str) -> Self {
        self.rules
            .entry(field.to_string())
            .or_default()
            .push(ValidationRule::Email);
        self
    }

    /// Build the rules.
    pub fn build(self) -> HashMap<String, Vec<ValidationRule>> {
        self.rules
    }
}

impl Default for ValidationRulesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    struct TestRequest {
        title: String,
        email: String,
    }

    #[async_trait]
    impl FormRequest for TestRequest {
        async fn authorize(&self) -> FormRequestResult<()> {
            if self.title.is_empty() {
                return Err(FormRequestError::AuthorizationFailed(
                    "Title cannot be empty".to_string(),
                ));
            }
            Ok(())
        }

        fn rules(&self) -> HashMap<String, Vec<ValidationRule>> {
            ValidationRulesBuilder::new()
                .required("title")
                .min_length("title", 3)
                .email("email")
                .build()
        }
    }

    #[tokio::test]
    async fn test_form_request_authorize() {
        let request = TestRequest {
            title: "Test".to_string(),
            email: "test@example.com".to_string(),
        };

        assert!(request.authorize().await.is_ok());
    }

    #[tokio::test]
    async fn test_form_request_validation() {
        let request = TestRequest {
            title: "Test".to_string(),
            email: "test@example.com".to_string(),
        };

        assert!(request.validate().await.is_ok());
    }

    #[tokio::test]
    async fn test_form_request_rejects_invalid_input() {
        let request = TestRequest {
            title: "".to_string(),         // violates required + min_length(3)
            email: "not-an-email".to_string(), // violates email
        };

        let err = request
            .validate()
            .await
            .expect_err("invalid input must be rejected by the rules engine");
        match err {
            FormRequestError::ValidationFailed(msg) => {
                assert!(msg.contains("title"), "expected title errors, got: {msg}");
                assert!(msg.contains("email"), "expected email error, got: {msg}");
            }
            other => panic!("expected ValidationFailed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_validation_rules_builder() {
        let rules = ValidationRulesBuilder::new()
            .required("name")
            .min_length("name", 3)
            .max_length("name", 255)
            .email("email")
            .build();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules["name"].len(), 3);
        assert_eq!(rules["email"].len(), 1);
    }
}
