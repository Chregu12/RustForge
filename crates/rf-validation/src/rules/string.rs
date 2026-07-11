//! String validation rules
//!
//! Provides comprehensive validation rules for string values including
//! email, URL, UUID, length constraints, pattern matching, and more.

use crate::validator::{Rule, RuleResult};
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

// ============================================================================
// Required Rule
// ============================================================================

/// Validates that a field is present and not empty
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "username" => vec![Box::new(RequiredRule)],
/// });
/// ```
pub struct RequiredRule;

#[async_trait]
impl Rule for RequiredRule {
    fn name(&self) -> &str {
        "required"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        match value {
            Value::Null => Err(self.message()),
            Value::String(s) if s.trim().is_empty() => Err(self.message()),
            Value::Array(arr) if arr.is_empty() => Err(self.message()),
            Value::Object(obj) if obj.is_empty() => Err(self.message()),
            _ => Ok(()),
        }
    }

    fn message(&self) -> String {
        "This field is required".to_string()
    }
}

// ============================================================================
// String Type Rule
// ============================================================================

/// Validates that a value is a string
pub struct StringRule;

#[async_trait]
impl Rule for StringRule {
    fn name(&self) -> &str {
        "string"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        if value.is_string() {
            Ok(())
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        "This field must be a string".to_string()
    }
}

// ============================================================================
// Email Rule
// ============================================================================

/// Validates that a string is a valid email address
///
/// Uses a comprehensive regex pattern that covers most email formats.
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "email" => vec![Box::new(EmailRule)],
/// });
/// ```
pub struct EmailRule;

// Compiled once at first use; re-used on every subsequent validation call.
// Previously the regex was compiled on EVERY call — a measured 10–40 µs cliff
// per validation (see docs/PERFORMANCE.md).  File: crates/rf-validation/src/rules/string.rs
static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();

impl EmailRule {
    #[inline]
    fn email_regex() -> &'static Regex {
        EMAIL_REGEX.get_or_init(|| {
            // Requires at least one dot in domain (TLD)
            Regex::new(
                r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$"
            ).unwrap()
        })
    }
}

#[async_trait]
impl Rule for EmailRule {
    fn name(&self) -> &str {
        "email"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if Self::email_regex().is_match(s) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must be a valid email address".to_string()
    }
}

// ============================================================================
// URL Rule
// ============================================================================

/// Validates that a string is a valid URL
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "website" => vec![Box::new(UrlRule)],
/// });
/// ```
pub struct UrlRule;

static URL_REGEX: OnceLock<Regex> = OnceLock::new();

impl UrlRule {
    #[inline]
    fn url_regex() -> &'static Regex {
        URL_REGEX.get_or_init(|| {
            Regex::new(
                r"^https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&/=]*)$"
            ).unwrap()
        })
    }
}

#[async_trait]
impl Rule for UrlRule {
    fn name(&self) -> &str {
        "url"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if Self::url_regex().is_match(s) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must be a valid URL".to_string()
    }
}

// ============================================================================
// IP Address Rule
// ============================================================================

/// Validates that a string is a valid IP address (v4 or v6)
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "ip_address" => vec![Box::new(IpRule)],
/// });
/// ```
pub struct IpRule;

static IPV4_REGEX: OnceLock<Regex> = OnceLock::new();
static IPV6_REGEX: OnceLock<Regex> = OnceLock::new();

impl IpRule {
    #[inline]
    fn ipv4_regex() -> &'static Regex {
        IPV4_REGEX.get_or_init(|| {
            Regex::new(
                r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"
            ).unwrap()
        })
    }

    #[inline]
    fn ipv6_regex() -> &'static Regex {
        IPV6_REGEX.get_or_init(|| {
            Regex::new(
                r"^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$"
            ).unwrap()
        })
    }
}

#[async_trait]
impl Rule for IpRule {
    fn name(&self) -> &str {
        "ip"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if Self::ipv4_regex().is_match(s) || Self::ipv6_regex().is_match(s) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must be a valid IP address".to_string()
    }
}

// ============================================================================
// UUID Rule
// ============================================================================

/// Validates that a string is a valid UUID
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "id" => vec![Box::new(UuidRule)],
/// });
/// ```
pub struct UuidRule;

static UUID_REGEX: OnceLock<Regex> = OnceLock::new();

impl UuidRule {
    #[inline]
    fn uuid_regex() -> &'static Regex {
        UUID_REGEX.get_or_init(|| {
            Regex::new(
                r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
            )
            .unwrap()
        })
    }
}

#[async_trait]
impl Rule for UuidRule {
    fn name(&self) -> &str {
        "uuid"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if Self::uuid_regex().is_match(s) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must be a valid UUID".to_string()
    }
}

// ============================================================================
// Length Rules
// ============================================================================

/// Validates minimum string length
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "password" => vec![Box::new(MinLengthRule::new(8))],
/// });
/// ```
pub struct MinLengthRule {
    min: usize,
}

impl MinLengthRule {
    pub fn new(min: usize) -> Self {
        Self { min }
    }
}

#[async_trait]
impl Rule for MinLengthRule {
    fn name(&self) -> &str {
        "min_length"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s.chars().count() >= self.min => Ok(()),
            Some(s) => Err(format!(
                "This field must be at least {} characters (currently {})",
                self.min,
                s.chars().count()
            )),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        format!("This field must be at least {} characters", self.min)
    }
}

/// Validates maximum string length
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "username" => vec![Box::new(MaxLengthRule::new(50))],
/// });
/// ```
pub struct MaxLengthRule {
    max: usize,
}

impl MaxLengthRule {
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}

#[async_trait]
impl Rule for MaxLengthRule {
    fn name(&self) -> &str {
        "max_length"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s.chars().count() <= self.max => Ok(()),
            Some(s) => Err(format!(
                "This field must not exceed {} characters (currently {})",
                self.max,
                s.chars().count()
            )),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        format!("This field must not exceed {} characters", self.max)
    }
}

/// Validates string length is between min and max
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "bio" => vec![Box::new(BetweenLengthRule::new(10, 500))],
/// });
/// ```
pub struct BetweenLengthRule {
    min: usize,
    max: usize,
}

impl BetweenLengthRule {
    pub fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

#[async_trait]
impl Rule for BetweenLengthRule {
    fn name(&self) -> &str {
        "between_length"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s.len() >= self.min && s.len() <= self.max => Ok(()),
            Some(s) => Err(format!(
                "This field must be between {} and {} characters (currently {})",
                self.min,
                self.max,
                s.len()
            )),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        format!(
            "This field must be between {} and {} characters",
            self.min, self.max
        )
    }
}

// ============================================================================
// Pattern Rules
// ============================================================================

/// Validates string starts with a specific prefix
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "username" => vec![Box::new(StartsWithRule::new("user_"))],
/// });
/// ```
pub struct StartsWithRule {
    prefix: String,
}

impl StartsWithRule {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

#[async_trait]
impl Rule for StartsWithRule {
    fn name(&self) -> &str {
        "starts_with"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s.starts_with(&self.prefix) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        format!("This field must start with '{}'", self.prefix)
    }
}

/// Validates string ends with a specific suffix
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "email" => vec![Box::new(EndsWithRule::new("@example.com"))],
/// });
/// ```
pub struct EndsWithRule {
    suffix: String,
}

impl EndsWithRule {
    pub fn new(suffix: impl Into<String>) -> Self {
        Self {
            suffix: suffix.into(),
        }
    }
}

#[async_trait]
impl Rule for EndsWithRule {
    fn name(&self) -> &str {
        "ends_with"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s.ends_with(&self.suffix) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        format!("This field must end with '{}'", self.suffix)
    }
}

/// Validates string matches a regex pattern
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "zipcode" => vec![Box::new(RegexRule::new(r"^\d{5}$"))],
/// });
/// ```
pub struct RegexRule {
    pattern: Regex,
    pattern_str: String,
}

impl RegexRule {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            pattern: Regex::new(pattern)?,
            pattern_str: pattern.to_string(),
        })
    }
}

#[async_trait]
impl Rule for RegexRule {
    fn name(&self) -> &str {
        "regex"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if self.pattern.is_match(s) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        format!("This field must match the pattern: {}", self.pattern_str)
    }
}

// ============================================================================
// Character Type Rules
// ============================================================================

/// Validates string contains only alphabetic characters
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "name" => vec![Box::new(AlphaRule)],
/// });
/// ```
pub struct AlphaRule;

#[async_trait]
impl Rule for AlphaRule {
    fn name(&self) -> &str {
        "alpha"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if !s.is_empty() && s.chars().all(|c| c.is_alphabetic()) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must contain only letters".to_string()
    }
}

/// Validates string contains only alphanumeric characters
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "username" => vec![Box::new(AlphaNumericRule)],
/// });
/// ```
pub struct AlphaNumericRule;

#[async_trait]
impl Rule for AlphaNumericRule {
    fn name(&self) -> &str {
        "alpha_numeric"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric()) => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must contain only letters and numbers".to_string()
    }
}

/// Validates string is all lowercase
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "slug" => vec![Box::new(LowercaseRule)],
/// });
/// ```
pub struct LowercaseRule;

#[async_trait]
impl Rule for LowercaseRule {
    fn name(&self) -> &str {
        "lowercase"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s == s.to_lowercase() => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must be lowercase".to_string()
    }
}

/// Validates string is all uppercase
///
/// # Example
///
/// ```ignore
/// validator.rules(hashmap! {
///     "code" => vec![Box::new(UppercaseRule)],
/// });
/// ```
pub struct UppercaseRule;

#[async_trait]
impl Rule for UppercaseRule {
    fn name(&self) -> &str {
        "uppercase"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        match value.as_str() {
            Some(s) if s == s.to_uppercase() => Ok(()),
            Some(_) => Err(self.message()),
            None => Err("Value must be a string".to_string()),
        }
    }

    fn message(&self) -> String {
        "This field must be uppercase".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_required_rule() {
        let rule = RequiredRule;

        assert!(rule.validate(&json!(null), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(""), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!("  "), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!([]), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!({}), &HashMap::new()).await.is_err());

        assert!(rule.validate(&json!("test"), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(123), &HashMap::new()).await.is_ok());
    }

    #[tokio::test]
    async fn test_string_rule() {
        let rule = StringRule;

        assert!(rule.validate(&json!(null), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!("test"), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!(123), &HashMap::new()).await.is_err());
        assert!(rule.validate(&json!(true), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_email_rule() {
        let rule = EmailRule;

        assert!(rule
            .validate(&json!("user@example.com"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("user+tag@example.co.uk"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("invalid@email"), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!("not-an-email"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_url_rule() {
        let rule = UrlRule;

        assert!(rule
            .validate(&json!("https://example.com"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("http://example.com/path"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("not-a-url"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ip_rule() {
        let rule = IpRule;

        assert!(rule
            .validate(&json!("192.168.1.1"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(
                &json!("2001:0db8:85a3:0000:0000:8a2e:0370:7334"),
                &HashMap::new()
            )
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("999.999.999.999"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_uuid_rule() {
        let rule = UuidRule;

        assert!(rule
            .validate(
                &json!("550e8400-e29b-41d4-a716-446655440000"),
                &HashMap::new()
            )
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("not-a-uuid"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_min_length_rule() {
        let rule = MinLengthRule::new(5);

        assert!(rule
            .validate(&json!("hello"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule.validate(&json!("hi"), &HashMap::new()).await.is_err());
    }

    #[tokio::test]
    async fn test_max_length_rule() {
        let rule = MaxLengthRule::new(5);

        assert!(rule
            .validate(&json!("hello"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("toolong"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_between_length_rule() {
        let rule = BetweenLengthRule::new(3, 8);

        assert!(rule.validate(&json!("test"), &HashMap::new()).await.is_ok());
        assert!(rule.validate(&json!("hi"), &HashMap::new()).await.is_err());
        assert!(rule
            .validate(&json!("waytoolong"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_starts_with_rule() {
        let rule = StartsWithRule::new("user_");

        assert!(rule
            .validate(&json!("user_123"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("admin_123"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_ends_with_rule() {
        let rule = EndsWithRule::new(".com");

        assert!(rule
            .validate(&json!("example.com"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("example.org"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_regex_rule() {
        let rule = RegexRule::new(r"^\d{5}$").unwrap();

        assert!(rule
            .validate(&json!("12345"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("1234"), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!("abcde"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_alpha_rule() {
        let rule = AlphaRule;

        assert!(rule
            .validate(&json!("Hello"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("Hello123"), &HashMap::new())
            .await
            .is_err());
        assert!(rule
            .validate(&json!("Hello World"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_alpha_numeric_rule() {
        let rule = AlphaNumericRule;

        assert!(rule
            .validate(&json!("Hello123"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("Hello 123"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_lowercase_rule() {
        let rule = LowercaseRule;

        assert!(rule
            .validate(&json!("hello"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("Hello"), &HashMap::new())
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_uppercase_rule() {
        let rule = UppercaseRule;

        assert!(rule
            .validate(&json!("HELLO"), &HashMap::new())
            .await
            .is_ok());
        assert!(rule
            .validate(&json!("Hello"), &HashMap::new())
            .await
            .is_err());
    }
}
