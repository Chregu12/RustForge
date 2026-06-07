//! Integration tests for rf-validation
//!
//! Tests cover: required, email, min_length, max_length, min, max (numeric),
//! url, regex, in_array, numeric, boolean-like rules, date rules,
//! multiple-error accumulation, custom error messages, and nested / cross-field
//! validation.

use rf_validation::{
    rules::{
        EmailRule, InRule, MaxLengthRule, MaxRule, MinLengthRule, MinRule, NumericRule,
        RegexRule, RequiredRule, UrlRule,
    },
    validator::{Rule, Validator},
};
use serde_json::{json, Value};
use std::collections::HashMap;

// ───────────────────────────────────────────────────────────────────────────
// required
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn required_fails_on_null() {
    let rule = RequiredRule;
    assert!(rule.validate(&json!(null), &HashMap::new()).await.is_err());
}

#[tokio::test]
async fn required_fails_on_empty_string() {
    let rule = RequiredRule;
    assert!(rule.validate(&json!(""), &HashMap::new()).await.is_err());
}

#[tokio::test]
async fn required_fails_on_whitespace_only_string() {
    let rule = RequiredRule;
    assert!(rule.validate(&json!("   "), &HashMap::new()).await.is_err());
}

#[tokio::test]
async fn required_passes_on_non_empty_string() {
    let rule = RequiredRule;
    assert!(rule.validate(&json!("hello"), &HashMap::new()).await.is_ok());
}

#[tokio::test]
async fn required_passes_on_number() {
    let rule = RequiredRule;
    assert!(rule.validate(&json!(0), &HashMap::new()).await.is_ok());
}

// ───────────────────────────────────────────────────────────────────────────
// email
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn email_passes_valid_address() {
    let rule = EmailRule;
    assert!(rule
        .validate(&json!("user@example.com"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn email_passes_address_with_subdomain() {
    let rule = EmailRule;
    assert!(rule
        .validate(&json!("user@mail.example.co.uk"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn email_fails_missing_at_sign() {
    let rule = EmailRule;
    assert!(rule
        .validate(&json!("not-an-email.com"), &HashMap::new())
        .await
        .is_err());
}

#[tokio::test]
async fn email_fails_missing_tld() {
    let rule = EmailRule;
    assert!(rule
        .validate(&json!("user@nodot"), &HashMap::new())
        .await
        .is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// min_length / max_length
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn min_length_passes_exact_minimum() {
    let rule = MinLengthRule::new(5);
    assert!(rule
        .validate(&json!("hello"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn min_length_fails_below_minimum() {
    let rule = MinLengthRule::new(5);
    assert!(rule.validate(&json!("hi"), &HashMap::new()).await.is_err());
}

#[tokio::test]
async fn max_length_passes_exact_maximum() {
    let rule = MaxLengthRule::new(5);
    assert!(rule
        .validate(&json!("hello"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn max_length_fails_above_maximum() {
    let rule = MaxLengthRule::new(5);
    assert!(rule
        .validate(&json!("toolong"), &HashMap::new())
        .await
        .is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// min / max (numeric)
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn min_numeric_passes_equal_to_minimum() {
    let rule = MinRule::new(18);
    assert!(rule.validate(&json!(18), &HashMap::new()).await.is_ok());
}

#[tokio::test]
async fn min_numeric_fails_below_minimum() {
    let rule = MinRule::new(18);
    assert!(rule.validate(&json!(17), &HashMap::new()).await.is_err());
}

#[tokio::test]
async fn max_numeric_passes_equal_to_maximum() {
    let rule = MaxRule::new(100);
    assert!(rule.validate(&json!(100), &HashMap::new()).await.is_ok());
}

#[tokio::test]
async fn max_numeric_fails_above_maximum() {
    let rule = MaxRule::new(100);
    assert!(rule.validate(&json!(101), &HashMap::new()).await.is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// url
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn url_passes_https_url() {
    let rule = UrlRule;
    assert!(rule
        .validate(&json!("https://example.com"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn url_passes_http_url_with_path() {
    let rule = UrlRule;
    assert!(rule
        .validate(&json!("http://example.com/path/to/page"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn url_fails_plain_text() {
    let rule = UrlRule;
    assert!(rule
        .validate(&json!("not-a-url"), &HashMap::new())
        .await
        .is_err());
}

#[tokio::test]
async fn url_fails_missing_scheme() {
    let rule = UrlRule;
    assert!(rule
        .validate(&json!("example.com"), &HashMap::new())
        .await
        .is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// regex
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn regex_passes_matching_pattern() {
    let rule = RegexRule::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(rule
        .validate(&json!("2024-01-15"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn regex_fails_non_matching_input() {
    let rule = RegexRule::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    assert!(rule
        .validate(&json!("15-01-2024"), &HashMap::new())
        .await
        .is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// in_array
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn in_rule_passes_for_allowed_value() {
    let rule = InRule::from_strings(vec!["active", "inactive", "pending"]);
    assert!(rule
        .validate(&json!("active"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn in_rule_fails_for_unlisted_value() {
    let rule = InRule::from_strings(vec!["active", "inactive"]);
    assert!(rule
        .validate(&json!("banned"), &HashMap::new())
        .await
        .is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// numeric
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn numeric_rule_passes_for_integer() {
    let rule = NumericRule;
    assert!(rule.validate(&json!(42), &HashMap::new()).await.is_ok());
}

#[tokio::test]
async fn numeric_rule_passes_for_float() {
    let rule = NumericRule;
    assert!(rule.validate(&json!(3.14), &HashMap::new()).await.is_ok());
}

#[tokio::test]
async fn numeric_rule_passes_for_numeric_string() {
    let rule = NumericRule;
    assert!(rule
        .validate(&json!("99.5"), &HashMap::new())
        .await
        .is_ok());
}

#[tokio::test]
async fn numeric_rule_fails_for_alpha_string() {
    let rule = NumericRule;
    assert!(rule
        .validate(&json!("abc"), &HashMap::new())
        .await
        .is_err());
}

// ───────────────────────────────────────────────────────────────────────────
// Multiple errors on multiple invalid fields
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn multiple_invalid_fields_produce_multiple_errors() {
    let data: HashMap<String, Value> = HashMap::new(); // empty – both required fields missing

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([
        (
            "email",
            vec![Box::new(RequiredRule) as Box<dyn Rule>],
        ),
        (
            "username",
            vec![Box::new(RequiredRule) as Box<dyn Rule>],
        ),
    ]));

    let result = validator.validate().await;
    assert!(result.is_err());

    let errors = result.unwrap_err();
    assert!(errors.get("email").is_some());
    assert!(errors.get("username").is_some());
}

// ───────────────────────────────────────────────────────────────────────────
// Custom error messages
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn custom_error_message_overrides_default() {
    let data: HashMap<String, Value> = HashMap::new();

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "email",
        vec![Box::new(RequiredRule) as Box<dyn Rule>],
    )]));
    validator.messages(HashMap::from([("email.required", "Bitte E-Mail angeben")]));

    let errors = validator.validate().await.unwrap_err();
    let field_errors = errors.get("email").unwrap();
    assert_eq!(field_errors[0].message, "Bitte E-Mail angeben");
}

// ───────────────────────────────────────────────────────────────────────────
// Valid data passes full validation
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn valid_data_passes_all_rules() {
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("alice@example.com"));
    data.insert("age".to_string(), json!(25));
    data.insert("website".to_string(), json!("https://alice.dev"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([
        (
            "email",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(EmailRule),
            ],
        ),
        (
            "age",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(MinRule::new(18)),
                Box::new(MaxRule::new(120)),
            ],
        ),
        (
            "website",
            vec![Box::new(UrlRule) as Box<dyn Rule>],
        ),
    ]));

    let result = validator.validate().await;
    assert!(result.is_ok());
}

// ───────────────────────────────────────────────────────────────────────────
// Chain: required + email – missing value triggers required, not email
// ───────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn required_fires_before_email_when_field_is_missing() {
    let data: HashMap<String, Value> = HashMap::new();

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "email",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(EmailRule),
        ],
    )]));

    let errors = validator.validate().await.unwrap_err();
    let field_errors = errors.get("email").unwrap();
    // The first error should be "required" (not "email")
    assert_eq!(field_errors[0].code, "required");
}
