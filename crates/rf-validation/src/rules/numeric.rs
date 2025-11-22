//! Numeric validation rules
//!
//! Provides validation rules for numeric values including integer/float validation,
//! range checking, digit counting, and sign validation.

use crate::validator::{Rule, RuleResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Type Rules
// ============================================================================

/// Validates that a value is an integer
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "age" => vec![Box::new(IntegerRule)],
/// });
/// ```
pub struct IntegerRule;

#[async_trait]
impl Rule for IntegerRule {
    fn name(&self) -> &str {
        "integer"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if value.is_i64() || value.is_u64() {
            Ok(())
        } else if let Some(f) = value.as_f64() {
            if f.fract() == 0.0 {
                Ok(())
            } else {
                Err(self.message())
            }
        } else if let Some(s) = value.as_str() {
            if s.parse::<i64>().is_ok() {
                Ok(())
            } else {
                Err(self.message())
            }
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        "This field must be an integer".to_string()
    }
}

/// Validates that a value is numeric (integer or float)
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "price" => vec![Box::new(NumericRule)],
/// });
/// ```
pub struct NumericRule;

#[async_trait]
impl Rule for NumericRule {
    fn name(&self) -> &str {
        "numeric"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if value.is_number() {
            Ok(())
        } else if let Some(s) = value.as_str() {
            if s.parse::<f64>().is_ok() {
                Ok(())
            } else {
                Err(self.message())
            }
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        "This field must be a number".to_string()
    }
}

// ============================================================================
// Range Rules
// ============================================================================

/// Validates that a numeric value is at least a minimum
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "age" => vec![Box::new(MinRule::new(18))],
/// });
/// ```
pub struct MinRule {
    min: f64,
}

impl MinRule {
    pub fn new(min: impl Into<f64>) -> Self {
        Self { min: min.into() }
    }
}

#[async_trait]
impl Rule for MinRule {
    fn name(&self) -> &str {
        "min"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let num = if let Some(n) = value.as_f64() {
            n
        } else if let Some(n) = value.as_i64() {
            n as f64
        } else if let Some(n) = value.as_u64() {
            n as f64
        } else if let Some(s) = value.as_str() {
            s.parse::<f64>()
                .map_err(|_| "Value must be a number".to_string())?
        } else {
            return Err("Value must be a number".to_string());
        };

        if num >= self.min {
            Ok(())
        } else {
            Err(format!(
                "This field must be at least {} (currently {})",
                self.min, num
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must be at least {}", self.min)
    }
}

/// Validates that a numeric value does not exceed a maximum
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "age" => vec![Box::new(MaxRule::new(120))],
/// });
/// ```
pub struct MaxRule {
    max: f64,
}

impl MaxRule {
    pub fn new(max: impl Into<f64>) -> Self {
        Self { max: max.into() }
    }
}

#[async_trait]
impl Rule for MaxRule {
    fn name(&self) -> &str {
        "max"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let num = if let Some(n) = value.as_f64() {
            n
        } else if let Some(n) = value.as_i64() {
            n as f64
        } else if let Some(n) = value.as_u64() {
            n as f64
        } else if let Some(s) = value.as_str() {
            s.parse::<f64>()
                .map_err(|_| "Value must be a number".to_string())?
        } else {
            return Err("Value must be a number".to_string());
        };

        if num <= self.max {
            Ok(())
        } else {
            Err(format!(
                "This field must not exceed {} (currently {})",
                self.max, num
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must not exceed {}", self.max)
    }
}

/// Validates that a numeric value is between min and max (inclusive)
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "rating" => vec![Box::new(BetweenRule::new(1, 5))],
/// });
/// ```
pub struct BetweenRule {
    min: f64,
    max: f64,
}

impl BetweenRule {
    pub fn new(min: impl Into<f64>, max: impl Into<f64>) -> Self {
        Self {
            min: min.into(),
            max: max.into(),
        }
    }
}

#[async_trait]
impl Rule for BetweenRule {
    fn name(&self) -> &str {
        "between"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let num = if let Some(n) = value.as_f64() {
            n
        } else if let Some(n) = value.as_i64() {
            n as f64
        } else if let Some(n) = value.as_u64() {
            n as f64
        } else if let Some(s) = value.as_str() {
            s.parse::<f64>()
                .map_err(|_| "Value must be a number".to_string())?
        } else {
            return Err("Value must be a number".to_string());
        };

        if num >= self.min && num <= self.max {
            Ok(())
        } else {
            Err(format!(
                "This field must be between {} and {} (currently {})",
                self.min, self.max, num
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must be between {} and {}", self.min, self.max)
    }
}

// ============================================================================
// Digit Rules
// ============================================================================

/// Validates that a numeric value has exactly n digits
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "zipcode" => vec![Box::new(DigitsRule::new(5))],
/// });
/// ```
pub struct DigitsRule {
    digits: usize,
}

impl DigitsRule {
    pub fn new(digits: usize) -> Self {
        Self { digits }
    }
}

#[async_trait]
impl Rule for DigitsRule {
    fn name(&self) -> &str {
        "digits"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let str_val = if let Some(s) = value.as_str() {
            s.to_string()
        } else if let Some(n) = value.as_i64() {
            n.to_string()
        } else if let Some(n) = value.as_u64() {
            n.to_string()
        } else {
            return Err("Value must be a number or string".to_string());
        };

        let digit_count = str_val.chars().filter(|c| c.is_ascii_digit()).count();

        if digit_count == self.digits {
            Ok(())
        } else {
            Err(format!(
                "This field must have exactly {} digits (currently {})",
                self.digits, digit_count
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must have exactly {} digits", self.digits)
    }
}

/// Validates that a numeric value has between min and max digits
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "phone" => vec![Box::new(DigitsBetweenRule::new(10, 15))],
/// });
/// ```
pub struct DigitsBetweenRule {
    min: usize,
    max: usize,
}

impl DigitsBetweenRule {
    pub fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

#[async_trait]
impl Rule for DigitsBetweenRule {
    fn name(&self) -> &str {
        "digits_between"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let str_val = if let Some(s) = value.as_str() {
            s.to_string()
        } else if let Some(n) = value.as_i64() {
            n.to_string()
        } else if let Some(n) = value.as_u64() {
            n.to_string()
        } else {
            return Err("Value must be a number or string".to_string());
        };

        let digit_count = str_val.chars().filter(|c| c.is_ascii_digit()).count();

        if digit_count >= self.min && digit_count <= self.max {
            Ok(())
        } else {
            Err(format!(
                "This field must have between {} and {} digits (currently {})",
                self.min, self.max, digit_count
            ))
        }
    }

    fn message(&self) -> String {
        format!(
            "This field must have between {} and {} digits",
            self.min, self.max
        )
    }
}

// ============================================================================
// Sign Rules
// ============================================================================

/// Validates that a numeric value is positive (> 0)
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "quantity" => vec![Box::new(PositiveRule)],
/// });
/// ```
pub struct PositiveRule;

#[async_trait]
impl Rule for PositiveRule {
    fn name(&self) -> &str {
        "positive"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let num = if let Some(n) = value.as_f64() {
            n
        } else if let Some(n) = value.as_i64() {
            n as f64
        } else if let Some(n) = value.as_u64() {
            n as f64
        } else if let Some(s) = value.as_str() {
            s.parse::<f64>()
                .map_err(|_| "Value must be a number".to_string())?
        } else {
            return Err("Value must be a number".to_string());
        };

        if num > 0.0 {
            Ok(())
        } else {
            Err(format!("This field must be positive (currently {})", num))
        }
    }

    fn message(&self) -> String {
        "This field must be positive".to_string()
    }
}

/// Validates that a numeric value is negative (< 0)
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "debt" => vec![Box::new(NegativeRule)],
/// });
/// ```
pub struct NegativeRule;

#[async_trait]
impl Rule for NegativeRule {
    fn name(&self) -> &str {
        "negative"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let num = if let Some(n) = value.as_f64() {
            n
        } else if let Some(n) = value.as_i64() {
            n as f64
        } else if let Some(s) = value.as_str() {
            s.parse::<f64>()
                .map_err(|_| "Value must be a number".to_string())?
        } else {
            return Err("Value must be a number".to_string());
        };

        if num < 0.0 {
            Ok(())
        } else {
            Err(format!("This field must be negative (currently {})", num))
        }
    }

    fn message(&self) -> String {
        "This field must be negative".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_integer_rule() {
        let rule = IntegerRule;

        assert!(rule.validate(&json!(42), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(-42), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!("42"), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(42.0), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(42.5), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!("abc"), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_numeric_rule() {
        let rule = NumericRule;

        assert!(rule.validate(&json!(42), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(42.5), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!("42.5"), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!("abc"), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(true), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_min_rule() {
        let rule = MinRule::new(18);

        assert!(rule.validate(&json!(20), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(18), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(15), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!("25"), &HashMap::new()).await.is_ok());
    }

    #[tokio::test]
    async fn test_max_rule() {
        let rule = MaxRule::new(100);

        assert!(rule.validate(&json!(50), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(100), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(150), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_between_rule() {
        let rule = BetweenRule::new(10, 100);

        assert!(rule.validate(&json!(50), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(10), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(100), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(5), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(150), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_digits_rule() {
        let rule = DigitsRule::new(5);

        assert!(rule
            .validate(&json!("12345"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule.validate(&json!(12345), &HashMap::new()).await.is_ok());
        assert!(rule
            .validate(&json!("1234"), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!("123456"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_digits_between_rule() {
        let rule = DigitsBetweenRule::new(3, 6);

        assert!(rule
            .validate(&json!("12345"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule.validate(&json!("123"), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!("12"), &HashMap::new()).await.is_err());
        assert!(rule
            .validate(&json!("1234567"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_positive_rule() {
        let rule = PositiveRule;

        assert!(rule.validate(&json!(1), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(100), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(0), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(-1), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_negative_rule() {
        let rule = NegativeRule;

        assert!(rule.validate(&json!(-1), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(-100), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(0), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(1), &HashMap::new()).await.is_err());
    }
}
