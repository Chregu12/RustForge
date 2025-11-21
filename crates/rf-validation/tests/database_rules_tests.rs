//! Comprehensive tests for generic database validation rules
//!
//! These tests verify that ExistsRule and UniqueRule with ValidatableEntity
//! perform actual database queries and correctly validate data.

use rf_validation::rules::database::{ExistsRule, UniqueRule, ValidatableEntity};
use rf_validation::validator::Rule;
use async_trait::async_trait;
use sea_orm::{
    entity::prelude::*, Database, DatabaseBackend, DatabaseConnection,
    Schema, Set, DbErr, Statement,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Test Entities - User
// ============================================================================

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub username: String,
    pub role_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// Create user module
pub mod user {
    pub use super::Entity;
    pub use super::Model;
    pub use super::ActiveModel;
    pub use super::Column;
}

// ============================================================================
// Test Entities - Role
// ============================================================================

pub mod role {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "roles")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub name: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// ValidatableEntity Implementations
// ============================================================================

/// Implementation of ValidatableEntity for User entity
#[async_trait]
impl ValidatableEntity for user::Entity {
    async fn exists_in_column(
        db: &DatabaseConnection,
        column: &str,
        value: &Value,
    ) -> Result<bool, DbErr> {
        // Match on column name to query the right column
        let count = match column {
            "id" => {
                let id = value.as_i64().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for id".to_string())
                })?;
                Entity::find()
                    .filter(Column::Id.eq(id))
                    .count(db)
                    .await?
            }
            "name" => {
                let name = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for name".to_string())
                })?;
                Entity::find()
                    .filter(Column::Name.eq(name))
                    .count(db)
                    .await?
            }
            "email" => {
                let email = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for email".to_string())
                })?;
                Entity::find()
                    .filter(Column::Email.eq(email))
                    .count(db)
                    .await?
            }
            "username" => {
                let username = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for username".to_string())
                })?;
                Entity::find()
                    .filter(Column::Username.eq(username))
                    .count(db)
                    .await?
            }
            _ => return Err(DbErr::Custom(format!("Unknown column: {}", column))),
        };

        Ok(count > 0)
    }

    async fn unique_in_column(
        db: &DatabaseConnection,
        column: &str,
        value: &Value,
        ignore_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        // Match on column name to query the right column
        let count = match column {
            "email" => {
                let email = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for email".to_string())
                })?;
                let mut query = Entity::find().filter(Column::Email.eq(email));

                if let Some(id) = ignore_id {
                    query = query.filter(Column::Id.ne(id));
                }

                query.count(db).await?
            }
            "username" => {
                let username = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for username".to_string())
                })?;
                let mut query = Entity::find().filter(Column::Username.eq(username));

                if let Some(id) = ignore_id {
                    query = query.filter(Column::Id.ne(id));
                }

                query.count(db).await?
            }
            "role_id" => {
                let role_id = value.as_i64().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for role_id".to_string())
                })?;
                let mut query = Entity::find().filter(Column::RoleId.eq(role_id));

                if let Some(id) = ignore_id {
                    query = query.filter(Column::Id.ne(id));
                }

                query.count(db).await?
            }
            _ => return Err(DbErr::Custom(format!("Unknown column: {}", column))),
        };

        Ok(count == 0)
    }

    fn table_name() -> &'static str {
        "users"
    }
}

/// Implementation of ValidatableEntity for Role entity
#[async_trait]
impl ValidatableEntity for role::Entity {
    async fn exists_in_column(
        db: &DatabaseConnection,
        column: &str,
        value: &Value,
    ) -> Result<bool, DbErr> {
        let count = match column {
            "id" => {
                let id = value.as_i64().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for id".to_string())
                })?;
                Entity::find()
                    .filter(Column::Id.eq(id))
                    .count(db)
                    .await?
            }
            "name" => {
                let name = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for name".to_string())
                })?;
                Entity::find()
                    .filter(Column::Name.eq(name))
                    .count(db)
                    .await?
            }
            _ => return Err(DbErr::Custom(format!("Unknown column: {}", column))),
        };

        Ok(count > 0)
    }

    async fn unique_in_column(
        db: &DatabaseConnection,
        column: &str,
        value: &Value,
        ignore_id: Option<i64>,
    ) -> Result<bool, DbErr> {
        let count = match column {
            "name" => {
                let name = value.as_str().ok_or_else(|| {
                    DbErr::Custom("Invalid value type for name".to_string())
                })?;
                let mut query = Entity::find().filter(Column::Name.eq(name));

                if let Some(id) = ignore_id {
                    query = query.filter(Column::Id.ne(id));
                }

                query.count(db).await?
            }
            _ => return Err(DbErr::Custom(format!("Unknown column: {}", column))),
        };

        Ok(count == 0)
    }

    fn table_name() -> &'static str {
        "roles"
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Setup an in-memory SQLite database for testing
async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    let db = Database::connect("sqlite::memory:").await?;

    // Create schema
    let schema = Schema::new(DatabaseBackend::Sqlite);

    // Create users table
    let stmt = schema.create_table_from_entity(user::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Create roles table
    let stmt = schema.create_table_from_entity(role::Entity);
    db.execute(db.get_database_backend().build(&stmt)).await?;

    // Insert test roles
    for (id, name) in [(1, "admin"), (2, "user"), (3, "moderator")] {
        let role = role::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
        };
        role.insert(&db).await?;
    }

    // Insert test users
    for (id, name, email, username, role_id) in [
        (1, "John Doe", "john@example.com", "john_doe", Some(2)),
        (2, "Jane Smith", "jane@example.com", "jane_smith", Some(1)),
        (3, "Bob Jones", "bob@example.com", "bob_jones", Some(2)),
    ] {
        let user = user::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            email: Set(email.to_string()),
            username: Set(username.to_string()),
            role_id: Set(role_id),
        };
        user.insert(&db).await?;
    }

    Ok(db)
}

// ============================================================================
// ExistsRule Tests
// ============================================================================

#[tokio::test]
async fn test_exists_rule_passes_for_existing_id() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let rule = ExistsRule::<user::Entity>::new(db, "id");
    let value = Value::Number(serde_json::Number::from(1));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for existing user ID");
}

#[tokio::test]
async fn test_exists_rule_fails_for_non_existing_id() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let rule = ExistsRule::<user::Entity>::new(db, "id");
    let value = Value::Number(serde_json::Number::from(999));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_err(), "Should fail for non-existing user ID");
    assert!(
        result.unwrap_err().contains("does not exist"),
        "Error message should mention 'does not exist'"
    );
}

#[tokio::test]
async fn test_exists_rule_with_string_column() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let rule = ExistsRule::<user::Entity>::new(db.clone(), "email");
    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for existing email");

    // Test with non-existing email
    let rule2 = ExistsRule::<user::Entity>::new(db, "email");
    let value2 = Value::String("nonexistent@example.com".to_string());

    let result2 = rule2.validate(&value2, &data).await;

    assert!(result2.is_err(), "Should fail for non-existing email");
}

#[tokio::test]
async fn test_exists_rule_with_null_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let rule = ExistsRule::<user::Entity>::new(db, "id");
    let value = Value::Null;
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for null value");
}

#[tokio::test]
async fn test_exists_rule_for_foreign_key_validation() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Valid foreign key (role_id)
    let rule = ExistsRule::<role::Entity>::new(db.clone(), "id");
    let value = Value::Number(serde_json::Number::from(2));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(
        result.is_ok(),
        "Should pass for valid foreign key (role_id = 2)"
    );

    // Invalid foreign key
    let rule2 = ExistsRule::<role::Entity>::new(db, "id");
    let value2 = Value::Number(serde_json::Number::from(999));

    let result2 = rule2.validate(&value2, &data).await;

    assert!(
        result2.is_err(),
        "Should fail for invalid foreign key (role_id = 999)"
    );
}

#[tokio::test]
async fn test_exists_rule_error_message_includes_table_and_column() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let rule = ExistsRule::<user::Entity>::new(db, "id");
    let value = Value::Number(serde_json::Number::from(999));
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("users"), "Error should mention table name");
    assert!(error.contains("id"), "Error should mention column name");
}

// ============================================================================
// UniqueRule Tests
// ============================================================================

#[tokio::test]
async fn test_unique_rule_fails_for_existing_email() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let rule = UniqueRule::<user::Entity>::new(db, "email");
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
    let db = Arc::new(db);

    let rule = UniqueRule::<user::Entity>::new(db, "email");
    let value = Value::String("new@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for new email");
}

#[tokio::test]
async fn test_unique_rule_with_except_excludes_current_record() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Test updating user ID 1's email to the same value (should pass)
    let rule = UniqueRule::<user::Entity>::new(db.clone(), "email").except(1);
    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(
        result.is_ok(),
        "Should pass when updating to same email with except()"
    );

    // Test updating user ID 1's email to another user's email (should fail)
    let rule2 = UniqueRule::<user::Entity>::new(db, "email").except(1);
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
    let db = Arc::new(db);

    let rule = UniqueRule::<user::Entity>::new(db, "email");
    let value = Value::Null;
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(result.is_ok(), "Should pass for null value");
}

#[tokio::test]
async fn test_unique_rule_with_numeric_value() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Test with existing role_id
    let rule = UniqueRule::<user::Entity>::new(db, "role_id");
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
async fn test_unique_rule_multiple_excepts() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // User ID 1 can keep their username
    let rule = UniqueRule::<user::Entity>::new(db, "username").except(1);
    let value = Value::String("john_doe".to_string());
    let data = HashMap::new();

    let result = rule.validate(&value, &data).await;

    assert!(
        result.is_ok(),
        "Should work with except() parameter"
    );
}

// ============================================================================
// Integration Tests - Real World Scenarios
// ============================================================================

#[tokio::test]
async fn test_user_registration_validation_workflow() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Scenario: User tries to register with existing email
    let email_rule = UniqueRule::<user::Entity>::new(db.clone(), "email");
    let username_rule = UniqueRule::<user::Entity>::new(db.clone(), "username");
    let role_rule = ExistsRule::<role::Entity>::new(db.clone(), "id");

    let data = HashMap::new();

    // Test with existing email (should fail)
    let email_result = email_rule
        .validate(&Value::String("john@example.com".to_string()), &data)
        .await;
    assert!(email_result.is_err(), "Email validation should fail");

    // Test with new email (should pass)
    let email_rule2 = UniqueRule::<user::Entity>::new(db.clone(), "email");
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
    let role_rule2 = ExistsRule::<role::Entity>::new(db, "id");
    let role_result2 = role_rule2
        .validate(&Value::Number(serde_json::Number::from(999)), &data)
        .await;
    assert!(role_result2.is_err(), "Invalid role_id should fail");
}

#[tokio::test]
async fn test_user_update_validation_workflow() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Scenario: User ID 1 updates their profile
    let user_id = 1;

    // Should allow keeping the same email
    let email_rule = UniqueRule::<user::Entity>::new(db.clone(), "email").except(user_id);
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
    let email_rule2 = UniqueRule::<user::Entity>::new(db, "email").except(user_id);
    let result3 = email_rule2
        .validate(&Value::String("newemail@example.com".to_string()), &data)
        .await;
    assert!(result3.is_ok(), "Should allow changing to new email");
}

#[tokio::test]
async fn test_rule_name_methods() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    let unique_rule = UniqueRule::<user::Entity>::new(db.clone(), "email");
    assert_eq!(unique_rule.name(), "unique");

    let exists_rule = ExistsRule::<role::Entity>::new(db, "id");
    assert_eq!(exists_rule.name(), "exists");
}

#[tokio::test]
async fn test_error_messages_formatting() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Test unique rule error message
    let unique_rule = UniqueRule::<user::Entity>::new(db.clone(), "email");
    let value = Value::String("john@example.com".to_string());
    let data = HashMap::new();

    let result = unique_rule.validate(&value, &data).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("email"));
    assert!(error.contains("already been taken"));

    // Test exists rule error message
    let exists_rule = ExistsRule::<role::Entity>::new(db, "id");
    let value2 = Value::Number(serde_json::Number::from(999));

    let result2 = exists_rule.validate(&value2, &data).await;
    assert!(result2.is_err());
    let error2 = result2.unwrap_err();
    assert!(error2.contains("does not exist"));
    assert!(error2.contains("roles"));
}

#[tokio::test]
async fn test_concurrent_validation() {
    let db = setup_test_db().await.expect("Failed to setup test database");
    let db = Arc::new(db);

    // Test that multiple validations can run concurrently
    let rule1 = ExistsRule::<user::Entity>::new(db.clone(), "id");
    let rule2 = UniqueRule::<user::Entity>::new(db, "email");

    let data = HashMap::new();
    let value1 = Value::Number(serde_json::Number::from(1));
    let value2 = Value::String("new@example.com".to_string());

    // Run validations concurrently
    let (result1, result2) = tokio::join!(
        rule1.validate(&value1, &data),
        rule2.validate(&value2, &data)
    );

    assert!(result1.is_ok(), "Exists validation should pass");
    assert!(result2.is_ok(), "Unique validation should pass");
}
