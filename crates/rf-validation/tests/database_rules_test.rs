//! Integration tests for database validation rules
//!
//! These tests verify that UniqueRule and ExistsRule perform actual database queries
//! and correctly validate data against database constraints.

use rf_validation::rules::database::{SimpleExistsRule, SimpleUniqueRule};
use rf_validation::validator::Rule;
use sea_orm::{
    ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement, DbErr,
};
use serde_json::Value;
use std::collections::HashMap;

/// Setup an in-memory SQLite database for testing
async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create test tables
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL UNIQUE,
            role_id INTEGER
        )
        "#.to_string(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE roles (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        )
        "#.to_string(),
    ))
    .await?;

    // Insert test data
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        INSERT INTO roles (id, name) VALUES
            (1, 'admin'),
            (2, 'user'),
            (3, 'moderator')
        "#.to_string(),
    ))
    .await?;

    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"
        INSERT INTO users (id, email, username, role_id) VALUES
            (1, 'john@example.com', 'john_doe', 2),
            (2, 'jane@example.com', 'jane_smith', 1),
            (3, 'bob@example.com', 'bob_jones', 2)
        "#.to_string(),
    ))
    .await?;

    Ok(db)
}

// ============================================================================
// SimpleUniqueRule Tests
// ============================================================================

#[tokio::test]
async fn test_unique_rule_fails_for_existing_email() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleUniqueRule::new(db, "users", "email");
    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_err(), "Should fail for existing email");
    assert!(
        result.unwrap_err().contains("already been taken"),
        "Error message should mention 'already been taken'"
    );
}

#[tokio::test]
async fn test_unique_rule_passes_for_new_email() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleUniqueRule::new(db, "users", "email");
    let value = Value::String("new@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for new email");
}

#[tokio::test]
async fn test_unique_rule_with_except_excludes_current_record() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    // Test updating user ID 1's email to the same value (should pass)
    let rule = SimpleUniqueRule::new(db.clone(), "users", "email").except(1);
    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(
        result.is_ok(),
        "Should pass when updating to same email with except()"
    );

    // Test updating user ID 1's email to another user's email (should fail)
    let rule2 = SimpleUniqueRule::new(db, "users", "email").except(1);
    let value2 = Value::String("jane@example.com".to_string());

    let result2 = rule2.validate(&value2, &data).await;

    assert!(
        result2.is_err(),
        "Should fail when trying to use another user's email"
    );
}

#[tokio::test]
async fn test_unique_rule_with_null_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleUniqueRule::new(db, "users", "email");
    let value = Value::Null;
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for null value");
}

#[tokio::test]
async fn test_unique_rule_with_numeric_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    // Test with existing role_id
    let rule = SimpleUniqueRule::new(db.clone(), "users", "role_id");
    let value = Value::Number(serde_json::Number::from(2));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    // Should fail because role_id 2 is used by multiple users
    assert!(
        result.is_err(),
        "Should fail for existing role_id used by multiple users"
    );
}

#[tokio::test]
async fn test_unique_rule_with_custom_id_column() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleUniqueRule::new(db, "users", "email")
        .with_id_column("id")
        .except(1);

    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(
        result.is_ok(),
        "Should work with custom id_column parameter"
    );
}

// ============================================================================
// SimpleExistsRule Tests
// ============================================================================

#[tokio::test]
async fn test_exists_rule_passes_for_existing_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleExistsRule::new(db, "roles", "id");
    let value = Value::Number(serde_json::Number::from(1));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for existing role ID");
}

#[tokio::test]
async fn test_exists_rule_fails_for_non_existing_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleExistsRule::new(db, "roles", "id");
    let value = Value::Number(serde_json::Number::from(999));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_err(), "Should fail for non-existing role ID");
    assert!(
        result.unwrap_err().contains("does not exist"),
        "Error message should mention 'does not exist'"
    );
}

#[tokio::test]
async fn test_exists_rule_with_string_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleExistsRule::new(db.clone(), "roles", "name");
    let value = Value::String("admin".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for existing role name");

    // Test with non-existing name
    let rule2 = SimpleExistsRule::new(db, "roles", "name");
    let value2 = Value::String("superadmin".to_string());

    let result2 = rule2.validate(&value2, &data).await;

    assert!(result2.is_err(), "Should fail for non-existing role name");
}

#[tokio::test]
async fn test_exists_rule_with_null_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let rule = SimpleExistsRule::new(db, "roles", "id");
    let value = Value::Null;
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for null value");
}

#[tokio::test]
async fn test_exists_rule_for_foreign_key_validation() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    // Valid foreign key
    let rule = SimpleExistsRule::new(db.clone(), "roles", "id");
    let value = Value::Number(serde_json::Number::from(2));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(
        result.is_ok(),
        "Should pass for valid foreign key (role_id = 2)"
    );

    // Invalid foreign key
    let rule2 = SimpleExistsRule::new(db, "roles", "id");
    let value2 = Value::Number(serde_json::Number::from(999));

    let result2 = rule2.validate(&value2, &data).await;

    assert!(
        result2.is_err(),
        "Should fail for invalid foreign key (role_id = 999)"
    );
}

// ============================================================================
// Integration Tests - Real World Scenarios
// ============================================================================

#[tokio::test]
async fn test_user_registration_validation() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    // Scenario: User tries to register with existing email
    let email_rule = SimpleUniqueRule::new(db.clone(), "users", "email");
    let username_rule = SimpleUniqueRule::new(db.clone(), "users", "username");
    let role_rule = SimpleExistsRule::new(db.clone(), "roles", "id");

    let data = HashMap::new();

    // Test with existing email (should fail)
    let email_result = email_rule
        .validate(&Value::String("john@example.com".to_string()), &data)
        .await;
    assert!(email_result.is_err(), "Email validation should fail");

    // Test with new email (should pass)
    let email_rule2 = SimpleUniqueRule::new(db.clone(), "users", "email");
    let email_result2 = email_rule2
        .validate(&Value::String("newuser@example.com".to_string()), &data)
        .await;
    assert!(email_result2.is_ok(), "New email validation should pass");

    // Test with existing username (should fail)
    let username_result = username_rule
        .validate(&Value::String("john_doe".to_string()), &data)
        .await;
    assert!(username_result.is_err(), "Username validation should fail");

    // Test with valid role_id (should pass)
    let role_result = role_rule
        .validate(&Value::Number(serde_json::Number::from(2)), &data)
        .await;
    assert!(role_result.is_ok(), "Valid role_id should pass");

    // Test with invalid role_id (should fail)
    let role_rule2 = SimpleExistsRule::new(db, "roles", "id");
    let role_result2 = role_rule2
        .validate(&Value::Number(serde_json::Number::from(999)), &data)
        .await;
    assert!(role_result2.is_err(), "Invalid role_id should fail");
}

#[tokio::test]
async fn test_user_update_validation() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    // Scenario: User ID 1 updates their profile
    let user_id = 1;

    // Should allow keeping the same email
    let email_rule = SimpleUniqueRule::new(db.clone(), "users", "email").except(user_id);
    let data = HashMap::new();

    let result = email_rule
        .validate(&Value::String("john@example.com".to_string()), &data)
        .await;
    assert!(
        result.is_ok(),
        "Should allow user to keep their own email"
    );

    // Should not allow using another user's email
    let result2 = email_rule
        .validate(&Value::String("jane@example.com".to_string()), &data)
        .await;
    assert!(
        result2.is_err(),
        "Should not allow using another user's email"
    );

    // Should allow changing to a completely new email
    let email_rule2 = SimpleUniqueRule::new(db, "users", "email").except(user_id);
    let result3 = email_rule2
        .validate(&Value::String("newemail@example.com".to_string()), &data)
        .await;
    assert!(result3.is_ok(), "Should allow changing to new email");
}

#[tokio::test]
async fn test_rule_name_methods() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    let unique_rule = SimpleUniqueRule::new(db.clone(), "users", "email");
    assert_eq!(unique_rule.name(), "unique");

    let exists_rule = SimpleExistsRule::new(db, "roles", "id");
    assert_eq!(exists_rule.name(), "exists");
}

#[tokio::test]
async fn test_error_messages() {
    let db = setup_test_db().await.expect("Failed to setup test database");

    // Test unique rule error message
    let unique_rule = SimpleUniqueRule::new(db.clone(), "users", "email");
    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = unique_rule.validate(&value, &data).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("email"));
    assert!(error.contains("already been taken"));

    // Test exists rule error message
    let exists_rule = SimpleExistsRule::new(db, "roles", "id");
    let value2 = Value::Number(serde_json::Number::from(999));

    let result2 = exists_rule.validate(&value2, &data).await;
    assert!(result2.is_err());
    let error2 = result2.unwrap_err();
    assert!(error2.contains("does not exist"));
    assert!(error2.contains("roles"));
}
