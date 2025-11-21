//! Form request validation pattern.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
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
#[async_trait]
pub trait FormRequest: DeserializeOwned + Sized + Send + Sync {
    /// Authorize the request.
    async fn authorize(&self) -> FormRequestResult<()> {
        Ok(())
    }

    /// Get validation rules.
    fn rules(&self) -> HashMap<String, Vec<ValidationRule>> {
        HashMap::new()
    }

    /// Validate the request.
    async fn validate(&self) -> FormRequestResult<()> {
        let rules = self.rules();
        let mut errors = Vec::new();

        for (field, field_rules) in rules.iter() {
            for rule in field_rules {
                if let Err(e) = rule.validate(field) {
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
    /// Validate a field.
    pub fn validate(&self, _field: &str) -> Result<(), String> {
        // Simplified validation - in real implementation, this would check the actual value
        Ok(())
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
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
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
