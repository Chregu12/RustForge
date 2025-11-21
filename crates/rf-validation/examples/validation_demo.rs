//! Comprehensive validation system demo
//!
//! Run with: cargo run --example validation_demo

use rf_validation::prelude::*;
use rf_validation::rules::*;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    println!("🦀 RustForge Validation System Demo\n");

    // Example 1: Basic String Validation
    println!("📧 Example 1: Email Validation");
    demo_email_validation().await;

    println!("\n---\n");

    // Example 2: Numeric Validation
    println!("🔢 Example 2: Age Validation");
    demo_age_validation().await;

    println!("\n---\n");

    // Example 3: Conditional Validation
    println!("🔀 Example 3: Password Confirmation");
    demo_password_confirmation().await;

    println!("\n---\n");

    // Example 4: Array Validation
    println!("📋 Example 4: Array Validation");
    demo_array_validation().await;

    println!("\n---\n");

    // Example 5: Date Validation
    println!("📅 Example 5: Date Range Validation");
    demo_date_validation().await;

    println!("\n---\n");

    // Example 6: Complex Multi-Field Validation
    println!("🎯 Example 6: Complete User Registration");
    demo_user_registration().await;
}

async fn demo_email_validation() {
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("user@example.com"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "email",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(EmailRule),
        ],
    )]));

    match validator.validate().await {
        Ok(validated) => {
            println!("✅ Valid email: {}", validated.get_string("email").unwrap());
        }
        Err(errors) => {
            println!("❌ Validation failed:");
            print_errors(&errors);
        }
    }

    // Test with invalid email
    let mut data = HashMap::new();
    data.insert("email".to_string(), json!("invalid-email"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "email",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(EmailRule),
        ],
    )]));

    match validator.validate().await {
        Ok(_) => println!("✅ Validation passed"),
        Err(errors) => {
            println!("❌ Invalid email caught:");
            print_errors(&errors);
        }
    }
}

async fn demo_age_validation() {
    let mut data = HashMap::new();
    data.insert("age".to_string(), json!(25));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "age",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(IntegerRule),
            Box::new(MinRule::new(18)),
            Box::new(MaxRule::new(120)),
        ],
    )]));

    validator.messages(HashMap::from([
        ("age.min", "You must be at least 18 years old"),
        ("age.max", "Please enter a valid age"),
    ]));

    match validator.validate().await {
        Ok(validated) => {
            println!("✅ Valid age: {}", validated.get_i64("age").unwrap());
        }
        Err(errors) => {
            println!("❌ Validation failed:");
            print_errors(&errors);
        }
    }

    // Test with age too young
    let mut data = HashMap::new();
    data.insert("age".to_string(), json!(15));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "age",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(IntegerRule),
            Box::new(MinRule::new(18)),
        ],
    )]));
    validator.messages(HashMap::from([("age.min", "You must be at least 18 years old")]));

    match validator.validate().await {
        Ok(_) => println!("✅ Validation passed"),
        Err(errors) => {
            println!("❌ Underage user caught:");
            print_errors(&errors);
        }
    }
}

async fn demo_password_confirmation() {
    let mut data = HashMap::new();
    data.insert("password".to_string(), json!("secret123"));
    data.insert("password_confirmation".to_string(), json!("secret123"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([
        (
            "password",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(MinLengthRule::new(8)),
            ],
        ),
        (
            "password_confirmation",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(SameRule::new("password")),
            ],
        ),
    ]));

    match validator.validate().await {
        Ok(_) => println!("✅ Passwords match and meet requirements"),
        Err(errors) => {
            println!("❌ Validation failed:");
            print_errors(&errors);
        }
    }

    // Test with mismatched passwords
    let mut data = HashMap::new();
    data.insert("password".to_string(), json!("secret123"));
    data.insert("password_confirmation".to_string(), json!("different456"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([
        (
            "password",
            vec![Box::new(RequiredRule) as Box<dyn Rule>],
        ),
        (
            "password_confirmation",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(SameRule::new("password")),
            ],
        ),
    ]));

    match validator.validate().await {
        Ok(_) => println!("✅ Validation passed"),
        Err(errors) => {
            println!("❌ Password mismatch caught:");
            print_errors(&errors);
        }
    }
}

async fn demo_array_validation() {
    let mut data = HashMap::new();
    data.insert("tags".to_string(), json!(["rust", "web", "validation"]));
    data.insert("status".to_string(), json!("active"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([
        (
            "tags",
            vec![
                Box::new(ArrayRule) as Box<dyn Rule>,
                Box::new(MinArraySizeRule::new(1)),
                Box::new(MaxArraySizeRule::new(10)),
                Box::new(DistinctRule),
            ],
        ),
        (
            "status",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(InRule::from_strings(vec!["active", "inactive", "pending"])),
            ],
        ),
    ]));

    match validator.validate().await {
        Ok(_) => println!("✅ Array validation passed"),
        Err(errors) => {
            println!("❌ Validation failed:");
            print_errors(&errors);
        }
    }

    // Test with invalid status
    let mut data = HashMap::new();
    data.insert("status".to_string(), json!("deleted"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([(
        "status",
        vec![
            Box::new(RequiredRule) as Box<dyn Rule>,
            Box::new(InRule::from_strings(vec!["active", "inactive", "pending"])),
        ],
    )]));

    match validator.validate().await {
        Ok(_) => println!("✅ Validation passed"),
        Err(errors) => {
            println!("❌ Invalid status caught:");
            print_errors(&errors);
        }
    }
}

async fn demo_date_validation() {
    let mut data = HashMap::new();
    data.insert("birth_date".to_string(), json!("1990-05-15"));
    data.insert("event_date".to_string(), json!("2025-12-31"));

    let mut validator = Validator::new(data);
    validator.rules(HashMap::from([
        (
            "birth_date",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(DateRule),
                Box::new(BeforeRule::new("2010-01-01")),
            ],
        ),
        (
            "event_date",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(DateRule),
                Box::new(AfterRule::new("2024-01-01")),
                Box::new(BeforeRule::new("2026-12-31")),
            ],
        ),
    ]));

    match validator.validate().await {
        Ok(_) => println!("✅ Date validation passed"),
        Err(errors) => {
            println!("❌ Validation failed:");
            print_errors(&errors);
        }
    }
}

async fn demo_user_registration() {
    let mut data = HashMap::new();
    data.insert("username".to_string(), json!("johndoe"));
    data.insert("email".to_string(), json!("john@example.com"));
    data.insert("age".to_string(), json!(25));
    data.insert("password".to_string(), json!("SecurePass123"));
    data.insert("password_confirmation".to_string(), json!("SecurePass123"));
    data.insert("terms_accepted".to_string(), json!(true));
    data.insert("tags".to_string(), json!(["developer", "rust"]));

    let mut validator = Validator::new(data);

    validator.rules(HashMap::from([
        (
            "username",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(MinLengthRule::new(3)),
                Box::new(MaxLengthRule::new(20)),
                Box::new(AlphaNumericRule),
            ],
        ),
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
        (
            "password",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(MinLengthRule::new(8)),
                Box::new(MaxLengthRule::new(128)),
            ],
        ),
        (
            "password_confirmation",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(SameRule::new("password")),
            ],
        ),
        (
            "tags",
            vec![
                Box::new(ArrayRule) as Box<dyn Rule>,
                Box::new(MinArraySizeRule::new(1)),
                Box::new(DistinctRule),
            ],
        ),
    ]));

    validator.messages(HashMap::from([
        ("username.required", "Username is required"),
        ("username.min_length", "Username must be at least 3 characters"),
        ("username.alpha_numeric", "Username can only contain letters and numbers"),
        ("email.required", "Email is required"),
        ("email.email", "Please provide a valid email address"),
        ("age.min", "You must be at least 18 years old"),
        ("password.min_length", "Password must be at least 8 characters"),
        ("password_confirmation.same", "Passwords must match"),
    ]));

    match validator.validate().await {
        Ok(validated) => {
            println!("✅ User registration validation successful!");
            println!("   Username: {}", validated.get_string("username").unwrap());
            println!("   Email: {}", validated.get_string("email").unwrap());
            println!("   Age: {}", validated.get_i64("age").unwrap());
            println!("   Tags: {:?}", validated.get("tags").unwrap());
        }
        Err(errors) => {
            println!("❌ Validation failed:");
            print_errors(&errors);
        }
    }
}

fn print_errors(errors: &ValidationErrors) {
    for (field, field_errors) in errors.field_errors() {
        for error in field_errors {
            println!("   - {}: {}", field, error.message);
        }
    }
}
