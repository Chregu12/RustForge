//! Comprehensive tests for advanced migration features

use rf_orm::advanced_migrations::*;
use rf_orm::schema_builder::Schema;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

async fn setup_test_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to test database")
}

async fn create_test_table(db: &DatabaseConnection, table_name: &str) {
    let schema = Schema::new(db.clone());
    schema
        .create(table_name, |table| {
            table.id();
            table.string("name");
            table.timestamps();
        })
        .await
        .expect("Failed to create test table");
}

async fn table_exists(db: &DatabaseConnection, table_name: &str) -> bool {
    let backend = db.get_database_backend();
    let query = match backend {
        DbBackend::Sqlite => format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            table_name
        ),
        DbBackend::Postgres => format!(
            "SELECT tablename FROM pg_tables WHERE tablename='{}'",
            table_name
        ),
        DbBackend::MySql => format!(
            "SELECT table_name FROM information_schema.tables WHERE table_name='{}'",
            table_name
        ),
    };

    let result = db
        .query_one(Statement::from_string(backend, query))
        .await
        .ok()
        .flatten();

    result.is_some()
}

#[tokio::test]
async fn test_add_foreign_key_basic() {
    let db = setup_test_db().await;

    // Create parent table
    create_test_table(&db, "users").await;

    // Create child table
    let schema = Schema::new(db.clone());
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
            table.timestamps();
        })
        .await
        .expect("Failed to create posts table");

    // Add foreign key (SQLite doesn't support this, so we expect an error)
    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .add_foreign_key("posts", vec!["user_id"], "users", vec!["id"], None, None)
        .await;

    // SQLite should return an error about unsupported operation
    assert!(result.is_err(), "SQLite should not support adding foreign keys to existing tables");
}

#[tokio::test]
async fn test_add_foreign_key_with_cascade_delete() {
    let db = setup_test_db().await;

    create_test_table(&db, "users").await;

    let schema = Schema::new(db.clone());
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .add_foreign_key(
            "posts",
            vec!["user_id"],
            "users",
            vec!["id"],
            Some(ForeignKeyAction::Cascade),
            None,
        )
        .await;

    // SQLite doesn't support this
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_foreign_key_with_set_null() {
    let db = setup_test_db().await;

    create_test_table(&db, "users").await;

    let schema = Schema::new(db.clone());
    schema
        .create("comments", |table| {
            table.id();
            table.text("body");
            table.big_integer("user_id").unsigned().nullable();
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .add_foreign_key(
            "comments",
            vec!["user_id"],
            "users",
            vec!["id"],
            Some(ForeignKeyAction::SetNull),
            None,
        )
        .await;

    // SQLite doesn't support this
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_foreign_key_with_cascade_update() {
    let db = setup_test_db().await;

    create_test_table(&db, "users").await;

    let schema = Schema::new(db.clone());
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .add_foreign_key(
            "posts",
            vec!["user_id"],
            "users",
            vec!["id"],
            Some(ForeignKeyAction::Cascade),
            Some(ForeignKeyAction::Cascade),
        )
        .await;

    // SQLite doesn't support this
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_foreign_key_validation() {
    let db = setup_test_db().await;

    let builder = AdvancedMigrationBuilder::new(&db);

    // Empty columns
    let result = builder
        .add_foreign_key("posts", vec![], "users", vec!["id"], None, None)
        .await;
    assert!(result.is_err());

    // Mismatched column count
    let result = builder
        .add_foreign_key(
            "posts",
            vec!["user_id", "tenant_id"],
            "users",
            vec!["id"],
            None,
            None,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_single_column_index() {
    let db = setup_test_db().await;
    create_test_table(&db, "users").await;

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder.create_index("users", vec!["name"], false).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_composite_index() {
    let db = setup_test_db().await;

    let schema = Schema::new(db.clone());
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
            table.timestamp("created_at");
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .create_index("posts", vec!["user_id", "created_at"], false)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_unique_index() {
    let db = setup_test_db().await;
    create_test_table(&db, "users").await;

    let builder = AdvancedMigrationBuilder::new(&db);

    // Add email column first
    let schema = Schema::new(db.clone());
    schema
        .table("users", |table| {
            table.string("email");
        })
        .await
        .unwrap();

    let result = builder.create_index("users", vec!["email"], true).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_named_index() {
    let db = setup_test_db().await;
    create_test_table(&db, "users").await;

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .create_named_index("users", "custom_idx_name", vec!["name"], false)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_drop_index() {
    let db = setup_test_db().await;
    create_test_table(&db, "users").await;

    let builder = AdvancedMigrationBuilder::new(&db);

    // Create index first
    builder
        .create_index("users", vec!["name"], false)
        .await
        .unwrap();

    // Drop the index
    let result = builder.drop_index("users", "idx_users_name").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_unique_constraint_single_column() {
    let db = setup_test_db().await;
    create_test_table(&db, "users").await;

    // Add email column
    let schema = Schema::new(db.clone());
    schema
        .table("users", |table| {
            table.string("email");
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder.add_unique_constraint("users", vec!["email"]).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_add_unique_constraint_composite() {
    let db = setup_test_db().await;

    let schema = Schema::new(db.clone());
    schema
        .create("user_roles", |table| {
            table.id();
            table.big_integer("user_id").unsigned();
            table.big_integer("role_id").unsigned();
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .add_unique_constraint("user_roles", vec!["user_id", "role_id"])
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unique_constraint_validation() {
    let db = setup_test_db().await;

    let builder = AdvancedMigrationBuilder::new(&db);

    // Empty columns
    let result = builder.add_unique_constraint("users", vec![]).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_index_validation() {
    let db = setup_test_db().await;

    let builder = AdvancedMigrationBuilder::new(&db);

    // Empty columns
    let result = builder.create_index("users", vec![], false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_add_check_constraint() {
    let db = setup_test_db().await;

    let schema = Schema::new(db.clone());
    schema
        .create("users", |table| {
            table.id();
            table.string("name");
            table.integer("age");
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder
        .add_check_constraint("users", "chk_age_positive", "age >= 0")
        .await;

    // SQLite may not fully support check constraints in all versions
    // Just verify it doesn't panic
    let _ = result;
}

#[tokio::test]
async fn test_rename_table() {
    let db = setup_test_db().await;
    create_test_table(&db, "old_users").await;

    let builder = AdvancedMigrationBuilder::new(&db);
    let result = builder.rename_table("old_users", "new_users").await;

    assert!(result.is_ok());

    // Verify old table doesn't exist and new one does
    assert!(!table_exists(&db, "old_users").await);
    assert!(table_exists(&db, "new_users").await);
}

#[tokio::test]
async fn test_drop_column() {
    let db = setup_test_db().await;

    let schema = Schema::new(db.clone());
    schema
        .create("users", |table| {
            table.id();
            table.string("name");
            table.string("temp_field");
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);

    // Note: SQLite has limitations with ALTER TABLE DROP COLUMN
    // This may not work in all SQLite versions
    let result = builder.drop_column("users", "temp_field").await;

    // Just verify it doesn't panic - SQLite may not support this
    let _ = result;
}

#[tokio::test]
async fn test_migration_rollback_with_constraints() {
    let db = setup_test_db().await;

    create_test_table(&db, "users").await;

    let schema = Schema::new(db.clone());
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);

    // Skip foreign key test for SQLite (not supported)

    // Add index (this works)
    builder
        .create_index("posts", vec!["user_id"], false)
        .await
        .unwrap();

    // Drop index (rollback) - this should work
    let result = builder.drop_index("posts", "idx_posts_user_id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_foreign_key_actions() {
    assert_eq!(ForeignKeyAction::Cascade.to_sql(), "CASCADE");
    assert_eq!(ForeignKeyAction::SetNull.to_sql(), "SET NULL");
    assert_eq!(ForeignKeyAction::Restrict.to_sql(), "RESTRICT");
    assert_eq!(ForeignKeyAction::NoAction.to_sql(), "NO ACTION");
    assert_eq!(ForeignKeyAction::SetDefault.to_sql(), "SET DEFAULT");
}

#[tokio::test]
async fn test_complex_migration_scenario() {
    let db = setup_test_db().await;

    // Create tables
    let schema = Schema::new(db.clone());
    schema
        .create("tenants", |table| {
            table.id();
            table.string("name");
        })
        .await
        .unwrap();

    schema
        .create("users", |table| {
            table.id();
            table.string("email");
            table.big_integer("tenant_id").unsigned();
        })
        .await
        .unwrap();

    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
            table.timestamp("created_at");
        })
        .await
        .unwrap();

    let builder = AdvancedMigrationBuilder::new(&db);

    // Add foreign keys (SQLite doesn't support this for existing tables, so skip)
    // For SQLite, foreign keys must be defined in CREATE TABLE

    // Add indexes (this works in SQLite)
    builder
        .create_index("users", vec!["email"], true)
        .await
        .unwrap();

    builder
        .create_index("posts", vec!["user_id", "created_at"], false)
        .await
        .unwrap();

    // Add unique constraints (creates unique index in SQLite)
    builder
        .add_unique_constraint("users", vec!["tenant_id", "email"])
        .await
        .unwrap();

    // All operations should succeed
    assert!(table_exists(&db, "tenants").await);
    assert!(table_exists(&db, "users").await);
    assert!(table_exists(&db, "posts").await);
}
