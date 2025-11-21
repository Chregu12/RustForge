//! Array validation rules
//!
//! Provides validation rules for array/collection values including
//! type checking, value membership, and uniqueness validation.

use crate::validator::{Rule, RuleResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Array Type Rule
// ============================================================================

/// Validates that a value is an array
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "tags" => vec![Box::new(ArrayRule)],
/// });
/// ```
pub struct ArrayRule;

#[async_trait]
impl Rule for ArrayRule {
    fn name(&self) -> &str {
        "array"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if value.is_array() {
            Ok(())
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        "This field must be an array".to_string()
    }
}

// ============================================================================
// In Rule
// ============================================================================

/// Validates that a value is in a predefined list of values
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "status" => vec![Box::new(InRule::new(vec!["active", "inactive", "pending"]))],
/// });
/// ```
pub struct InRule {
    allowed: Vec<Value>,
}

impl InRule {
    pub fn new<T: Into<Value>>(values: Vec<T>) -> Self {
        Self {
            allowed: values.into_iter().map(|v| v.into()).collect(),
        }
    }

    pub fn from_strings(values: Vec<&str>) -> Self {
        Self {
            allowed: values
                .into_iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        }
    }

    pub fn from_ints(values: Vec<i64>) -> Self {
        Self {
            allowed: values.into_iter().map(|i| Value::from(i)).collect(),
        }
    }
}

#[async_trait]
impl Rule for InRule {
    fn name(&self) -> &str {
        "in"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if self.allowed.contains(value) {
            Ok(())
        } else {
            Err(format!(
                "This field must be one of: {}",
                self.allowed
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => format!("'{}'", s),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => "?".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    }

    fn message(&self) -> String {
        format!(
            "This field must be one of: {}",
            self.allowed
                .iter()
                .map(|v| match v {
                    Value::String(s) => format!("'{}'", s),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => "?".to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

// ============================================================================
// Not In Rule
// ============================================================================

/// Validates that a value is NOT in a predefined list of values
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "username" => vec![Box::new(NotInRule::from_strings(vec!["admin", "root"]))],
/// });
/// ```
pub struct NotInRule {
    forbidden: Vec<Value>,
}

impl NotInRule {
    pub fn new<T: Into<Value>>(values: Vec<T>) -> Self {
        Self {
            forbidden: values.into_iter().map(|v| v.into()).collect(),
        }
    }

    pub fn from_strings(values: Vec<&str>) -> Self {
        Self {
            forbidden: values
                .into_iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        }
    }

    pub fn from_ints(values: Vec<i64>) -> Self {
        Self {
            forbidden: values.into_iter().map(|i| Value::from(i)).collect(),
        }
    }
}

#[async_trait]
impl Rule for NotInRule {
    fn name(&self) -> &str {
        "not_in"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if !self.forbidden.contains(value) {
            Ok(())
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        format!(
            "This field must not be one of: {}",
            self.forbidden
                .iter()
                .map(|v| match v {
                    Value::String(s) => format!("'{}'", s),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => "?".to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

// ============================================================================
// Distinct Rule
// ============================================================================

/// Validates that an array contains no duplicate values
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "tags" => vec![Box::new(DistinctRule)],
/// });
/// ```
pub struct DistinctRule;

#[async_trait]
impl Rule for DistinctRule {
    fn name(&self) -> &str {
        "distinct"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let arr = value
            .as_array()
            .ok_or_else(|| "Value must be an array".to_string())?;

        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();

        for (index, item) in arr.iter().enumerate() {
            let item_str = serde_json::to_string(item).unwrap_or_default();
            if !seen.insert(item_str.clone()) {
                duplicates.push((index, item_str));
            }
        }

        if duplicates.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "This field must not contain duplicate values (found {} duplicates)",
                duplicates.len()
            ))
        }
    }

    fn message(&self) -> String {
        "This field must not contain duplicate values".to_string()
    }
}

// ============================================================================
// Array Size Rules
// ============================================================================

/// Validates minimum array size
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "tags" => vec![Box::new(MinArraySizeRule::new(3))],
/// });
/// ```
pub struct MinArraySizeRule {
    min: usize,
}

impl MinArraySizeRule {
    pub fn new(min: usize) -> Self {
        Self { min }
    }
}

#[async_trait]
impl Rule for MinArraySizeRule {
    fn name(&self) -> &str {
        "min_array_size"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let arr = value
            .as_array()
            .ok_or_else(|| "Value must be an array".to_string())?;

        if arr.len() >= self.min {
            Ok(())
        } else {
            Err(format!(
                "This field must contain at least {} items (currently {})",
                self.min,
                arr.len()
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must contain at least {} items", self.min)
    }
}

/// Validates maximum array size
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "tags" => vec![Box::new(MaxArraySizeRule::new(10))],
/// });
/// ```
pub struct MaxArraySizeRule {
    max: usize,
}

impl MaxArraySizeRule {
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}

#[async_trait]
impl Rule for MaxArraySizeRule {
    fn name(&self) -> &str {
        "max_array_size"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let arr = value
            .as_array()
            .ok_or_else(|| "Value must be an array".to_string())?;

        if arr.len() <= self.max {
            Ok(())
        } else {
            Err(format!(
                "This field must not contain more than {} items (currently {})",
                self.max,
                arr.len()
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must not contain more than {} items", self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_array_rule() {
        let rule = ArrayRule;

        assert!(rule
            .validate(&json!([1, 2, 3]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule.validate(&json!([]), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(123), &HashMap::new()).await.is_err());
        assert!(rule
            .validate(&json!("string"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_in_rule() {
        let rule = InRule::from_strings(vec!["active", "inactive", "pending"]);

        assert!(rule
            .validate(&json!("active"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("pending"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("deleted"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_in_rule_ints() {
        let rule = InRule::from_ints(vec![1, 2, 3, 5, 8]);

        assert!(rule.validate(&json!(1), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(5), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(4), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(10), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_not_in_rule() {
        let rule = NotInRule::from_strings(vec!["admin", "root", "system"]);

        assert!(rule
            .validate(&json!("user"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("admin"), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!("root"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_distinct_rule() {
        let rule = DistinctRule;

        assert!(rule
            .validate(&json!([1, 2, 3, 4, 5]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!(["a", "b", "c"]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!([1, 2, 2, 3]), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!(["a", "b", "a"]), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_min_array_size_rule() {
        let rule = MinArraySizeRule::new(3);

        assert!(rule
            .validate(&json!([1, 2, 3]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!([1, 2, 3, 4]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!([1, 2]), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_max_array_size_rule() {
        let rule = MaxArraySizeRule::new(5);

        assert!(rule
            .validate(&json!([1, 2, 3]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!([1, 2, 3, 4, 5]), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!([1, 2, 3, 4, 5, 6]), &HashMap::new())
            .await
            .is_err());
    }
}
