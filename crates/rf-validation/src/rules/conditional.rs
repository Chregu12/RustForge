//! Conditional validation rules
//!
//! Provides validation rules that depend on the presence or values
//! of other fields in the data being validated.

use crate::validator::{Rule, RuleResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Required If Rule
// ============================================================================

/// Validates that a field is required if another field has a specific value
///
/// # Example
///
/// ```ignore
/// // "other_address" is required if "has_other_address" is true
/// validator.rules(hashmap! {
///     "other_address" => vec![
///         Box::new(RequiredIfRule::new("has_other_address", json!(true)))
///     ],
/// });
/// ```
pub struct RequiredIfRule {
    other_field: String,
    other_value: Value,
}

impl RequiredIfRule {
    pub fn new(other_field: impl Into<String>, other_value: Value) -> Self {
        Self {
            other_field: other_field.into(),
            other_value,
        }
    }

    pub fn new_bool(other_field: impl Into<String>, other_value: bool) -> Self {
        Self {
            other_field: other_field.into(),
            other_value: Value::Bool(other_value),
        }
    }

    pub fn new_string(other_field: impl Into<String>, other_value: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
            other_value: Value::String(other_value.into()),
        }
    }

    fn is_empty(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(s) if s.trim().is_empty() => true,
            Value::Array(arr) if arr.is_empty() => true,
            Value::Object(obj) if obj.is_empty() => true,
            _ => false,
        }
    }
}

#[async_trait]
impl Rule for RequiredIfRule {
    fn name(&self) -> &str {
        "required_if"
    }

    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult {
        let other_value = data.get(&self.other_field).unwrap_or(&Value::Null);

        if other_value == &self.other_value {
            // Other field has the target value, so this field is required
            if Self::is_empty(value) {
                Err(format!(
                    "This field is required when {} is {}",
                    self.other_field, self.other_value
                ))
            } else {
                Ok(())
            }
        } else {
            // Other field doesn't have the target value, so this field is optional
            Ok(())
        }
    }

    fn message(&self) -> String {
        format!(
            "This field is required when {} is {}",
            self.other_field, self.other_value
        )
    }
}

// ============================================================================
// Required Unless Rule
// ============================================================================

/// Validates that a field is required unless another field has a specific value
///
/// # Example
///
/// ```ignore
/// // "reason" is required unless "status" is "approved"
/// validator.rules(hashmap! {
///     "reason" => vec![
///         Box::new(RequiredUnlessRule::new_string("status", "approved"))
///     ],
/// });
/// ```
pub struct RequiredUnlessRule {
    other_field: String,
    other_value: Value,
}

impl RequiredUnlessRule {
    pub fn new(other_field: impl Into<String>, other_value: Value) -> Self {
        Self {
            other_field: other_field.into(),
            other_value,
        }
    }

    pub fn new_bool(other_field: impl Into<String>, other_value: bool) -> Self {
        Self {
            other_field: other_field.into(),
            other_value: Value::Bool(other_value),
        }
    }

    pub fn new_string(other_field: impl Into<String>, other_value: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
            other_value: Value::String(other_value.into()),
        }
    }

    fn is_empty(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(s) if s.trim().is_empty() => true,
            Value::Array(arr) if arr.is_empty() => true,
            Value::Object(obj) if obj.is_empty() => true,
            _ => false,
        }
    }
}

#[async_trait]
impl Rule for RequiredUnlessRule {
    fn name(&self) -> &str {
        "required_unless"
    }

    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult {
        let other_value = data.get(&self.other_field).unwrap_or(&Value::Null);

        if other_value != &self.other_value {
            // Other field doesn't have the target value, so this field is required
            if Self::is_empty(value) {
                Err(format!(
                    "This field is required unless {} is {}",
                    self.other_field, self.other_value
                ))
            } else {
                Ok(())
            }
        } else {
            // Other field has the target value, so this field is optional
            Ok(())
        }
    }

    fn message(&self) -> String {
        format!(
            "This field is required unless {} is {}",
            self.other_field, self.other_value
        )
    }
}

// ============================================================================
// Required With Rule
// ============================================================================

/// Validates that a field is required if another field is present
///
/// # Example
///
/// ```ignore
/// // "city" is required if "address" is present
/// validator.rules(hashmap! {
///     "city" => vec![Box::new(RequiredWithRule::new("address"))],
/// });
/// ```
pub struct RequiredWithRule {
    other_field: String,
}

impl RequiredWithRule {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }

    fn is_empty(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(s) if s.trim().is_empty() => true,
            Value::Array(arr) if arr.is_empty() => true,
            Value::Object(obj) if obj.is_empty() => true,
            _ => false,
        }
    }
}

#[async_trait]
impl Rule for RequiredWithRule {
    fn name(&self) -> &str {
        "required_with"
    }

    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult {
        let other_value = data.get(&self.other_field).unwrap_or(&Value::Null);

        if !Self::is_empty(other_value) {
            // Other field is present, so this field is required
            if Self::is_empty(value) {
                Err(format!(
                    "This field is required when {} is present",
                    self.other_field
                ))
            } else {
                Ok(())
            }
        } else {
            // Other field is not present, so this field is optional
            Ok(())
        }
    }

    fn message(&self) -> String {
        format!(
            "This field is required when {} is present",
            self.other_field
        )
    }
}

// ============================================================================
// Required Without Rule
// ============================================================================

/// Validates that a field is required if another field is not present
///
/// # Example
///
/// ```ignore
/// // "email" is required if "phone" is not present
/// validator.rules(hashmap! {
///     "email" => vec![Box::new(RequiredWithoutRule::new("phone"))],
/// });
/// ```
pub struct RequiredWithoutRule {
    other_field: String,
}

impl RequiredWithoutRule {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }

    fn is_empty(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::String(s) if s.trim().is_empty() => true,
            Value::Array(arr) if arr.is_empty() => true,
            Value::Object(obj) if obj.is_empty() => true,
            _ => false,
        }
    }
}

#[async_trait]
impl Rule for RequiredWithoutRule {
    fn name(&self) -> &str {
        "required_without"
    }

    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult {
        let other_value = data.get(&self.other_field).unwrap_or(&Value::Null);

        if Self::is_empty(other_value) {
            // Other field is not present, so this field is required
            if Self::is_empty(value) {
                Err(format!(
                    "This field is required when {} is not present",
                    self.other_field
                ))
            } else {
                Ok(())
            }
        } else {
            // Other field is present, so this field is optional
            Ok(())
        }
    }

    fn message(&self) -> String {
        format!(
            "This field is required when {} is not present",
            self.other_field
        )
    }
}

// ============================================================================
// Same Rule
// ============================================================================

/// Validates that a field value matches another field's value
///
/// # Example
///
/// ```ignore
/// // "password_confirmation" must match "password"
/// validator.rules(hashmap! {
///     "password_confirmation" => vec![Box::new(SameRule::new("password"))],
/// });
/// ```
pub struct SameRule {
    other_field: String,
}

impl SameRule {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }
}

#[async_trait]
impl Rule for SameRule {
    fn name(&self) -> &str {
        "same"
    }

    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let other_value = data.get(&self.other_field).unwrap_or(&Value::Null);

        if value == other_value {
            Ok(())
        } else {
            Err(format!("This field must match {}", self.other_field))
        }
    }

    fn message(&self) -> String {
        format!("This field must match {}", self.other_field)
    }
}

// ============================================================================
// Different Rule
// ============================================================================

/// Validates that a field value is different from another field's value
///
/// # Example
///
/// ```ignore
/// // "new_password" must be different from "old_password"
/// validator.rules(hashmap! {
///     "new_password" => vec![Box::new(DifferentRule::new("old_password"))],
/// });
/// ```
pub struct DifferentRule {
    other_field: String,
}

impl DifferentRule {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }
}

#[async_trait]
impl Rule for DifferentRule {
    fn name(&self) -> &str {
        "different"
    }

    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let other_value = data.get(&self.other_field).unwrap_or(&Value::Null);

        if value != other_value {
            Ok(())
        } else {
            Err(format!(
                "This field must be different from {}",
                self.other_field
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must be different from {}", self.other_field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_required_if_rule() {
        let rule = RequiredIfRule::new_bool("has_other_address", true);

        let mut data = HashMap::new();
        data.insert("has_other_address".to_string(), json!(true));

        // Should fail when field is empty and condition is met
        assert!(rule.validate(&json!(null), &data).await.is_err());
        assert!(rule.validate(&json!(""), &data).await.is_err());

        // Should pass when field has value and condition is met
        assert!(rule.validate(&json!("123 Main St"), &data).await.is_ok());

        // Should pass when condition is not met (regardless of field value)
        data.insert("has_other_address".to_string(), json!(false));
        assert!(rule.validate(&json!(null), &data).await.is_ok());
    }

    #[tokio::test]
    async fn test_required_unless_rule() {
        let rule = RequiredUnlessRule::new_string("status", "approved");

        let mut data = HashMap::new();
        data.insert("status".to_string(), json!("pending"));

        // Should fail when field is empty and condition is not met
        assert!(rule.validate(&json!(null), &data).await.is_err());

        // Should pass when field has value and condition is not met
        assert!(rule.validate(&json!("some reason"), &data).await.is_ok());

        // Should pass when condition is met (status is "approved")
        data.insert("status".to_string(), json!("approved"));
        assert!(rule.validate(&json!(null), &data).await.is_ok());
    }

    #[tokio::test]
    async fn test_required_with_rule() {
        let rule = RequiredWithRule::new("address");

        let mut data = HashMap::new();

        // Should pass when other field is not present
        assert!(rule.validate(&json!(null), &data).await.is_ok());

        // Should fail when other field is present but this field is empty
        data.insert("address".to_string(), json!("123 Main St"));
        assert!(rule.validate(&json!(null), &data).await.is_err());

        // Should pass when both fields are present
        assert!(rule.validate(&json!("New York"), &data).await.is_ok());
    }

    #[tokio::test]
    async fn test_required_without_rule() {
        let rule = RequiredWithoutRule::new("phone");

        let mut data = HashMap::new();

        // Should fail when other field is not present and this field is empty
        assert!(rule.validate(&json!(null), &data).await.is_err());

        // Should pass when other field is not present but this field has value
        assert!(rule
            .validate(&json!("user@example.com"), &data)
            .await
            .is_ok());

        // Should pass when other field is present
        data.insert("phone".to_string(), json!("555-1234"));
        assert!(rule.validate(&json!(null), &data).await.is_ok());
    }

    #[tokio::test]
    async fn test_same_rule() {
        let rule = SameRule::new("password");

        let mut data = HashMap::new();
        data.insert("password".to_string(), json!("secret123"));

        // Should pass when values match
        assert!(rule.validate(&json!("secret123"), &data).await.is_ok());

        // Should fail when values don't match
        assert!(rule.validate(&json!("different"), &data).await.is_err());
    }

    #[tokio::test]
    async fn test_different_rule() {
        let rule = DifferentRule::new("old_password");

        let mut data = HashMap::new();
        data.insert("old_password".to_string(), json!("old123"));

        // Should pass when values are different
        assert!(rule.validate(&json!("new456"), &data).await.is_ok());

        // Should fail when values are the same
        assert!(rule.validate(&json!("old123"), &data).await.is_err());
    }
}
