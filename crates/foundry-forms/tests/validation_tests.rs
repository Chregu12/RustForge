//! Comprehensive validation tests

use foundry_forms::validation::*;
use std::collections::HashMap;

// Helper function to create validation data
fn data_with(fields: Vec<(&str, &str)>) -> ValidationData {
    let mut map = HashMap::new();
    for (key, value) in fields {
        map.insert(key.to_string(), value.to_string());
    }
    ValidationData::from(map)
}

// ============================================================================
// Required Rules Tests
// ============================================================================

#[test]
fn test_required_passes() {
    let data = data_with(vec![("name", "John")]);
    let result = Validator::new(data)
        .rule("name", vec![required()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_required_fails_empty() {
    let data = data_with(vec![("name", "")]);
    let result = Validator::new(data)
        .rule("name", vec![required()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_required_fails_whitespace() {
    let data = data_with(vec![("name", "   ")]);
    let result = Validator::new(data)
        .rule("name", vec![required()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_required_if_passes_when_condition_met() {
    let data = data_with(vec![("role", "admin"), ("permission", "write")]);
    let result = Validator::new(data)
        .rule("permission", vec![required_if("role", "admin")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_required_if_fails_when_condition_met() {
    let data = data_with(vec![("role", "admin"), ("permission", "")]);
    let result = Validator::new(data)
        .rule("permission", vec![required_if("role", "admin")])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_required_if_passes_when_condition_not_met() {
    let data = data_with(vec![("role", "user"), ("permission", "")]);
    let result = Validator::new(data)
        .rule("permission", vec![required_if("role", "admin")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_required_with_passes() {
    let data = data_with(vec![("email", "test@example.com"), ("password", "secret")]);
    let result = Validator::new(data)
        .rule("password", vec![required_with("email")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_required_with_fails() {
    let data = data_with(vec![("email", "test@example.com"), ("password", "")]);
    let result = Validator::new(data)
        .rule("password", vec![required_with("email")])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Email & URL Rules Tests
// ============================================================================

#[test]
fn test_email_valid() {
    let data = data_with(vec![("email", "user@example.com")]);
    let result = Validator::new(data)
        .rule("email", vec![email()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_email_invalid() {
    let data = data_with(vec![("email", "not-an-email")]);
    let result = Validator::new(data)
        .rule("email", vec![email()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_email_empty_allowed() {
    let data = data_with(vec![("email", "")]);
    let result = Validator::new(data)
        .rule("email", vec![email()])
        .validate();
    assert!(result.is_ok()); // Email rule doesn't enforce required
}

#[test]
fn test_url_valid() {
    let data = data_with(vec![("website", "https://example.com")]);
    let result = Validator::new(data)
        .rule("website", vec![url()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_url_invalid() {
    let data = data_with(vec![("website", "not-a-url")]);
    let result = Validator::new(data)
        .rule("website", vec![url()])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// IP & UUID Rules Tests
// ============================================================================

#[test]
fn test_ip_valid_ipv4() {
    let data = data_with(vec![("ip", "192.168.1.1")]);
    let result = Validator::new(data)
        .rule("ip", vec![ip()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_ip_valid_ipv6() {
    let data = data_with(vec![("ip", "2001:0db8:85a3:0000:0000:8a2e:0370:7334")]);
    let result = Validator::new(data)
        .rule("ip", vec![ip()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_ip_invalid() {
    let data = data_with(vec![("ip", "not-an-ip")]);
    let result = Validator::new(data)
        .rule("ip", vec![ip()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_uuid_valid() {
    let data = data_with(vec![("id", "550e8400-e29b-41d4-a716-446655440000")]);
    let result = Validator::new(data)
        .rule("id", vec![uuid()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_uuid_invalid() {
    let data = data_with(vec![("id", "not-a-uuid")]);
    let result = Validator::new(data)
        .rule("id", vec![uuid()])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Length Rules Tests
// ============================================================================

#[test]
fn test_min_length_passes() {
    let data = data_with(vec![("username", "john")]);
    let result = Validator::new(data)
        .rule("username", vec![min_length(3)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_min_length_fails() {
    let data = data_with(vec![("username", "jo")]);
    let result = Validator::new(data)
        .rule("username", vec![min_length(3)])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_max_length_passes() {
    let data = data_with(vec![("username", "john")]);
    let result = Validator::new(data)
        .rule("username", vec![max_length(10)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_max_length_fails() {
    let data = data_with(vec![("username", "verylongusername")]);
    let result = Validator::new(data)
        .rule("username", vec![max_length(10)])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_between_passes() {
    let data = data_with(vec![("username", "john")]);
    let result = Validator::new(data)
        .rule("username", vec![between(3, 10)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_between_fails_too_short() {
    let data = data_with(vec![("username", "jo")]);
    let result = Validator::new(data)
        .rule("username", vec![between(3, 10)])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_between_fails_too_long() {
    let data = data_with(vec![("username", "verylongusername")]);
    let result = Validator::new(data)
        .rule("username", vec![between(3, 10)])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_size_passes() {
    let data = data_with(vec![("code", "ABCD")]);
    let result = Validator::new(data)
        .rule("code", vec![size(4)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_size_fails() {
    let data = data_with(vec![("code", "ABC")]);
    let result = Validator::new(data)
        .rule("code", vec![size(4)])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Numeric Rules Tests
// ============================================================================

#[test]
fn test_numeric_valid_integer() {
    let data = data_with(vec![("age", "25")]);
    let result = Validator::new(data)
        .rule("age", vec![numeric()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_numeric_valid_float() {
    let data = data_with(vec![("price", "19.99")]);
    let result = Validator::new(data)
        .rule("price", vec![numeric()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_numeric_invalid() {
    let data = data_with(vec![("age", "not-a-number")]);
    let result = Validator::new(data)
        .rule("age", vec![numeric()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_integer_valid() {
    let data = data_with(vec![("count", "42")]);
    let result = Validator::new(data)
        .rule("count", vec![integer()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_integer_invalid_float() {
    let data = data_with(vec![("count", "42.5")]);
    let result = Validator::new(data)
        .rule("count", vec![integer()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_min_numeric_passes() {
    let data = data_with(vec![("age", "25")]);
    let result = Validator::new(data)
        .rule("age", vec![min(18.0)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_min_numeric_fails() {
    let data = data_with(vec![("age", "15")]);
    let result = Validator::new(data)
        .rule("age", vec![min(18.0)])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_max_numeric_passes() {
    let data = data_with(vec![("age", "25")]);
    let result = Validator::new(data)
        .rule("age", vec![max(100.0)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_max_numeric_fails() {
    let data = data_with(vec![("age", "150")]);
    let result = Validator::new(data)
        .rule("age", vec![max(100.0)])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Boolean & Type Rules Tests
// ============================================================================

#[test]
fn test_boolean_valid_true() {
    let data = data_with(vec![("accept", "true")]);
    let result = Validator::new(data)
        .rule("accept", vec![boolean()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_boolean_valid_1() {
    let data = data_with(vec![("accept", "1")]);
    let result = Validator::new(data)
        .rule("accept", vec![boolean()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_boolean_valid_yes() {
    let data = data_with(vec![("accept", "yes")]);
    let result = Validator::new(data)
        .rule("accept", vec![boolean()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_boolean_invalid() {
    let data = data_with(vec![("accept", "maybe")]);
    let result = Validator::new(data)
        .rule("accept", vec![boolean()])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Comparison Rules Tests
// ============================================================================

#[test]
fn test_confirmed_passes() {
    let data = data_with(vec![
        ("password", "secret123"),
        ("password_confirmation", "secret123"),
    ]);
    let result = Validator::new(data)
        .rule("password", vec![confirmed()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_confirmed_fails() {
    let data = data_with(vec![
        ("password", "secret123"),
        ("password_confirmation", "different"),
    ]);
    let result = Validator::new(data)
        .rule("password", vec![confirmed()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_same_passes() {
    let data = data_with(vec![("email", "test@example.com"), ("email_confirm", "test@example.com")]);
    let result = Validator::new(data)
        .rule("email_confirm", vec![same("email")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_same_fails() {
    let data = data_with(vec![("email", "test@example.com"), ("email_confirm", "different@example.com")]);
    let result = Validator::new(data)
        .rule("email_confirm", vec![same("email")])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_different_passes() {
    let data = data_with(vec![("new_password", "newpass"), ("old_password", "oldpass")]);
    let result = Validator::new(data)
        .rule("new_password", vec![different("old_password")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_different_fails() {
    let data = data_with(vec![("new_password", "samepass"), ("old_password", "samepass")]);
    let result = Validator::new(data)
        .rule("new_password", vec![different("old_password")])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// In/NotIn Rules Tests
// ============================================================================

#[test]
fn test_in_passes() {
    let data = data_with(vec![("status", "active")]);
    let result = Validator::new(data)
        .rule("status", vec![in_list(vec!["active".to_string(), "inactive".to_string(), "pending".to_string()])])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_in_fails() {
    let data = data_with(vec![("status", "invalid")]);
    let result = Validator::new(data)
        .rule("status", vec![in_list(vec!["active".to_string(), "inactive".to_string()])])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_not_in_passes() {
    let data = data_with(vec![("username", "john")]);
    let result = Validator::new(data)
        .rule("username", vec![not_in(vec!["admin".to_string(), "root".to_string()])])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_not_in_fails() {
    let data = data_with(vec![("username", "admin")]);
    let result = Validator::new(data)
        .rule("username", vec![not_in(vec!["admin".to_string(), "root".to_string()])])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Pattern Rules Tests
// ============================================================================

#[test]
fn test_regex_passes() {
    let data = data_with(vec![("username", "john_doe123")]);
    let result = Validator::new(data)
        .rule("username", vec![regex(r"^[a-z_0-9]+$")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_regex_fails() {
    let data = data_with(vec![("username", "John Doe!")]);
    let result = Validator::new(data)
        .rule("username", vec![regex(r"^[a-z_0-9]+$")])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_alpha_passes() {
    let data = data_with(vec![("name", "JohnDoe")]);
    let result = Validator::new(data)
        .rule("name", vec![alpha()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_alpha_fails() {
    let data = data_with(vec![("name", "John123")]);
    let result = Validator::new(data)
        .rule("name", vec![alpha()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_alpha_numeric_passes() {
    let data = data_with(vec![("code", "ABC123")]);
    let result = Validator::new(data)
        .rule("code", vec![alpha_numeric()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_alpha_numeric_fails() {
    let data = data_with(vec![("code", "ABC-123")]);
    let result = Validator::new(data)
        .rule("code", vec![alpha_numeric()])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Date Rules Tests
// ============================================================================

#[test]
fn test_date_valid() {
    let data = data_with(vec![("birthdate", "1990-05-15")]);
    let result = Validator::new(data)
        .rule("birthdate", vec![date()])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_date_invalid_format() {
    let data = data_with(vec![("birthdate", "15/05/1990")]);
    let result = Validator::new(data)
        .rule("birthdate", vec![date()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_date_invalid_values() {
    let data = data_with(vec![("birthdate", "1990-13-45")]);
    let result = Validator::new(data)
        .rule("birthdate", vec![date()])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_before_passes() {
    let data = data_with(vec![("start_date", "2020-01-01")]);
    let result = Validator::new(data)
        .rule("start_date", vec![before("2025-01-01")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_before_fails() {
    let data = data_with(vec![("start_date", "2026-01-01")]);
    let result = Validator::new(data)
        .rule("start_date", vec![before("2025-01-01")])
        .validate();
    assert!(result.is_err());
}

#[test]
fn test_after_passes() {
    let data = data_with(vec![("end_date", "2025-01-01")]);
    let result = Validator::new(data)
        .rule("end_date", vec![after("2020-01-01")])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_after_fails() {
    let data = data_with(vec![("end_date", "2019-01-01")]);
    let result = Validator::new(data)
        .rule("end_date", vec![after("2020-01-01")])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// Multiple Rules & Error Handling Tests
// ============================================================================

#[test]
fn test_multiple_rules_all_pass() {
    let data = data_with(vec![("email", "user@example.com")]);
    let result = Validator::new(data)
        .rule("email", vec![required(), email(), min_length(5)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_multiple_rules_first_fails() {
    let data = data_with(vec![("email", "")]);
    let result = Validator::new(data)
        .rule("email", vec![required(), email()])
        .validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.get("email").is_some());
}

#[test]
fn test_multiple_fields() {
    let data = data_with(vec![
        ("name", "John"),
        ("email", "john@example.com"),
        ("age", "25"),
    ]);
    let result = Validator::new(data)
        .rule("name", vec![required(), min_length(3)])
        .rule("email", vec![required(), email()])
        .rule("age", vec![required(), numeric(), min(18.0)])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_multiple_fields_with_errors() {
    let data = data_with(vec![
        ("name", "Jo"),  // Too short
        ("email", "invalid"),  // Invalid email
        ("age", "15"),  // Below minimum
    ]);
    let result = Validator::new(data)
        .rule("name", vec![required(), min_length(3)])
        .rule("email", vec![required(), email()])
        .rule("age", vec![required(), numeric(), min(18.0)])
        .validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.errors.len(), 3);
}

#[test]
fn test_custom_error_messages() {
    let data = data_with(vec![("email", "")]);
    let result = Validator::new(data)
        .rule("email", vec![required()])
        .message("email", "Please provide your email address")
        .validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.first("email").unwrap(), "Please provide your email address");
}

// ============================================================================
// Custom Rules Tests
// ============================================================================

#[test]
fn test_custom_rule_passes() {
    let data = data_with(vec![("username", "validuser")]);
    let custom_validator = custom("custom", |_field, value, _data| {
        if let Some(v) = value {
            if v.contains("admin") {
                return Err("Username cannot contain 'admin'".to_string());
            }
        }
        Ok(())
    });
    let result = Validator::new(data)
        .rule("username", vec![custom_validator])
        .validate();
    assert!(result.is_ok());
}

#[test]
fn test_custom_rule_fails() {
    let data = data_with(vec![("username", "adminuser")]);
    let custom_validator = custom("custom", |_field, value, _data| {
        if let Some(v) = value {
            if v.contains("admin") {
                return Err("Username cannot contain 'admin'".to_string());
            }
        }
        Ok(())
    });
    let result = Validator::new(data)
        .rule("username", vec![custom_validator])
        .validate();
    assert!(result.is_err());
}

// ============================================================================
// ValidationErrors Tests
// ============================================================================

#[test]
fn test_validation_errors_first() {
    let mut errors = ValidationErrors::new();
    errors.add("email", "Email is required");
    errors.add("email", "Email is invalid");

    assert_eq!(errors.first("email").unwrap(), "Email is required");
}

#[test]
fn test_validation_errors_all() {
    let mut errors = ValidationErrors::new();
    errors.add("email", "Email is required");
    errors.add("password", "Password is too short");

    let all = errors.all();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_validation_errors_has_errors() {
    let mut errors = ValidationErrors::new();
    assert!(!errors.has_errors());

    errors.add("email", "Email is required");
    assert!(errors.has_errors());
}
