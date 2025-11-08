//! FormRequest tests

use foundry_forms::form_request::*;
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
// Test FormRequest Implementation
// ============================================================================

struct CreateUserRequest;

impl FormRequest for CreateUserRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![required(), min_length(3)])
            .rule("email", vec![required(), email()])
            .rule("password", vec![required(), min_length(8)])
    }

    fn authorize(&self) -> bool {
        true
    }
}

struct UnauthorizedRequest;

impl FormRequest for UnauthorizedRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![required()])
    }

    fn authorize(&self) -> bool {
        false
    }
}

struct CustomMessagesRequest;

impl FormRequest for CustomMessagesRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("email", vec![required(), email()])
    }

    fn messages(&self) -> HashMap<String, String> {
        let mut messages = HashMap::new();
        messages.insert("email".to_string(), "We need your email address!".to_string());
        messages
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_form_request_passes() {
    let request = CreateUserRequest;
    let data = data_with(vec![
        ("name", "John Doe"),
        ("email", "john@example.com"),
        ("password", "secret123"),
    ]);

    let result = request.validate(data);
    assert!(result.is_ok());
}

#[test]
fn test_form_request_fails_validation() {
    let request = CreateUserRequest;
    let data = data_with(vec![
        ("name", "Jo"),  // Too short
        ("email", "invalid"),  // Invalid email
        ("password", "short"),  // Too short
    ]);

    let result = request.validate(data);
    assert!(result.is_err());

    match result.unwrap_err() {
        FormRequestError::Validation(errors) => {
            assert!(errors.has_errors());
            assert!(errors.get("name").is_some());
            assert!(errors.get("email").is_some());
            assert!(errors.get("password").is_some());
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_form_request_unauthorized() {
    let request = UnauthorizedRequest;
    let data = data_with(vec![("name", "John")]);

    let result = request.validate(data);
    assert!(result.is_err());

    match result.unwrap_err() {
        FormRequestError::Unauthorized => {}
        _ => panic!("Expected unauthorized error"),
    }
}

#[test]
fn test_form_request_custom_messages() {
    let request = CustomMessagesRequest;
    let data = data_with(vec![("email", "")]);

    let result = request.validate(data);
    assert!(result.is_err());

    match result.unwrap_err() {
        FormRequestError::Validation(errors) => {
            let message = errors.first("email").unwrap();
            assert_eq!(message, "We need your email address!");
        }
        _ => panic!("Expected validation error"),
    }
}

#[test]
fn test_form_request_validator_builder() {
    let validator = FormRequestValidator::new()
        .rule("name", vec![required()])
        .rule("email", vec![required(), email()]);

    assert_eq!(validator.rules.len(), 2);
    assert!(validator.rules.contains_key("name"));
    assert!(validator.rules.contains_key("email"));
}

#[test]
fn test_form_request_with_confirmed() {
    struct PasswordResetRequest;

    impl FormRequest for PasswordResetRequest {
        fn rules(&self) -> FormRequestValidator {
            FormRequestValidator::new()
                .rule("password", vec![required(), min_length(8), confirmed()])
        }
    }

    let request = PasswordResetRequest;
    let data = data_with(vec![
        ("password", "secret123"),
        ("password_confirmation", "secret123"),
    ]);

    let result = request.validate(data);
    assert!(result.is_ok());
}

#[test]
fn test_form_request_with_confirmed_fails() {
    struct PasswordResetRequest;

    impl FormRequest for PasswordResetRequest {
        fn rules(&self) -> FormRequestValidator {
            FormRequestValidator::new()
                .rule("password", vec![required(), confirmed()])
        }
    }

    let request = PasswordResetRequest;
    let data = data_with(vec![
        ("password", "secret123"),
        ("password_confirmation", "different"),
    ]);

    let result = request.validate(data);
    assert!(result.is_err());
}

#[test]
fn test_form_request_with_conditional_validation() {
    struct ConditionalRequest;

    impl FormRequest for ConditionalRequest {
        fn rules(&self) -> FormRequestValidator {
            FormRequestValidator::new()
                .rule("role", vec![required()])
                .rule("admin_code", vec![required_if("role", "admin")])
        }
    }

    let request = ConditionalRequest;

    // Test with admin role and code
    let data = data_with(vec![
        ("role", "admin"),
        ("admin_code", "SECRET"),
    ]);
    assert!(request.validate(data).is_ok());

    // Test with admin role but no code
    let data = data_with(vec![
        ("role", "admin"),
        ("admin_code", ""),
    ]);
    assert!(request.validate(data).is_err());

    // Test with user role and no code
    let data = data_with(vec![
        ("role", "user"),
        ("admin_code", ""),
    ]);
    assert!(request.validate(data).is_ok());
}
