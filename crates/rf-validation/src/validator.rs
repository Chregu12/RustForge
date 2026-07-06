//! Core validation system
//!
//! Provides a flexible, rule-based validation system for validating arbitrary data
//! structures with customizable error messages.

use crate::error::{FieldError, ValidationErrors};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Result type for validation rules
pub type RuleResult = Result<(), String>;

/// Validated data wrapper
///
/// Contains the original data after successful validation.
#[derive(Debug, Clone)]
pub struct ValidatedData {
    pub data: HashMap<String, Value>,
}

impl ValidatedData {
    /// Create new validated data
    pub fn new(data: HashMap<String, Value>) -> Self {
        Self { data }
    }

    /// Get a field value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Get a field value as a string
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.data.get(key)?.as_str().map(String::from)
    }

    /// Get a field value as an i64
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.data.get(key)?.as_i64()
    }

    /// Get a field value as an f64
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.data.get(key)?.as_f64()
    }

    /// Get a field value as a boolean
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key)?.as_bool()
    }

    /// Get all data
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.data
    }
}

/// Validation rule trait
///
/// Implement this trait to create custom validation rules.
#[async_trait]
pub trait Rule: Send + Sync {
    /// Get the rule name (e.g., "required", "email", "min")
    fn name(&self) -> &str;

    /// Validate a value
    ///
    /// Returns Ok(()) if valid, Err(message) if invalid.
    /// The data parameter contains all fields being validated, allowing
    /// for cross-field validation.
    async fn validate(&self, value: &Value, data: &HashMap<String, Value>) -> RuleResult;

    /// Get the default error message for this rule
    fn message(&self) -> String;
}

/// The main validator struct
///
/// # Example
///
/// ```ignore
/// use rf_validation::Validator;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let mut data = HashMap::new();
/// data.insert("email".to_string(), json!("user@example.com"));
/// data.insert("age".to_string(), json!(25));
///
/// let mut validator = Validator::new(data);
///
/// validator.rules(hashmap! {
///     "email" => vec![
///         Box::new(RequiredRule),
///         Box::new(EmailRule),
///     ],
///     "age" => vec![
///         Box::new(IntegerRule),
///         Box::new(MinRule::new(18)),
///     ],
/// });
///
/// validator.messages(hashmap! {
///     "email.required" => "Email is required",
///     "age.min" => "You must be at least 18",
/// });
///
/// match validator.validate().await {
///     Ok(validated) => println!("Valid: {:?}", validated.all()),
///     Err(errors) => println!("Errors: {:?}", errors),
/// }
/// ```
pub struct Validator {
    /// The data being validated
    data: HashMap<String, Value>,

    /// Rules for each field
    rules: HashMap<String, Vec<Box<dyn Rule>>>,

    /// Custom error messages
    custom_messages: HashMap<String, String>,
}

impl Validator {
    /// Create a new validator with data to validate
    pub fn new(data: HashMap<String, Value>) -> Self {
        Self {
            data,
            rules: HashMap::new(),
            custom_messages: HashMap::new(),
        }
    }

    /// Add validation rules
    ///
    /// The key is the field name, and the value is a vector of rules to apply.
    pub fn rules(&mut self, rules: HashMap<&str, Vec<Box<dyn Rule>>>) -> &mut Self {
        for (field, field_rules) in rules {
            self.rules.insert(field.to_string(), field_rules);
        }
        self
    }

    /// Add custom error messages
    ///
    /// The key format is "field.rule" (e.g., "email.required", "age.min")
    pub fn messages(&mut self, messages: HashMap<&str, &str>) -> &mut Self {
        for (key, message) in messages {
            self.custom_messages
                .insert(key.to_string(), message.to_string());
        }
        self
    }

    /// Validate all fields
    ///
    /// Returns ValidatedData on success, or ValidationErrors on failure.
    pub async fn validate(&self) -> Result<ValidatedData, ValidationErrors> {
        let mut errors = ValidationErrors::new();

        for (field, rules) in &self.rules {
            let value = self.data.get(field).unwrap_or(&Value::Null);

            for rule in rules {
                match rule.validate(value, &self.data).await {
                    Ok(_) => continue,
                    Err(default_message) => {
                        // Check for custom message
                        let message_key = format!("{}.{}", field, rule.name());
                        let message = self
                            .custom_messages
                            .get(&message_key)
                            .cloned()
                            .unwrap_or(default_message);

                        errors.add(field, FieldError::new(rule.name(), message));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(ValidatedData::new(self.data.clone()))
        } else {
            Err(errors)
        }
    }

    /// Quick validate - convenience method for simple validation
    pub async fn quick_validate(
        data: HashMap<String, Value>,
        rules: HashMap<&str, Vec<Box<dyn Rule>>>,
    ) -> Result<ValidatedData, ValidationErrors> {
        let mut validator = Self::new(data);
        validator.rules(rules);
        validator.validate().await
    }
}

/// A convention-inferred field validation spec: `(field_name, type_keyword, required)`.
///
/// This is exactly the shape emitted by the `Model!` macro's
/// `VALIDATION_RULES` / `validation_rules()` (see `rf-macros`). The
/// `type_keyword` is one of `"string"`, `"integer"`, `"numeric"`, `"boolean"`,
/// or `""` (no type rule inferred); `required` reflects whether the declared
/// field was non-`Option`.
pub type FieldSpec = (&'static str, &'static str, bool);

/// Bridge convention-inferred field specs into REAL `rf_validation` rule-sets.
///
/// Translates each seeded `(name, type_keyword, required)` tuple produced by the
/// `Model!` macro into the concrete engine rules:
///   - `required == true`   -> [`RequiredRule`](crate::rules::RequiredRule)
///   - `"string"`           -> [`StringRule`](crate::rules::StringRule)
///   - `"integer"`          -> [`IntegerRule`](crate::rules::IntegerRule)
///   - `"numeric"`          -> [`NumericRule`](crate::rules::NumericRule)
///   - `""` / `"boolean"`   -> only requiredness (no built-in boolean type rule)
///
/// The returned map is ready to feed straight into [`Validator::rules`], so a
/// single model declaration drives real validation end to end:
///
/// ```ignore
/// let mut v = Validator::new(payload);
/// v.rules(rf_validation::rules_from_spec(CreateUser::VALIDATION_RULES));
/// v.validate().await?;
/// ```
pub fn rules_from_spec(spec: &[FieldSpec]) -> HashMap<&'static str, Vec<Box<dyn Rule>>> {
    use crate::rules::{IntegerRule, NumericRule, RequiredRule, StringRule};

    let mut map: HashMap<&'static str, Vec<Box<dyn Rule>>> = HashMap::new();
    for (field, keyword, required) in spec {
        let mut field_rules: Vec<Box<dyn Rule>> = Vec::new();
        if *required {
            field_rules.push(Box::new(RequiredRule));
        }
        match *keyword {
            "string" => field_rules.push(Box::new(StringRule)),
            "integer" => field_rules.push(Box::new(IntegerRule)),
            "numeric" => field_rules.push(Box::new(NumericRule)),
            // "boolean" and "" carry no built-in type rule; requiredness only.
            _ => {}
        }
        map.insert(*field, field_rules);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test rule
    struct AlwaysFailRule;

    #[async_trait]
    impl Rule for AlwaysFailRule {
        fn name(&self) -> &str {
            "always_fail"
        }

        async fn validate(&self, _value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
            Err("This always fails".to_string())
        }

        fn message(&self) -> String {
            "This always fails".to_string()
        }
    }

    struct AlwaysPassRule;

    #[async_trait]
    impl Rule for AlwaysPassRule {
        fn name(&self) -> &str {
            "always_pass"
        }

        async fn validate(&self, _value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
            Ok(())
        }

        fn message(&self) -> String {
            "This always passes".to_string()
        }
    }

    #[tokio::test]
    async fn test_validator_success() {
        let mut data = HashMap::new();
        data.insert("field1".to_string(), Value::String("test".to_string()));

        let mut validator = Validator::new(data);
        validator.rules(HashMap::from([(
            "field1",
            vec![Box::new(AlwaysPassRule) as Box<dyn Rule>],
        )]));

        let result = validator.validate().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validator_failure() {
        let mut data = HashMap::new();
        data.insert("field1".to_string(), Value::String("test".to_string()));

        let mut validator = Validator::new(data);
        validator.rules(HashMap::from([(
            "field1",
            vec![Box::new(AlwaysFailRule) as Box<dyn Rule>],
        )]));

        let result = validator.validate().await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(errors.get("field1").is_some());
    }

    #[tokio::test]
    async fn test_custom_messages() {
        let mut data = HashMap::new();
        data.insert("field1".to_string(), Value::String("test".to_string()));

        let mut validator = Validator::new(data);
        validator.rules(HashMap::from([(
            "field1",
            vec![Box::new(AlwaysFailRule) as Box<dyn Rule>],
        )]));
        validator.messages(HashMap::from([(
            "field1.always_fail",
            "Custom error message",
        )]));

        let result = validator.validate().await;
        assert!(result.is_err());

        let errors = result.unwrap_err();
        let field_errors = errors.get("field1").unwrap();
        assert_eq!(field_errors[0].message, "Custom error message");
    }

    #[tokio::test]
    async fn test_validated_data_accessors() {
        let mut data = HashMap::new();
        data.insert(
            "string_field".to_string(),
            Value::String("test".to_string()),
        );
        data.insert("int_field".to_string(), Value::Number(42.into()));
        data.insert("bool_field".to_string(), Value::Bool(true));

        let validated = ValidatedData::new(data);

        assert_eq!(
            validated.get_string("string_field"),
            Some("test".to_string())
        );
        assert_eq!(validated.get_i64("int_field"), Some(42));
        assert_eq!(validated.get_bool("bool_field"), Some(true));
    }

    // Bridge from convention-inferred `(name, type_keyword, required)` specs
    // (as emitted by the `Model!` macro) into the real engine rule-sets.
    const CREATE_SPEC: &[FieldSpec] = &[
        ("title", "string", true),
        ("views", "integer", true),
        ("subtitle", "string", false), // Option<String> -> optional
    ];

    #[tokio::test]
    async fn rules_from_spec_accepts_valid_and_skips_optional() {
        let mut data = HashMap::new();
        data.insert("title".to_string(), Value::String("Hello".to_string()));
        data.insert("views".to_string(), Value::Number(42.into()));
        // `subtitle` omitted: optional, so absence is fine.

        let mut v = Validator::new(data);
        v.rules(rules_from_spec(CREATE_SPEC));
        assert!(v.validate().await.is_ok(), "valid data should pass");
    }

    #[tokio::test]
    async fn rules_from_spec_rejects_missing_required_and_wrong_type() {
        // Missing required `title`.
        let mut data = HashMap::new();
        data.insert("views".to_string(), Value::Number(1.into()));
        let mut v = Validator::new(data);
        v.rules(rules_from_spec(CREATE_SPEC));
        let errs = v.validate().await.expect_err("missing required must fail");
        assert!(errs.get("title").is_some());

        // Wrong type for inferred integer field.
        let mut data = HashMap::new();
        data.insert("title".to_string(), Value::String("Ok".to_string()));
        data.insert("views".to_string(), Value::String("nope".to_string()));
        let mut v = Validator::new(data);
        v.rules(rules_from_spec(CREATE_SPEC));
        let errs = v.validate().await.expect_err("wrong type must fail");
        assert!(errs.get("views").is_some());
    }
}
