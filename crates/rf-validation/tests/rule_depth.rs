//! Deep integration tests for rf-validation rules.
//!
//! These tests target real behavioral constraints: boundary values, Unicode
//! correctness, error-message content, cross-field rules, short-circuit
//! semantics and coercion — areas the external audit flagged as untested.

use rf_validation::rules::{
    AlphaNumericRule, AlphaRule, BetweenLengthRule, BetweenRule, DifferentRule, DigitsBetweenRule,
    DigitsRule, EmailRule, InRule, IntegerRule, IpRule, LowercaseRule, MaxLengthRule, MaxRule,
    MinLengthRule, MinRule, NegativeRule, NotInRule, NumericRule, PositiveRule, RegexRule,
    RequiredIfRule, RequiredRule, RequiredUnlessRule, RequiredWithRule, RequiredWithoutRule,
    SameRule, StartsWithRule, UppercaseRule, UuidRule,
};
use rf_validation::validator::{Rule, Validator};
use serde_json::{json, Value};
use std::collections::HashMap;

// ============================================================================
// Helper
// ============================================================================

/// Build a validator with a single field and the given rules, then run it.
async fn validate_field(
    field: &'static str,
    value: Value,
    rules: Vec<Box<dyn Rule>>,
) -> Result<(), Vec<String>> {
    let mut data = HashMap::new();
    data.insert(field.to_string(), value);
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(field, rules)]));
    match v.validate().await {
        Ok(_) => Ok(()),
        Err(e) => Err(e
            .get(field)
            .map(|fe| fe.iter().map(|f| f.message.clone()).collect())
            .unwrap_or_default()),
    }
}

// ============================================================================
// RequiredRule — edge cases
// ============================================================================

#[tokio::test]
async fn required_rejects_null() {
    assert!(validate_field("f", json!(null), vec![Box::new(RequiredRule)])
        .await
        .is_err());
}

#[tokio::test]
async fn required_rejects_empty_string() {
    assert!(validate_field("f", json!(""), vec![Box::new(RequiredRule)])
        .await
        .is_err());
}

#[tokio::test]
async fn required_rejects_whitespace_only() {
    // "   " is all whitespace — treated as empty by the rule.
    assert!(
        validate_field("f", json!("   "), vec![Box::new(RequiredRule)])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn required_accepts_zero_and_false() {
    // Numeric 0 and boolean false are NOT empty — they must pass RequiredRule.
    assert!(validate_field("f", json!(0), vec![Box::new(RequiredRule)])
        .await
        .is_ok());
    assert!(
        validate_field("f", json!(false), vec![Box::new(RequiredRule)])
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn required_message_text() {
    let errs = validate_field("f", json!(null), vec![Box::new(RequiredRule)])
        .await
        .unwrap_err();
    assert!(
        !errs.is_empty(),
        "expected at least one error message"
    );
    // The message must mention the word "required".
    assert!(
        errs[0].to_lowercase().contains("required"),
        "error message should contain 'required', got: {}",
        errs[0]
    );
}

// ============================================================================
// Missing field (not in data) vs null
// ============================================================================

#[tokio::test]
async fn required_missing_field_same_as_null() {
    // When the field is absent from data entirely the validator falls back to
    // Value::Null — RequiredRule must still reject it.
    let data: HashMap<String, Value> = HashMap::new(); // "username" not present
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "username",
        vec![Box::new(RequiredRule) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_err(), "absent field must fail required");
}

// ============================================================================
// EmailRule — edge cases
// ============================================================================

#[tokio::test]
async fn email_rejects_missing_tld() {
    // "user@localhost" has no dot in the domain — invalid per the rule regex.
    assert!(
        validate_field("e", json!("user@localhost"), vec![Box::new(EmailRule)])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn email_rejects_double_at() {
    assert!(
        validate_field("e", json!("a@@example.com"), vec![Box::new(EmailRule)])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn email_accepts_plus_tag_and_subdomain() {
    assert!(
        validate_field(
            "e",
            json!("alice+tag@mail.example.co.uk"),
            vec![Box::new(EmailRule)]
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn email_rejects_plain_string() {
    assert!(
        validate_field("e", json!("not-an-email"), vec![Box::new(EmailRule)])
            .await
            .is_err()
    );
}

// ============================================================================
// MinLengthRule / MaxLengthRule — boundary values
// ============================================================================

#[tokio::test]
async fn min_length_boundary() {
    let rule_min5 = || vec![Box::new(MinLengthRule::new(5)) as Box<dyn Rule>];

    // Exactly at boundary.
    assert!(validate_field("s", json!("abcde"), rule_min5()).await.is_ok()); // len=5 ✓
    // One short.
    assert!(
        validate_field("s", json!("abcd"), rule_min5())
            .await
            .is_err()
    ); // len=4 ✗
    // Over boundary.
    assert!(
        validate_field("s", json!("abcdef"), rule_min5())
            .await
            .is_ok()
    ); // len=6 ✓
}

#[tokio::test]
async fn max_length_boundary() {
    let rule_max5 = || vec![Box::new(MaxLengthRule::new(5)) as Box<dyn Rule>];

    assert!(validate_field("s", json!("abcde"), rule_max5()).await.is_ok()); // len=5 ✓
    assert!(
        validate_field("s", json!("abcdef"), rule_max5())
            .await
            .is_err()
    ); // len=6 ✗
    assert!(validate_field("s", json!("abc"), rule_max5()).await.is_ok()); // len=3 ✓
}

// ============================================================================
// BetweenLengthRule — Unicode / multibyte correctness (regression for byte bug)
// ============================================================================

#[tokio::test]
async fn between_length_uses_char_count_not_bytes() {
    // "café" = 4 Unicode chars, but 5 UTF-8 bytes.
    // BetweenLengthRule previously used s.len() (bytes), so BetweenLengthRule(4,4)
    // would see 5 bytes and incorrectly REJECT "café", even though
    // MinLengthRule(4) and MaxLengthRule(4) both accept it.
    // After the fix (chars().count()), all three must agree.
    let s = "café"; // 4 chars, 5 bytes

    assert!(
        validate_field(
            "s",
            json!(s),
            vec![Box::new(BetweenLengthRule::new(4, 4)) as Box<dyn Rule>]
        )
        .await
        .is_ok(),
        "BetweenLengthRule(4,4) must accept 'café' (4 chars, 5 bytes)"
    );

    // Also verify MinLength and MaxLength agree.
    assert!(
        validate_field(
            "s",
            json!(s),
            vec![Box::new(MinLengthRule::new(4)) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
    assert!(
        validate_field(
            "s",
            json!(s),
            vec![Box::new(MaxLengthRule::new(4)) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn between_length_with_emoji() {
    // Emoji are typically 4 bytes in UTF-8 but 1 or 2 Unicode chars (some are
    // surrogate pairs in UTF-16 but single code points in UTF-8).
    // "🦀🦀🦀" = 3 code points, 12 bytes.
    let s = "🦀🦀🦀";
    assert_eq!(s.chars().count(), 3);
    assert!(
        validate_field(
            "s",
            json!(s),
            vec![Box::new(BetweenLengthRule::new(3, 3)) as Box<dyn Rule>]
        )
        .await
        .is_ok(),
        "3 emoji code points must satisfy BetweenLengthRule(3,3)"
    );
    // Would fail with old byte-based check (12 bytes > 3).
    assert!(
        validate_field(
            "s",
            json!(s),
            vec![Box::new(BetweenLengthRule::new(1, 2)) as Box<dyn Rule>]
        )
        .await
        .is_err(),
        "3 emoji must fail BetweenLengthRule(1,2)"
    );
}

// ============================================================================
// NumericRule / IntegerRule — coercion from string
// ============================================================================

#[tokio::test]
async fn integer_rule_accepts_string_integer_coercion() {
    // A JSON string "42" that represents a valid integer is accepted by IntegerRule
    // (Laravel-style coercion).
    assert!(
        validate_field("n", json!("42"), vec![Box::new(IntegerRule) as Box<dyn Rule>])
            .await
            .is_ok()
    );
    // But "42.5" (fractional) is rejected.
    assert!(
        validate_field(
            "n",
            json!("42.5"),
            vec![Box::new(IntegerRule) as Box<dyn Rule>]
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn numeric_rule_accepts_string_float_coercion() {
    assert!(
        validate_field(
            "n",
            json!("3.14"),
            vec![Box::new(NumericRule) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
    assert!(
        validate_field(
            "n",
            json!("not-a-number"),
            vec![Box::new(NumericRule) as Box<dyn Rule>]
        )
        .await
        .is_err()
    );
}

// ============================================================================
// MinRule / MaxRule — Laravel size semantics
// ============================================================================

#[tokio::test]
async fn min_rule_uses_numeric_value_not_string_coercion() {
    // MinRule(18) on a string "25" checks string *length* (2 chars), not numeric
    // value. This follows Laravel's size semantics for mixed types.
    let res = validate_field("v", json!("25"), vec![Box::new(MinRule::new(18)) as Box<dyn Rule>])
        .await;
    // "25" has 2 chars, which is < 18 → FAIL
    assert!(res.is_err(), "string '25' has length 2, which is below min 18");
}

#[tokio::test]
async fn min_max_rule_on_numeric_value() {
    // On a numeric JSON value, MinRule / MaxRule compare the numeric value.
    assert!(
        validate_field("v", json!(18), vec![Box::new(MinRule::new(18)) as Box<dyn Rule>])
            .await
            .is_ok()
    );
    assert!(
        validate_field("v", json!(17), vec![Box::new(MinRule::new(18)) as Box<dyn Rule>])
            .await
            .is_err()
    );
    assert!(
        validate_field("v", json!(100), vec![Box::new(MaxRule::new(99)) as Box<dyn Rule>])
            .await
            .is_err()
    );
    assert!(
        validate_field("v", json!(99), vec![Box::new(MaxRule::new(99)) as Box<dyn Rule>])
            .await
            .is_ok()
    );
}

// ============================================================================
// BetweenRule — boundary inclusiveness
// ============================================================================

#[tokio::test]
async fn between_rule_inclusive_boundaries() {
    let rules = || vec![Box::new(BetweenRule::new(1, 10)) as Box<dyn Rule>];

    assert!(validate_field("v", json!(1), rules()).await.is_ok()); // lower bound ✓
    assert!(validate_field("v", json!(10), rules()).await.is_ok()); // upper bound ✓
    assert!(validate_field("v", json!(0), rules()).await.is_err()); // below ✗
    assert!(validate_field("v", json!(11), rules()).await.is_err()); // above ✗
}

// ============================================================================
// PositiveRule / NegativeRule — zero boundary
// ============================================================================

#[tokio::test]
async fn positive_rule_zero_is_not_positive() {
    assert!(
        validate_field("v", json!(0), vec![Box::new(PositiveRule) as Box<dyn Rule>])
            .await
            .is_err(),
        "0 is not positive"
    );
    assert!(
        validate_field("v", json!(0.001), vec![Box::new(PositiveRule) as Box<dyn Rule>])
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn negative_rule_zero_is_not_negative() {
    assert!(
        validate_field("v", json!(0), vec![Box::new(NegativeRule) as Box<dyn Rule>])
            .await
            .is_err(),
        "0 is not negative"
    );
    assert!(
        validate_field("v", json!(-0.001), vec![Box::new(NegativeRule) as Box<dyn Rule>])
            .await
            .is_ok()
    );
}

// ============================================================================
// InRule — strict type matching
// ============================================================================

#[tokio::test]
async fn in_rule_does_not_coerce_string_to_int() {
    // InRule uses JSON Value equality. "1" (string) is NOT the same as 1 (int).
    let rules = || {
        vec![Box::new(InRule::from_ints(vec![1, 2, 3])) as Box<dyn Rule>]
    };
    // Integer 1 → in list → OK
    assert!(validate_field("v", json!(1), rules()).await.is_ok());
    // String "1" → not in list of integers → ERR
    assert!(
        validate_field("v", json!("1"), rules()).await.is_err(),
        "string '1' must not match integer 1 in InRule"
    );
}

#[tokio::test]
async fn in_rule_error_message_lists_allowed_values() {
    let errs = validate_field(
        "status",
        json!("deleted"),
        vec![Box::new(InRule::from_strings(vec!["active", "inactive"])) as Box<dyn Rule>],
    )
    .await
    .unwrap_err();
    assert!(!errs.is_empty());
    // Message should mention the allowed values.
    assert!(
        errs[0].contains("active") || errs[0].contains("inactive"),
        "error message should list allowed values: {}",
        errs[0]
    );
}

// ============================================================================
// NotInRule — reserved names
// ============================================================================

#[tokio::test]
async fn not_in_rule_rejects_reserved_usernames() {
    let rules = || {
        vec![
            Box::new(NotInRule::from_strings(vec!["admin", "root", "system"])) as Box<dyn Rule>,
        ]
    };
    assert!(validate_field("u", json!("alice"), rules()).await.is_ok());
    assert!(validate_field("u", json!("admin"), rules()).await.is_err());
    assert!(validate_field("u", json!("root"), rules()).await.is_err());
}

// ============================================================================
// RegexRule — anchored pattern
// ============================================================================

#[tokio::test]
async fn regex_rule_anchored_pattern() {
    // US ZIP code: exactly 5 digits.
    let rules = || vec![Box::new(RegexRule::new(r"^\d{5}$").unwrap()) as Box<dyn Rule>];
    assert!(validate_field("zip", json!("12345"), rules()).await.is_ok());
    assert!(
        validate_field("zip", json!("1234"), rules())
            .await
            .is_err(),
        "4 digits must fail"
    );
    assert!(
        validate_field("zip", json!("123456"), rules())
            .await
            .is_err(),
        "6 digits must fail"
    );
    assert!(
        validate_field("zip", json!("1234a"), rules())
            .await
            .is_err(),
        "non-digit must fail"
    );
}

// ============================================================================
// AlphaRule / AlphaNumericRule — empty string edge case
// ============================================================================

#[tokio::test]
async fn alpha_rule_rejects_empty_string() {
    // AlphaRule requires at least one alpha char, so empty string is invalid.
    assert!(
        validate_field("v", json!(""), vec![Box::new(AlphaRule) as Box<dyn Rule>])
            .await
            .is_err(),
        "empty string has no alpha chars"
    );
}

#[tokio::test]
async fn alphanumeric_rule_rejects_spaces() {
    assert!(
        validate_field(
            "v",
            json!("hello world"),
            vec![Box::new(AlphaNumericRule) as Box<dyn Rule>]
        )
        .await
        .is_err()
    );
}

// ============================================================================
// LowercaseRule / UppercaseRule — numeric chars pass through
// ============================================================================

#[tokio::test]
async fn lowercase_rule_digits_pass() {
    // "hello123" contains only lowercase letters and digits — it should pass
    // because "hello123".to_lowercase() == "hello123".
    assert!(
        validate_field(
            "v",
            json!("hello123"),
            vec![Box::new(LowercaseRule) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn uppercase_rule_digits_pass() {
    assert!(
        validate_field(
            "v",
            json!("HELLO123"),
            vec![Box::new(UppercaseRule) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
}

// ============================================================================
// IpRule — boundary addresses
// ============================================================================

#[tokio::test]
async fn ip_rule_accepts_all_zeros_and_broadcast() {
    assert!(
        validate_field("ip", json!("0.0.0.0"), vec![Box::new(IpRule) as Box<dyn Rule>])
            .await
            .is_ok()
    );
    assert!(
        validate_field(
            "ip",
            json!("255.255.255.255"),
            vec![Box::new(IpRule) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn ip_rule_rejects_out_of_range_octet() {
    assert!(
        validate_field(
            "ip",
            json!("256.0.0.1"),
            vec![Box::new(IpRule) as Box<dyn Rule>]
        )
        .await
        .is_err()
    );
}

// ============================================================================
// UuidRule — case-insensitive and all-zeros
// ============================================================================

#[tokio::test]
async fn uuid_rule_accepts_nil_uuid() {
    assert!(
        validate_field(
            "id",
            json!("00000000-0000-0000-0000-000000000000"),
            vec![Box::new(UuidRule) as Box<dyn Rule>]
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn uuid_rule_rejects_wrong_length() {
    assert!(
        validate_field(
            "id",
            json!("550e8400-e29b-41d4-a716-44665544000"), // one char short
            vec![Box::new(UuidRule) as Box<dyn Rule>]
        )
        .await
        .is_err()
    );
}

// ============================================================================
// DigitsRule / DigitsBetweenRule — boundary counts
// ============================================================================

#[tokio::test]
async fn digits_rule_exact_count() {
    let rules = || vec![Box::new(DigitsRule::new(5)) as Box<dyn Rule>];
    assert!(validate_field("d", json!("12345"), rules()).await.is_ok());
    assert!(validate_field("d", json!("1234"), rules()).await.is_err()); // 4 digits
    assert!(validate_field("d", json!("123456"), rules()).await.is_err()); // 6 digits
}

#[tokio::test]
async fn digits_between_rule_inclusive_boundaries() {
    let rules = || vec![Box::new(DigitsBetweenRule::new(3, 5)) as Box<dyn Rule>];
    assert!(validate_field("d", json!("123"), rules()).await.is_ok()); // min ✓
    assert!(validate_field("d", json!("12345"), rules()).await.is_ok()); // max ✓
    assert!(validate_field("d", json!("12"), rules()).await.is_err()); // below ✗
    assert!(validate_field("d", json!("123456"), rules()).await.is_err()); // above ✗
}

// ============================================================================
// StartsWithRule
// ============================================================================

#[tokio::test]
async fn starts_with_rule_checks_prefix() {
    let rules = || vec![Box::new(StartsWithRule::new("https://")) as Box<dyn Rule>];
    assert!(
        validate_field("u", json!("https://example.com"), rules())
            .await
            .is_ok()
    );
    assert!(
        validate_field("u", json!("http://example.com"), rules())
            .await
            .is_err()
    );
}

// ============================================================================
// SameRule / DifferentRule — cross-field password patterns
// ============================================================================

#[tokio::test]
async fn same_rule_password_confirmation_match() {
    let mut data = HashMap::new();
    data.insert("password".to_string(), json!("super_secret"));
    data.insert("password_confirm".to_string(), json!("super_secret"));
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "password_confirm",
        vec![Box::new(SameRule::new("password")) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_ok());
}

#[tokio::test]
async fn same_rule_password_confirmation_mismatch() {
    let mut data = HashMap::new();
    data.insert("password".to_string(), json!("super_secret"));
    data.insert("password_confirm".to_string(), json!("different"));
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "password_confirm",
        vec![Box::new(SameRule::new("password")) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_err());
}

#[tokio::test]
async fn different_rule_new_password_must_change() {
    let mut data = HashMap::new();
    data.insert("old_password".to_string(), json!("password123"));
    data.insert("new_password".to_string(), json!("password123")); // same as old
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "new_password",
        vec![Box::new(DifferentRule::new("old_password")) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_err(), "new == old must fail DifferentRule");

    // Change new_password to something different.
    let mut data2 = HashMap::new();
    data2.insert("old_password".to_string(), json!("password123"));
    data2.insert("new_password".to_string(), json!("newpassword456"));
    let mut v2 = Validator::new(data2);
    v2.rules(HashMap::from([(
        "new_password",
        vec![Box::new(DifferentRule::new("old_password")) as Box<dyn Rule>],
    )]));
    assert!(v2.validate().await.is_ok());
}

// ============================================================================
// RequiredIf / RequiredUnless / RequiredWith / RequiredWithout
// ============================================================================

#[tokio::test]
async fn required_if_field_required_only_when_condition_met() {
    // "billing_address" is required when "pay_method" == "invoice"
    let rule = RequiredIfRule::new_string("pay_method", "invoice");

    // Condition met, field absent → must fail.
    let mut data = HashMap::new();
    data.insert("pay_method".to_string(), json!("invoice"));
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "billing_address",
        vec![Box::new(rule) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_err());

    // Condition not met, field absent → must pass.
    let rule2 = RequiredIfRule::new_string("pay_method", "invoice");
    let mut data2 = HashMap::new();
    data2.insert("pay_method".to_string(), json!("card"));
    let mut v2 = Validator::new(data2);
    v2.rules(HashMap::from([(
        "billing_address",
        vec![Box::new(rule2) as Box<dyn Rule>],
    )]));
    assert!(v2.validate().await.is_ok());
}

#[tokio::test]
async fn required_unless_field_optional_when_unless_condition_met() {
    // "reason" is required unless "status" is "approved".
    let rule = RequiredUnlessRule::new_string("status", "approved");

    // Status is pending → reason required → absent → fail.
    let mut data = HashMap::new();
    data.insert("status".to_string(), json!("pending"));
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "reason",
        vec![Box::new(rule) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_err());

    // Status is approved → reason optional → absent → pass.
    let rule2 = RequiredUnlessRule::new_string("status", "approved");
    let mut data2 = HashMap::new();
    data2.insert("status".to_string(), json!("approved"));
    let mut v2 = Validator::new(data2);
    v2.rules(HashMap::from([(
        "reason",
        vec![Box::new(rule2) as Box<dyn Rule>],
    )]));
    assert!(v2.validate().await.is_ok());
}

#[tokio::test]
async fn required_with_and_without() {
    // RequiredWith: "city" required when "address" is present.
    let rw = RequiredWithRule::new("address");
    let mut data = HashMap::new();
    data.insert("address".to_string(), json!("123 Main St"));
    // "city" is absent → fail.
    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "city",
        vec![Box::new(rw) as Box<dyn Rule>],
    )]));
    assert!(v.validate().await.is_err());

    // RequiredWithout: "email" required when "phone" is absent.
    let rwo = RequiredWithoutRule::new("phone");
    let mut data2: HashMap<String, Value> = HashMap::new(); // no phone
    // email also absent → fail.
    let mut v2 = Validator::new(data2.clone());
    v2.rules(HashMap::from([(
        "email",
        vec![Box::new(rwo) as Box<dyn Rule>],
    )]));
    assert!(v2.validate().await.is_err());

    // Phone present → email optional → pass even without email.
    let rwo2 = RequiredWithoutRule::new("phone");
    data2.insert("phone".to_string(), json!("555-1234"));
    let mut v3 = Validator::new(data2);
    v3.rules(HashMap::from([(
        "email",
        vec![Box::new(rwo2) as Box<dyn Rule>],
    )]));
    assert!(v3.validate().await.is_ok());
}

// ============================================================================
// Full Validator pipeline — multiple fields, all errors collected
// ============================================================================

#[tokio::test]
async fn validator_collects_errors_for_all_failing_fields() {
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("not-an-email"));
    data.insert("age".to_string(), json!("not-a-number"));

    let mut v = Validator::new(data);
    v.rules(HashMap::from([
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
                Box::new(IntegerRule),
            ],
        ),
    ]));

    let errs = v.validate().await.unwrap_err();
    assert!(errs.get("email").is_some(), "expected error on 'email'");
    assert!(errs.get("age").is_some(), "expected error on 'age'");
}

#[tokio::test]
async fn validator_ok_when_all_fields_pass() {
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("alice@example.com"));
    data.insert("age".to_string(), json!(25));

    let mut v = Validator::new(data);
    v.rules(HashMap::from([
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
                Box::new(IntegerRule),
                Box::new(MinRule::new(18)),
                Box::new(MaxRule::new(120)),
            ],
        ),
    ]));

    let validated = v.validate().await.expect("valid data must pass");
    assert_eq!(
        validated.get_string("email").as_deref(),
        Some("alice@example.com")
    );
    assert_eq!(validated.get_i64("age"), Some(25));
}

// ============================================================================
// Error message content — rule code appears in FieldError.code
// ============================================================================

#[tokio::test]
async fn error_carries_correct_rule_code() {
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("bad-email"));

    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "email",
        vec![Box::new(EmailRule) as Box<dyn Rule>],
    )]));

    let errs = v.validate().await.unwrap_err();
    let field_errors = errs.get("email").unwrap();
    assert_eq!(
        field_errors[0].code, "email",
        "code must be 'email', got: {}",
        field_errors[0].code
    );
}

// ============================================================================
// Custom message override in Validator::messages()
// ============================================================================

#[tokio::test]
async fn custom_message_overrides_default() {
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("not-valid"));

    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "email",
        vec![Box::new(EmailRule) as Box<dyn Rule>],
    )]));
    v.messages(HashMap::from([(
        "email.email",
        "Please provide a working email address.",
    )]));

    let errs = v.validate().await.unwrap_err();
    let field_errors = errs.get("email").unwrap();
    assert_eq!(
        field_errors[0].message,
        "Please provide a working email address.",
        "custom message must replace the default"
    );
}

// ============================================================================
// Short-circuit: required failure must block subsequent rules
// ============================================================================

#[tokio::test]
async fn required_failure_short_circuits_remaining_rules() {
    // If RequiredRule fails, the email rule should NOT produce an additional error
    // for the same field (since the value is absent, not "wrong format").
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!(null));

    let mut v = Validator::new(data);
    v.rules(HashMap::from([(
        "email",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(EmailRule),
        ],
    )]));

    let errs = v.validate().await.unwrap_err();
    let field_errors = errs.get("email").unwrap();
    // Only ONE error: required; the email format error must NOT appear.
    assert_eq!(
        field_errors.len(),
        1,
        "required must short-circuit: got {:?}",
        field_errors
    );
    assert_eq!(field_errors[0].code, "required");
}
