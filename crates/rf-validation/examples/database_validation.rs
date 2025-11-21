//! Database Validation Rules Example
//!
//! This example demonstrates how to use the database validation rules
//! to validate uniqueness and foreign key existence.
//!
//! Run with: cargo run --example database_validation

use rf_validation::rules::database::{SimpleExistsRule, SimpleUniqueRule};
use rf_validation::validator::{Rule, Validator};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement};
use serde_json::json;
use std::collections::HashMap;

/// Setup an example database with users and roles
async fn setup_example_db() -> DatabaseConnection {
    // Connect to in-memory SQLite database
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to database");

    // Create tables
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE roles (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        )
        "#
        .to_string(),
    ))
    .await
    .expect("Failed to create roles table");

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL UNIQUE,
            role_id INTEGER,
            FOREIGN KEY (role_id) REFERENCES roles(id)
        )
        "#
        .to_string(),
    ))
    .await
    .expect("Failed to create users table");

    // Insert test data
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        INSERT INTO roles (id, name) VALUES
            (1, 'admin'),
            (2, 'user'),
            (3, 'moderator')
        "#
        .to_string(),
    ))
    .await
    .expect("Failed to insert roles");

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        INSERT INTO users (id, email, username, role_id) VALUES
            (1, 'john@example.com', 'john_doe', 2),
            (2, 'jane@example.com', 'jane_smith', 1)
        "#
        .to_string(),
    ))
    .await
    .expect("Failed to insert users");

    db
}

#[tokio::main]
async fn main() {
    println!("=== Database Validation Rules Example ===\n");

    let db = setup_example_db().await;

    // Example 1: User Registration Validation
    println!("1. User Registration Validation");
    println!("--------------------------------");

    let registration_data = HashMap::from([
        ("email".to_string(), json!("newuser@example.com")),
        ("username".to_string(), json!("new_user")),
        ("role_id".to_string(), json!(2)),
    ]);

    let mut validator = Validator::new(registration_data.clone());

    // Add validation rules
    let mut rules: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();

    rules.insert(
        "email",
        vec![Box::new(SimpleUniqueRule::new(
            db.clone(),
            "users",
            "email",
        ))],
    );

    rules.insert(
        "username",
        vec![Box::new(SimpleUniqueRule::new(
            db.clone(),
            "users",
            "username",
        ))],
    );

    rules.insert(
        "role_id",
        vec![Box::new(SimpleExistsRule::new(
            db.clone(),
            "roles",
            "id",
        ))],
    );

    validator.rules(rules);

    match validator.validate().await {
        Ok(_) => println!("✓ Registration data is valid!\n"),
        Err(errors) => {
            println!("✗ Validation failed:");
            for (field, error) in &errors.errors {
                println!("  - {}: {:?}", field, error);
            }
            println!();
        }
    }

    // Example 2: Duplicate Email (Should Fail)
    println!("2. Duplicate Email Validation (Should Fail)");
    println!("-------------------------------------------");

    let duplicate_email_data = HashMap::from([
        ("email".to_string(), json!("john@example.com")),
        ("username".to_string(), json!("another_user")),
        ("role_id".to_string(), json!(2)),
    ]);

    let mut validator2 = Validator::new(duplicate_email_data);

    let mut rules2: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();
    rules2.insert(
        "email",
        vec![Box::new(SimpleUniqueRule::new(
            db.clone(),
            "users",
            "email",
        ))],
    );

    validator2.rules(rules2);

    match validator2.validate().await {
        Ok(_) => println!("✓ Email is unique\n"),
        Err(errors) => {
            println!("✗ Validation failed (as expected):");
            for (field, error) in &errors.errors {
                println!("  - {}: {}", field, error[0].message);
            }
            println!();
        }
    }

    // Example 3: Invalid Foreign Key (Should Fail)
    println!("3. Invalid Foreign Key Validation (Should Fail)");
    println!("-----------------------------------------------");

    let invalid_role_data = HashMap::from([
        ("email".to_string(), json!("valid@example.com")),
        ("role_id".to_string(), json!(999)), // Non-existent role
    ]);

    let mut validator3 = Validator::new(invalid_role_data);

    let mut rules3: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();
    rules3.insert(
        "role_id",
        vec![Box::new(SimpleExistsRule::new(
            db.clone(),
            "roles",
            "id",
        ))],
    );

    validator3.rules(rules3);

    match validator3.validate().await {
        Ok(_) => println!("✓ Role ID is valid\n"),
        Err(errors) => {
            println!("✗ Validation failed (as expected):");
            for (field, error) in &errors.errors {
                println!("  - {}: {}", field, error[0].message);
            }
            println!();
        }
    }

    // Example 4: Update User with .except() - Keep Same Email
    println!("4. Update User Validation (Keep Same Email)");
    println!("--------------------------------------------");

    let user_id = 1; // Updating user with ID 1
    let update_data = HashMap::from([
        ("email".to_string(), json!("john@example.com")), // Same email as current
        ("username".to_string(), json!("john_updated")),
    ]);

    let mut validator4 = Validator::new(update_data);

    let mut rules4: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();
    rules4.insert(
        "email",
        vec![Box::new(
            SimpleUniqueRule::new(db.clone(), "users", "email").except(user_id),
        )],
    );
    rules4.insert(
        "username",
        vec![Box::new(
            SimpleUniqueRule::new(db.clone(), "users", "username").except(user_id),
        )],
    );

    validator4.rules(rules4);

    match validator4.validate().await {
        Ok(_) => println!("✓ Update data is valid (same email allowed with .except())\n"),
        Err(errors) => {
            println!("✗ Validation failed:");
            for (field, error) in &errors.errors {
                println!("  - {}: {:?}", field, error);
            }
            println!();
        }
    }

    // Example 5: Update User with Different User's Email (Should Fail)
    println!("5. Update User with Another User's Email (Should Fail)");
    println!("-------------------------------------------------------");

    let update_to_existing = HashMap::from([
        ("email".to_string(), json!("jane@example.com")), // Another user's email
    ]);

    let mut validator5 = Validator::new(update_to_existing);

    let mut rules5: HashMap<&str, Vec<Box<dyn Rule>>> = HashMap::new();
    rules5.insert(
        "email",
        vec![Box::new(
            SimpleUniqueRule::new(db.clone(), "users", "email").except(user_id),
        )],
    );

    validator5.rules(rules5);

    match validator5.validate().await {
        Ok(_) => println!("✓ Email is valid\n"),
        Err(errors) => {
            println!("✗ Validation failed (as expected):");
            for (field, error) in &errors.errors {
                println!("  - {}: {}", field, error[0].message);
            }
            println!();
        }
    }

    println!("=== All Examples Complete ===");
}
