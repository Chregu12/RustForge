//! Date validation rules
//!
//! Provides validation rules for date values including date parsing,
//! format validation, and temporal comparisons.

use crate::validator::{Rule, RuleResult};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;

// ============================================================================
// Date Type Rules
// ============================================================================

/// Validates that a value is a valid date
///
/// Supports ISO 8601 format strings and Unix timestamps.
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "birth_date" => vec![Box::new(DateRule)],
/// });
/// ```
pub struct DateRule;

impl DateRule {
    fn parse_date(value: &Value) -> Option<DateTime<Utc>> {
        if let Some(s) = value.as_str() {
            // Try ISO 8601 datetime
            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&Utc));
            }

            // Try date only (YYYY-MM-DD)
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                    return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                }
            }

            // Try other common formats
            let formats = vec![
                "%Y-%m-%d %H:%M:%S",
                "%Y/%m/%d",
                "%d-%m-%Y",
                "%d/%m/%Y",
                "%m/%d/%Y",
            ];

            for format in formats {
                if let Ok(dt) = NaiveDateTime::parse_from_str(s, format) {
                    return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                }
                if let Ok(date) = NaiveDate::parse_from_str(s, format) {
                    if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
                    }
                }
            }
        } else if let Some(timestamp) = value.as_i64() {
            // Unix timestamp
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                return Some(dt);
            }
        }

        None
    }
}

#[async_trait]
impl Rule for DateRule {
    fn name(&self) -> &str {
        "date"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if Self::parse_date(value).is_some() {
            Ok(())
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        "This field must be a valid date".to_string()
    }
}

// ============================================================================
// Date Format Rule
// ============================================================================

/// Validates that a date string matches a specific format
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "birth_date" => vec![Box::new(DateFormatRule::new("%Y-%m-%d"))],
/// });
/// ```
pub struct DateFormatRule {
    format: String,
}

impl DateFormatRule {
    pub fn new(format: impl Into<String>) -> Self {
        Self {
            format: format.into(),
        }
    }
}

#[async_trait]
impl Rule for DateFormatRule {
    fn name(&self) -> &str {
        "date_format"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let s = value
            .as_str()
            .ok_or_else(|| "Value must be a string".to_string())?;

        // Try parsing as datetime first
        if NaiveDateTime::parse_from_str(s, &self.format).is_ok() {
            return Ok(());
        }

        // Try parsing as date
        if NaiveDate::parse_from_str(s, &self.format).is_ok() {
            return Ok(());
        }

        Err(format!(
            "This field must be a date in format '{}'",
            self.format
        ))
    }

    fn message(&self) -> String {
        format!("This field must be a date in format '{}'", self.format)
    }
}

// ============================================================================
// Temporal Comparison Rules
// ============================================================================

/// Validates that a date is before a specific date
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "birth_date" => vec![Box::new(BeforeRule::new("2010-01-01"))],
/// });
/// ```
pub struct BeforeRule {
    date: DateTime<Utc>,
    date_str: String,
}

impl BeforeRule {
    pub fn new(date: impl Into<String>) -> Self {
        let date_str = date.into();
        let date = DateTime::parse_from_rfc3339(&date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc)
                })
            })
            .unwrap_or_else(|_| Utc::now());

        Self { date, date_str }
    }
}

#[async_trait]
impl Rule for BeforeRule {
    fn name(&self) -> &str {
        "before"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let value_date =
            DateRule::parse_date(value).ok_or_else(|| "Value must be a valid date".to_string())?;

        if value_date < self.date {
            Ok(())
        } else {
            Err(format!("This field must be before {}", self.date_str))
        }
    }

    fn message(&self) -> String {
        format!("This field must be before {}", self.date_str)
    }
}

/// Validates that a date is after a specific date
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "event_date" => vec![Box::new(AfterRule::new("2024-01-01"))],
/// });
/// ```
pub struct AfterRule {
    date: DateTime<Utc>,
    date_str: String,
}

impl AfterRule {
    pub fn new(date: impl Into<String>) -> Self {
        let date_str = date.into();
        let date = DateTime::parse_from_rfc3339(&date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc)
                })
            })
            .unwrap_or_else(|_| Utc::now());

        Self { date, date_str }
    }
}

#[async_trait]
impl Rule for AfterRule {
    fn name(&self) -> &str {
        "after"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let value_date =
            DateRule::parse_date(value).ok_or_else(|| "Value must be a valid date".to_string())?;

        if value_date > self.date {
            Ok(())
        } else {
            Err(format!("This field must be after {}", self.date_str))
        }
    }

    fn message(&self) -> String {
        format!("This field must be after {}", self.date_str)
    }
}

/// Validates that a date is between two dates
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "appointment" => vec![Box::new(BetweenDatesRule::new("2024-01-01", "2024-12-31"))],
/// });
/// ```
pub struct BetweenDatesRule {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    start_str: String,
    end_str: String,
}

impl BetweenDatesRule {
    pub fn new(start: impl Into<String>, end: impl Into<String>) -> Self {
        let start_str = start.into();
        let end_str = end.into();

        let start = DateTime::parse_from_rfc3339(&start_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(&start_str, "%Y-%m-%d").map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc)
                })
            })
            .unwrap_or_else(|_| Utc::now());

        let end = DateTime::parse_from_rfc3339(&end_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(&end_str, "%Y-%m-%d").map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(
                        d.and_hms_opt(23, 59, 59).unwrap(),
                        Utc,
                    )
                })
            })
            .unwrap_or_else(|_| Utc::now());

        Self {
            start,
            end,
            start_str,
            end_str,
        }
    }
}

#[async_trait]
impl Rule for BetweenDatesRule {
    fn name(&self) -> &str {
        "between_dates"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let value_date =
            DateRule::parse_date(value).ok_or_else(|| "Value must be a valid date".to_string())?;

        if value_date >= self.start && value_date <= self.end {
            Ok(())
        } else {
            Err(format!(
                "This field must be between {} and {}",
                self.start_str, self.end_str
            ))
        }
    }

    fn message(&self) -> String {
        format!(
            "This field must be between {} and {}",
            self.start_str, self.end_str
        )
    }
}

/// Validates that a date is before or equal to a specific date
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "deadline" => vec![Box::new(BeforeOrEqualRule::new("2024-12-31"))],
/// });
/// ```
pub struct BeforeOrEqualRule {
    date: DateTime<Utc>,
    date_str: String,
}

impl BeforeOrEqualRule {
    pub fn new(date: impl Into<String>) -> Self {
        let date_str = date.into();
        let date = DateTime::parse_from_rfc3339(&date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(
                        d.and_hms_opt(23, 59, 59).unwrap(),
                        Utc,
                    )
                })
            })
            .unwrap_or_else(|_| Utc::now());

        Self { date, date_str }
    }
}

#[async_trait]
impl Rule for BeforeOrEqualRule {
    fn name(&self) -> &str {
        "before_or_equal"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let value_date =
            DateRule::parse_date(value).ok_or_else(|| "Value must be a valid date".to_string())?;

        if value_date <= self.date {
            Ok(())
        } else {
            Err(format!(
                "This field must be before or equal to {}",
                self.date_str
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must be before or equal to {}", self.date_str)
    }
}

/// Validates that a date is after or equal to a specific date
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "start_date" => vec![Box::new(AfterOrEqualRule::new("2024-01-01"))],
/// });
/// ```
pub struct AfterOrEqualRule {
    date: DateTime<Utc>,
    date_str: String,
}

impl AfterOrEqualRule {
    pub fn new(date: impl Into<String>) -> Self {
        let date_str = date.into();
        let date = DateTime::parse_from_rfc3339(&date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map(|d| {
                    DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc)
                })
            })
            .unwrap_or_else(|_| Utc::now());

        Self { date, date_str }
    }
}

#[async_trait]
impl Rule for AfterOrEqualRule {
    fn name(&self) -> &str {
        "after_or_equal"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        let value_date =
            DateRule::parse_date(value).ok_or_else(|| "Value must be a valid date".to_string())?;

        if value_date >= self.date {
            Ok(())
        } else {
            Err(format!(
                "This field must be after or equal to {}",
                self.date_str
            ))
        }
    }

    fn message(&self) -> String {
        format!("This field must be after or equal to {}", self.date_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_date_rule() {
        let rule = DateRule;

        assert!(rule
            .validate(&json!("2024-01-01"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2024-01-01T10:30:00Z"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!(1704067200), &HashMap::new())
            .await
            .is_ok()); // Unix timestamp
        assert!(rule
            .validate(&json!("not-a-date"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_date_format_rule() {
        let rule = DateFormatRule::new("%Y-%m-%d");

        assert!(rule
            .validate(&json!("2024-01-01"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("01/01/2024"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_before_rule() {
        let rule = BeforeRule::new("2024-12-31");

        assert!(rule
            .validate(&json!("2024-01-01"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2025-01-01"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_after_rule() {
        let rule = AfterRule::new("2024-01-01");

        assert!(rule
            .validate(&json!("2024-12-31"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2023-12-31"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_between_dates_rule() {
        let rule = BetweenDatesRule::new("2024-01-01", "2024-12-31");

        assert!(rule
            .validate(&json!("2024-06-15"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2023-12-31"), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!("2025-01-01"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_before_or_equal_rule() {
        let rule = BeforeOrEqualRule::new("2024-12-31");

        assert!(rule
            .validate(&json!("2024-12-31"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2024-01-01"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2025-01-01"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_after_or_equal_rule() {
        let rule = AfterOrEqualRule::new("2024-01-01");

        assert!(rule
            .validate(&json!("2024-01-01"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2024-12-31"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("2023-12-31"), &HashMap::new())
            .await
            .is_err());
    }
}
