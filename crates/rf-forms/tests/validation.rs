//! Behavioural unit tests for the Laravel-style validation rules.
//!
//! These exercise the public `Validator` + rule-factory API end to end. A key
//! framework contract verified here: **only `required` enforces presence** —
//! every other rule treats a missing/empty value as "nothing to validate" and
//! passes, exactly like Laravel. The tests pin that contract down so a
//! regression (e.g. `email()` suddenly rejecting empty input) is caught.

use rf_forms::validation::*;
use std::collections::HashMap;

/// Build `ValidationData` from key/value pairs. A field that is *not* listed is
/// absent (`None`), which is distinct from being present but empty (`""`).
fn data(pairs: &[(&str, &str)]) -> ValidationData {
    let mut m = HashMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.to_string());
    }
    ValidationData::from(m)
}

/// Run `rules` against `field` over `pairs` and report whether validation passed.
fn passes(pairs: &[(&str, &str)], field: &str, rules: Vec<Box<dyn ValidationRuleTrait>>) -> bool {
    Validator::new(data(pairs)).rule(field, rules).check().is_ok()
}

#[test]
fn required_enforces_presence_and_non_whitespace() {
    assert!(passes(&[("name", "Ada")], "name", vec![required()]));
    // Absent field.
    assert!(!passes(&[], "name", vec![required()]));
    // Present but empty / whitespace-only.
    assert!(!passes(&[("name", "")], "name", vec![required()]));
    assert!(!passes(&[("name", "   ")], "name", vec![required()]));
}

#[test]
fn optional_rules_pass_on_absent_value() {
    // The hallmark Laravel behaviour: without `required`, a missing field is OK
    // even under strict rules.
    assert!(passes(&[], "email", vec![email()]));
    assert!(passes(&[], "age", vec![numeric(), min(18.0)]));
    assert!(passes(&[], "slug", vec![alpha(), min_length(5)]));
}

#[test]
fn email_accepts_valid_and_rejects_malformed() {
    assert!(passes(&[("e", "user@example.com")], "e", vec![email()]));
    assert!(passes(&[("e", "a@b.co")], "e", vec![email()]));
    assert!(!passes(&[("e", "not-an-email")], "e", vec![email()]));
    assert!(!passes(&[("e", "missing@dot")], "e", vec![email()]));
    assert!(!passes(&[("e", "with space@example.com")], "e", vec![email()]));
}

#[test]
fn numeric_and_integer_distinguish_floats_from_ints() {
    assert!(passes(&[("n", "3.14")], "n", vec![numeric()]));
    assert!(passes(&[("n", "-42")], "n", vec![numeric()]));
    assert!(!passes(&[("n", "abc")], "n", vec![numeric()]));

    assert!(passes(&[("n", "10")], "n", vec![integer()]));
    // A float is numeric but not an integer.
    assert!(passes(&[("n", "3.14")], "n", vec![numeric()]));
    assert!(!passes(&[("n", "3.14")], "n", vec![integer()]));
}

#[test]
fn numeric_min_max_compare_values_not_lengths() {
    assert!(passes(&[("age", "18")], "age", vec![min(18.0)]));
    assert!(!passes(&[("age", "17")], "age", vec![min(18.0)]));
    assert!(passes(&[("age", "100")], "age", vec![max(100.0)]));
    assert!(!passes(&[("age", "101")], "age", vec![max(100.0)]));
    // A value inside a [min, max] band.
    assert!(passes(&[("age", "30")], "age", vec![min(18.0), max(65.0)]));
}

#[test]
fn length_rules_count_characters() {
    assert!(passes(&[("p", "secret12")], "p", vec![min_length(8)]));
    assert!(!passes(&[("p", "short")], "p", vec![min_length(8)]));
    assert!(passes(&[("p", "abc")], "p", vec![max_length(5)]));
    assert!(!passes(&[("p", "abcdef")], "p", vec![max_length(5)]));

    // `between` checks length inclusively on both ends.
    assert!(passes(&[("u", "abcd")], "u", vec![between(3, 10)]));
    assert!(passes(&[("u", "abc")], "u", vec![between(3, 10)]));
    assert!(!passes(&[("u", "ab")], "u", vec![between(3, 10)]));
    // Present-but-empty fails `between` (length 0) even though it's not `required`.
    assert!(!passes(&[("u", "")], "u", vec![between(3, 10)]));
}

#[test]
fn confirmed_matches_the_confirmation_field() {
    assert!(passes(
        &[("password", "hunter2"), ("password_confirmation", "hunter2")],
        "password",
        vec![confirmed()],
    ));
    assert!(!passes(
        &[("password", "hunter2"), ("password_confirmation", "nope")],
        "password",
        vec![confirmed()],
    ));
    // Missing confirmation field does not match a present value.
    assert!(!passes(&[("password", "hunter2")], "password", vec![confirmed()]));
}

#[test]
fn same_and_different_compare_two_fields() {
    let both = &[("a", "x"), ("b", "x")];
    assert!(passes(both, "a", vec![same("b")]));
    assert!(!passes(both, "a", vec![different("b")]));

    let differ = &[("a", "x"), ("b", "y")];
    assert!(!passes(differ, "a", vec![same("b")]));
    assert!(passes(differ, "a", vec![different("b")]));
}

#[test]
fn in_list_and_not_in_check_membership() {
    let roles = || vec!["admin".to_string(), "editor".to_string()];
    assert!(passes(&[("role", "admin")], "role", vec![in_list(roles())]));
    assert!(!passes(&[("role", "guest")], "role", vec![in_list(roles())]));

    assert!(passes(&[("role", "guest")], "role", vec![not_in(roles())]));
    assert!(!passes(&[("role", "admin")], "role", vec![not_in(roles())]));
}

#[test]
fn boolean_accepts_truthy_and_falsy_spellings() {
    for v in ["true", "false", "1", "0", "yes", "no", "on", "off", "YES", "Off"] {
        assert!(passes(&[("flag", v)], "flag", vec![boolean()]), "{v} should be boolean");
    }
    assert!(!passes(&[("flag", "maybe")], "flag", vec![boolean()]));
}

#[test]
fn alpha_and_alpha_numeric() {
    assert!(passes(&[("s", "abcDEF")], "s", vec![alpha()]));
    assert!(!passes(&[("s", "abc1")], "s", vec![alpha()]));

    assert!(passes(&[("s", "abc123")], "s", vec![alpha_numeric()]));
    assert!(!passes(&[("s", "abc-123")], "s", vec![alpha_numeric()]));
}

#[test]
fn url_ip_and_uuid_formats() {
    assert!(passes(&[("u", "https://example.com/path")], "u", vec![url()]));
    assert!(!passes(&[("u", "ftp://example.com")], "u", vec![url()]));

    assert!(passes(&[("ip", "127.0.0.1")], "ip", vec![ip()]));
    assert!(passes(&[("ip", "::1")], "ip", vec![ip()]));
    assert!(!passes(&[("ip", "999.1.1.1")], "ip", vec![ip()]));

    assert!(passes(
        &[("id", "550e8400-e29b-41d4-a716-446655440000")],
        "id",
        vec![uuid()],
    ));
    assert!(!passes(&[("id", "not-a-uuid")], "id", vec![uuid()]));
}

#[test]
fn first_failing_rule_short_circuits_per_field() {
    // `required` fails first, so the field has exactly one error even though
    // `email` would also fail.
    let errs = Validator::new(data(&[]))
        .rule("email", vec![required(), email()])
        .check()
        .unwrap_err();
    assert_eq!(errs.get("email").map(|v| v.len()), Some(1));
}

#[test]
fn custom_message_overrides_the_default() {
    let errs = Validator::new(data(&[]))
        .rule("email", vec![required()])
        .message("email", "We need your email.")
        .check()
        .unwrap_err();
    assert_eq!(errs.get("email").unwrap(), &vec!["We need your email.".to_string()]);
}

#[test]
fn errors_aggregate_across_multiple_fields() {
    let errs = Validator::new(data(&[("age", "abc")]))
        .rule("name", vec![required()])
        .rule("age", vec![numeric()])
        .check()
        .unwrap_err();
    assert!(errs.has_errors());
    assert!(errs.get("name").is_some());
    assert!(errs.get("age").is_some());
}

#[test]
fn validate_returns_the_data_on_success() {
    let result = Validator::new(data(&[("name", "Ada"), ("email", "ada@example.com")]))
        .rule("name", vec![required()])
        .rule("email", vec![required(), email()])
        .validate();
    let validated = result.expect("validation should pass");
    assert_eq!(validated.get("name"), Some("Ada"));
}
