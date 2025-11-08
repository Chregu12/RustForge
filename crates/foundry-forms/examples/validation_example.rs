//! Comprehensive validation examples
//!
//! Run with: cargo run --example validation_example

use foundry_forms::form_request::*;
use foundry_forms::validation::*;
use std::collections::HashMap;

fn example_basic_validation() {
    println!("\n=== Basic Validation Example ===\n");

    let mut data = HashMap::new();
    data.insert("email".to_string(), "user@example.com".to_string());
    data.insert("age".to_string(), "25".to_string());

    let result = Validator::new(ValidationData::from(data))
        .rule("email", vec![required(), email()])
        .rule("age", vec![required(), numeric(), min(18.0)])
        .validate();

    match result {
        Ok(_) => println!("✓ Validation passed!"),
        Err(errors) => {
            println!("✗ Validation failed:");
            for (field, messages) in errors.errors {
                println!("  {}: {:?}", field, messages);
            }
        }
    }
}

fn example_password_validation() {
    println!("\n=== Password Validation Example ===\n");

    let mut data = HashMap::new();
    data.insert("password".to_string(), "Secret123".to_string());
    data.insert("password_confirmation".to_string(), "Secret123".to_string());

    let result = Validator::new(ValidationData::from(data))
        .rule(
            "password",
            vec![
                required(),
                min_length(8),
                regex(r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).*$"),
                confirmed(),
            ],
        )
        .message(
            "password",
            "Password must be at least 8 characters with uppercase, lowercase, and numbers",
        )
        .validate();

    match result {
        Ok(_) => println!("✓ Password validation passed!"),
        Err(errors) => {
            println!("✗ Password validation failed:");
            for (field, messages) in errors.errors {
                println!("  {}: {:?}", field, messages);
            }
        }
    }
}

fn example_conditional_validation() {
    println!("\n=== Conditional Validation Example ===\n");

    let mut data = HashMap::new();
    data.insert("role".to_string(), "admin".to_string());
    data.insert("admin_code".to_string(), "SECRET123".to_string());

    let result = Validator::new(ValidationData::from(data))
        .rule("role", vec![required()])
        .rule("admin_code", vec![required_if("role", "admin"), size(9)])
        .validate();

    match result {
        Ok(_) => println!("✓ Conditional validation passed!"),
        Err(errors) => {
            println!("✗ Conditional validation failed:");
            for (field, messages) in errors.errors {
                println!("  {}: {:?}", field, messages);
            }
        }
    }
}

fn example_custom_validation() {
    println!("\n=== Custom Validation Example ===\n");

    let mut data = HashMap::new();
    data.insert("username".to_string(), "validuser".to_string());

    let no_admin_rule = custom("no_admin", |_field, value, _data| {
        if let Some(v) = value {
            if v.to_lowercase().contains("admin") {
                return Err("Username cannot contain 'admin'".to_string());
            }
        }
        Ok(())
    });

    let result = Validator::new(ValidationData::from(data))
        .rule("username", vec![required(), no_admin_rule])
        .validate();

    match result {
        Ok(_) => println!("✓ Custom validation passed!"),
        Err(errors) => {
            println!("✗ Custom validation failed:");
            for (field, messages) in errors.errors {
                println!("  {}: {:?}", field, messages);
            }
        }
    }
}

// FormRequest example
struct RegisterUserRequest;

impl FormRequest for RegisterUserRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![required(), min_length(3), max_length(50), alpha()])
            .rule("email", vec![required(), email(), max_length(255)])
            .rule(
                "password",
                vec![
                    required(),
                    min_length(8),
                    regex(r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).*$"),
                    confirmed(),
                ],
            )
            .rule("age", vec![required(), integer(), min(18.0), max(120.0)])
    }

    fn authorize(&self) -> bool {
        true // Public registration endpoint
    }

    fn messages(&self) -> HashMap<String, String> {
        let mut messages = HashMap::new();
        messages.insert(
            "password".to_string(),
            "Password must be at least 8 characters with uppercase, lowercase, and numbers"
                .to_string(),
        );
        messages.insert(
            "age".to_string(),
            "You must be at least 18 years old to register".to_string(),
        );
        messages
    }
}

fn example_form_request() {
    println!("\n=== FormRequest Pattern Example ===\n");

    let mut data = HashMap::new();
    data.insert("name".to_string(), "John".to_string());
    data.insert("email".to_string(), "john@example.com".to_string());
    data.insert("password".to_string(), "Secret123".to_string());
    data.insert("password_confirmation".to_string(), "Secret123".to_string());
    data.insert("age".to_string(), "25".to_string());

    let request = RegisterUserRequest;
    let result = request.validate(ValidationData::from(data));

    match result {
        Ok(_) => println!("✓ User registration validation passed!"),
        Err(FormRequestError::Validation(errors)) => {
            println!("✗ Validation failed:");
            for (field, messages) in errors.errors {
                println!("  {}: {:?}", field, messages);
            }
        }
        Err(FormRequestError::Unauthorized) => {
            println!("✗ Unauthorized");
        }
    }
}

fn example_multiple_errors() {
    println!("\n=== Multiple Validation Errors Example ===\n");

    let mut data = HashMap::new();
    data.insert("name".to_string(), "Jo".to_string()); // Too short
    data.insert("email".to_string(), "invalid-email".to_string()); // Invalid format
    data.insert("age".to_string(), "15".to_string()); // Below minimum

    let result = Validator::new(ValidationData::from(data))
        .rule("name", vec![required(), min_length(3)])
        .rule("email", vec![required(), email()])
        .rule("age", vec![required(), numeric(), min(18.0)])
        .validate();

    match result {
        Ok(_) => println!("✓ Validation passed!"),
        Err(errors) => {
            println!("✗ Multiple validation errors:");
            for (field, messages) in &errors.errors {
                println!("  {}: {:?}", field, messages);
            }
            println!("\nFirst error per field:");
            for (field, _) in &errors.errors {
                if let Some(first) = errors.first(field) {
                    println!("  {}: {}", field, first);
                }
            }
        }
    }
}

fn example_all_validation_rules() {
    println!("\n=== All Validation Rules Showcase ===\n");

    let mut data = HashMap::new();

    // Required rules
    data.insert("required_field".to_string(), "value".to_string());
    data.insert("role".to_string(), "admin".to_string());
    data.insert("admin_code".to_string(), "CODE123".to_string());
    data.insert("email".to_string(), "user@example.com".to_string());
    data.insert("confirm_email".to_string(), "user@example.com".to_string());

    // String rules
    data.insert("website".to_string(), "https://example.com".to_string());
    data.insert("name".to_string(), "JohnDoe".to_string());
    data.insert("code".to_string(), "ABC123".to_string());
    data.insert("pattern_field".to_string(), "valid_123".to_string());

    // Length rules
    data.insert("username".to_string(), "john_doe".to_string());
    data.insert("exact_code".to_string(), "123456".to_string());

    // Numeric rules
    data.insert("price".to_string(), "19.99".to_string());
    data.insert("count".to_string(), "42".to_string());
    data.insert("age".to_string(), "25".to_string());

    // Format rules
    data.insert("ip".to_string(), "192.168.1.1".to_string());
    data.insert("uuid".to_string(), "550e8400-e29b-41d4-a716-446655440000".to_string());

    // Comparison rules
    data.insert("password".to_string(), "secret".to_string());
    data.insert("password_confirmation".to_string(), "secret".to_string());
    data.insert("new_pass".to_string(), "newsecret".to_string());
    data.insert("old_pass".to_string(), "oldsecret".to_string());

    // List rules
    data.insert("status".to_string(), "active".to_string());
    data.insert("forbidden_name".to_string(), "user".to_string());

    // Type rules
    data.insert("accept".to_string(), "true".to_string());

    // Date rules
    data.insert("birthdate".to_string(), "1990-05-15".to_string());
    data.insert("start_date".to_string(), "2020-01-01".to_string());
    data.insert("end_date".to_string(), "2025-12-31".to_string());

    let result = Validator::new(ValidationData::from(data))
        // Required rules
        .rule("required_field", vec![required()])
        .rule("admin_code", vec![required_if("role", "admin")])
        .rule("confirm_email", vec![required_with("email")])

        // String rules
        .rule("email", vec![email()])
        .rule("website", vec![url()])
        .rule("name", vec![alpha()])
        .rule("code", vec![alpha_numeric()])
        .rule("pattern_field", vec![regex(r"^[a-z_0-9]+$")])

        // Length rules
        .rule("username", vec![min_length(3), max_length(20)])
        .rule("exact_code", vec![size(6)])

        // Numeric rules
        .rule("price", vec![numeric()])
        .rule("count", vec![integer()])
        .rule("age", vec![numeric(), min(18.0), max(100.0)])

        // Format rules
        .rule("ip", vec![ip()])
        .rule("uuid", vec![uuid()])

        // Comparison rules
        .rule("password", vec![confirmed()])
        .rule("confirm_email", vec![same("email")])
        .rule("new_pass", vec![different("old_pass")])

        // List rules
        .rule("status", vec![in_list(vec!["active".to_string(), "inactive".to_string()])])
        .rule("forbidden_name", vec![not_in(vec!["admin".to_string(), "root".to_string()])])

        // Type rules
        .rule("accept", vec![boolean()])

        // Date rules
        .rule("birthdate", vec![date()])
        .rule("start_date", vec![date(), after("2019-12-31")])
        .rule("end_date", vec![date(), before("2026-01-01")])
        .validate();

    match result {
        Ok(_) => println!("✓ All validation rules passed!"),
        Err(errors) => {
            println!("✗ Some validations failed:");
            for (field, messages) in errors.errors {
                println!("  {}: {:?}", field, messages);
            }
        }
    }
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║     Foundry Forms Validation System - Examples            ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    example_basic_validation();
    example_password_validation();
    example_conditional_validation();
    example_custom_validation();
    example_form_request();
    example_multiple_errors();
    example_all_validation_rules();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║     All examples completed!                                ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
}
