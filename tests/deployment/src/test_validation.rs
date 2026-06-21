//! Deployment tests for rf-validation

#[cfg(test)]
mod tests {
    use rf_validation::{
        FieldError, ValidationErrors, Rule, ValidatedData, Validator,
        RulesBuilder, MessagesBuilder,
    };
    use rf_validation::rules::string::*;
    use rf_validation::rules::numeric::*;
    use rf_validation::rules::array::*;
    use rf_validation::rules::date::*;
    use rf_validation::rules::conditional::*;
    use rf_validation::validators::{email, url, ip, uuid};
    use serde_json::json;
    use std::collections::HashMap;

    // ── ValidationErrors ─────────────────────────────────────────

    #[test]
    fn validation_errors_creation_and_access() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());

        errors.add("email", FieldError::new("required", "Email is required"));
        assert!(!errors.is_empty());
        assert!(errors.get("email").is_some());
        assert!(errors.get("name").is_none());
    }

    #[test]
    fn field_error_with_params() {
        let err = FieldError::new("min", "Must be at least 3 characters")
            .with_param("min", 3);
        assert_eq!(err.code, "min");
    }

    #[test]
    fn validation_errors_serialization() {
        let mut errors = ValidationErrors::new();
        errors.add("name", FieldError::new("required", "Name is required"));
        let json = serde_json::to_string(&errors).expect("serialize");
        assert!(json.contains("name"));
        assert!(json.contains("required"));
    }

    // ── ValidatedData ────────────────────────────────────────────

    #[test]
    fn validated_data_accessors() {
        let mut data = HashMap::new();
        data.insert("name".into(), json!("John"));
        data.insert("age".into(), json!(30));
        data.insert("active".into(), json!(true));
        data.insert("score".into(), json!(9.5));

        let validated = ValidatedData::new(data);
        assert_eq!(validated.get_string("name"), Some("John".to_string()));
        assert_eq!(validated.get_i64("age"), Some(30));
        assert_eq!(validated.get_bool("active"), Some(true));
        assert_eq!(validated.get_f64("score"), Some(9.5));
        assert!(validated.get("nonexistent").is_none());
    }

    // ── Validator ────────────────────────────────────────────────

    #[tokio::test]
    async fn validator_passes_valid_data() {
        let mut data = HashMap::new();
        data.insert("email".into(), json!("test@example.com"));
        data.insert("name".into(), json!("John"));

        let mut rules: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();
        rules.insert("email", vec![Box::new(RequiredRule), Box::new(EmailRule)]);
        rules.insert("name", vec![Box::new(RequiredRule), Box::new(StringRule)]);

        let result = Validator::quick_validate(data, rules).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validator_fails_invalid_data() {
        let mut data = HashMap::new();
        data.insert("email".into(), json!("not-an-email"));

        let mut rules: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();
        rules.insert("email", vec![Box::new(EmailRule)]);

        let result = Validator::quick_validate(data, rules).await;
        assert!(result.is_err());
    }

    // ── String Rules ─────────────────────────────────────────────

    #[tokio::test]
    async fn required_rule() {
        let data = HashMap::new();
        assert!(RequiredRule.validate(&json!("hello"), &data).await.is_ok());
        assert!(RequiredRule.validate(&json!(""), &data).await.is_err());
        assert!(RequiredRule.validate(&json!(null), &data).await.is_err());
    }

    #[tokio::test]
    async fn email_rule() {
        let data = HashMap::new();
        assert!(EmailRule.validate(&json!("user@example.com"), &data).await.is_ok());
        assert!(EmailRule.validate(&json!("invalid"), &data).await.is_err());
    }

    #[tokio::test]
    async fn url_rule() {
        let data = HashMap::new();
        assert!(UrlRule.validate(&json!("https://example.com"), &data).await.is_ok());
        assert!(UrlRule.validate(&json!("not-a-url"), &data).await.is_err());
    }

    #[tokio::test]
    async fn min_max_length_rules() {
        let data = HashMap::new();
        let min = MinLengthRule::new(3);
        let max = MaxLengthRule::new(10);

        assert!(min.validate(&json!("abc"), &data).await.is_ok());
        assert!(min.validate(&json!("ab"), &data).await.is_err());
        assert!(max.validate(&json!("short"), &data).await.is_ok());
        assert!(max.validate(&json!("this is way too long"), &data).await.is_err());
    }

    #[tokio::test]
    async fn between_length_rule() {
        let data = HashMap::new();
        let rule = BetweenLengthRule::new(3, 8);
        assert!(rule.validate(&json!("hello"), &data).await.is_ok());
        assert!(rule.validate(&json!("hi"), &data).await.is_err());
        assert!(rule.validate(&json!("waytoolong"), &data).await.is_err());
    }

    #[tokio::test]
    async fn starts_ends_with_rules() {
        let data = HashMap::new();
        assert!(StartsWithRule::new("he").validate(&json!("hello"), &data).await.is_ok());
        assert!(StartsWithRule::new("wo").validate(&json!("hello"), &data).await.is_err());
        assert!(EndsWithRule::new("lo").validate(&json!("hello"), &data).await.is_ok());
        assert!(EndsWithRule::new("he").validate(&json!("hello"), &data).await.is_err());
    }

    #[tokio::test]
    async fn alpha_rules() {
        let data = HashMap::new();
        assert!(AlphaRule.validate(&json!("hello"), &data).await.is_ok());
        assert!(AlphaRule.validate(&json!("hello123"), &data).await.is_err());
        assert!(AlphaNumericRule.validate(&json!("hello123"), &data).await.is_ok());
        assert!(AlphaNumericRule.validate(&json!("hello!"), &data).await.is_err());
    }

    #[tokio::test]
    async fn case_rules() {
        let data = HashMap::new();
        assert!(LowercaseRule.validate(&json!("hello"), &data).await.is_ok());
        assert!(LowercaseRule.validate(&json!("Hello"), &data).await.is_err());
        assert!(UppercaseRule.validate(&json!("HELLO"), &data).await.is_ok());
        assert!(UppercaseRule.validate(&json!("Hello"), &data).await.is_err());
    }

    #[tokio::test]
    async fn regex_rule() {
        let data = HashMap::new();
        let rule = RegexRule::new(r"^\d{3}-\d{4}$").expect("valid regex");
        assert!(rule.validate(&json!("123-4567"), &data).await.is_ok());
        assert!(rule.validate(&json!("abc"), &data).await.is_err());
    }

    // ── Numeric Rules ────────────────────────────────────────────

    #[tokio::test]
    async fn integer_and_numeric_rules() {
        let data = HashMap::new();
        assert!(IntegerRule.validate(&json!(42), &data).await.is_ok());
        assert!(IntegerRule.validate(&json!(3.14), &data).await.is_err());
        assert!(NumericRule.validate(&json!(42), &data).await.is_ok());
        assert!(NumericRule.validate(&json!(3.14), &data).await.is_ok());
        assert!(NumericRule.validate(&json!("abc"), &data).await.is_err());
    }

    #[tokio::test]
    async fn min_max_between_numeric_rules() {
        let data = HashMap::new();
        assert!(MinRule::new(10).validate(&json!(15), &data).await.is_ok());
        assert!(MinRule::new(10).validate(&json!(5), &data).await.is_err());
        assert!(MaxRule::new(100).validate(&json!(50), &data).await.is_ok());
        assert!(MaxRule::new(100).validate(&json!(150), &data).await.is_err());
        assert!(BetweenRule::new(1, 10).validate(&json!(5), &data).await.is_ok());
        assert!(BetweenRule::new(1, 10).validate(&json!(15), &data).await.is_err());
    }

    #[tokio::test]
    async fn positive_negative_rules() {
        let data = HashMap::new();
        assert!(PositiveRule.validate(&json!(5), &data).await.is_ok());
        assert!(PositiveRule.validate(&json!(-1), &data).await.is_err());
        assert!(NegativeRule.validate(&json!(-5), &data).await.is_ok());
        assert!(NegativeRule.validate(&json!(1), &data).await.is_err());
    }

    // ── Array Rules ──────────────────────────────────────────────

    #[tokio::test]
    async fn array_rules() {
        let data = HashMap::new();
        assert!(ArrayRule.validate(&json!([1, 2, 3]), &data).await.is_ok());
        assert!(ArrayRule.validate(&json!("not array"), &data).await.is_err());
    }

    #[tokio::test]
    async fn in_not_in_rules() {
        let data = HashMap::new();
        let in_rule = InRule::from_strings(vec!["a", "b", "c"]);
        assert!(in_rule.validate(&json!("a"), &data).await.is_ok());
        assert!(in_rule.validate(&json!("d"), &data).await.is_err());

        let not_in = NotInRule::from_strings(vec!["x", "y"]);
        assert!(not_in.validate(&json!("a"), &data).await.is_ok());
        assert!(not_in.validate(&json!("x"), &data).await.is_err());
    }

    #[tokio::test]
    async fn array_size_rules() {
        let data = HashMap::new();
        assert!(MinArraySizeRule::new(2).validate(&json!([1, 2, 3]), &data).await.is_ok());
        assert!(MinArraySizeRule::new(5).validate(&json!([1, 2]), &data).await.is_err());
        assert!(MaxArraySizeRule::new(3).validate(&json!([1, 2]), &data).await.is_ok());
        assert!(MaxArraySizeRule::new(1).validate(&json!([1, 2, 3]), &data).await.is_err());
    }

    // ── Date Rules ───────────────────────────────────────────────

    #[tokio::test]
    async fn date_rule() {
        let data = HashMap::new();
        assert!(DateRule.validate(&json!("2024-01-15"), &data).await.is_ok());
        assert!(DateRule.validate(&json!("not-a-date"), &data).await.is_err());
    }

    #[tokio::test]
    async fn before_after_rules() {
        let data = HashMap::new();
        assert!(BeforeRule::new("2030-01-01").validate(&json!("2024-06-01"), &data).await.is_ok());
        assert!(AfterRule::new("2020-01-01").validate(&json!("2024-06-01"), &data).await.is_ok());
    }

    // ── Conditional Rules ────────────────────────────────────────

    #[tokio::test]
    async fn required_if_rule() {
        let mut data = HashMap::new();
        data.insert("type".into(), json!("business"));

        let rule = RequiredIfRule::new_string("type", "business");
        assert!(rule.validate(&json!("ACME Corp"), &data).await.is_ok());
        assert!(rule.validate(&json!(""), &data).await.is_err());
    }

    #[tokio::test]
    async fn same_different_rules() {
        let mut data = HashMap::new();
        data.insert("password".into(), json!("secret123"));

        let same = SameRule::new("password");
        assert!(same.validate(&json!("secret123"), &data).await.is_ok());
        assert!(same.validate(&json!("different"), &data).await.is_err());

        let diff = DifferentRule::new("password");
        assert!(diff.validate(&json!("other"), &data).await.is_ok());
        assert!(diff.validate(&json!("secret123"), &data).await.is_err());
    }

    // ── Validator Helpers ────────────────────────────────────────

    #[test]
    fn validate_email_helper() {
        assert!(email::validate_email("user@example.com"));
        assert!(!email::validate_email("not-email"));
    }

    #[test]
    fn validate_url_helper() {
        assert!(url::validate_url("https://example.com"));
        assert!(!url::validate_url("not a url"));
    }

    #[test]
    fn validate_ip_helper() {
        assert!(ip::validate_ip("192.168.1.1"));
        assert!(ip::validate_ip("::1"));
        assert!(!ip::validate_ip("999.999.999.999"));
    }

    #[test]
    fn validate_uuid_helper() {
        assert!(uuid::validate_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!uuid::validate_uuid("not-a-uuid"));
    }

    // ── Builders ─────────────────────────────────────────────────

    #[test]
    fn rules_builder() {
        let rules = RulesBuilder::new()
            .add("email", vec![Box::new(RequiredRule), Box::new(EmailRule)])
            .add("name", vec![Box::new(RequiredRule)])
            .build();

        assert!(rules.contains_key("email"));
        assert!(rules.contains_key("name"));
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn messages_builder() {
        let messages = MessagesBuilder::new()
            .add("email.required", "Bitte E-Mail eingeben")
            .add("name.required", "Name ist erforderlich")
            .build();

        assert_eq!(messages.len(), 2);
    }
}
