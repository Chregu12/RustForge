//! Integration tests for Schema Builder

use rf_orm::prelude::*;
use sea_orm::ConnectionTrait;

/// Helper to create in-memory SQLite database for testing
async fn setup_test_db() -> (DatabaseConnection, Schema) {
    let db = DatabaseManager::connect(DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        ..Default::default()
    })
    .await
    .unwrap();

    let schema = Schema::new(db.connection().clone());
    (db.connection().clone(), schema)
}

#[tokio::test]
async fn test_create_simple_table() {
    let (db, schema) = setup_test_db().await;

    // Create table with basic columns
    schema
        .create("users", |table| {
            table.id();
            table.string("email");
            table.string("name");
        })
        .await
        .unwrap();

    // Verify table exists by querying it
    let result = db.execute_unprepared("SELECT * FROM users").await;
    assert!(result.is_ok(), "Table should exist and be queryable");
}

#[tokio::test]
async fn test_create_table_with_all_column_types() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("all_types", |table| {
            table.id();
            table.string("str_col");
            table.string_with_length("str_len_col", 100);
            table.text("text_col");
            table.integer("int_col");
            table.big_integer("bigint_col");
            table.tiny_integer("tinyint_col");
            table.float("float_col");
            table.double("double_col");
            table.decimal("decimal_col", 10, 2);
            table.boolean("bool_col");
            table.json("json_col");
            table.date("date_col");
            table.datetime("datetime_col");
            table.timestamp("timestamp_col");
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_column_modifiers() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("products", |table| {
            table.id();
            table.string("sku").unique();
            table.string("name");
            table.text("description").nullable();
            table.integer("stock").default("0").unsigned();
            table.decimal("price", 10, 2).default("0.00");
            table.boolean("active").default("true");
        })
        .await
        .unwrap();

    // Insert test data to verify defaults and constraints
    let result = db
        .execute_unprepared("INSERT INTO products (sku, name) VALUES ('SKU001', 'Test Product')")
        .await;
    assert!(result.is_ok());

    // Verify default values were applied
    use sea_orm::FromQueryResult;

    #[derive(Debug, FromQueryResult)]
    struct ProductRow {
        stock: i32,
        active: i32,
    }

    let row = ProductRow::find_by_statement(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT stock, active FROM products WHERE sku = 'SKU001'".to_string(),
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap();

    assert_eq!(row.stock, 0);
    assert_eq!(row.active, 1); // true as integer
}

#[tokio::test]
async fn test_nullable_columns() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.text("body").nullable();
            table.string("excerpt").nullable();
        })
        .await
        .unwrap();

    // Insert with NULL values
    let result = db
        .execute_unprepared("INSERT INTO posts (title, body, excerpt) VALUES ('Test', NULL, NULL)")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unique_constraint() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("emails", |table| {
            table.id();
            table.string("email").unique();
        })
        .await
        .unwrap();

    // First insert should succeed
    let result = db
        .execute_unprepared("INSERT INTO emails (email) VALUES ('test@example.com')")
        .await;
    assert!(result.is_ok());

    // Second insert with same email should fail
    let result = db
        .execute_unprepared("INSERT INTO emails (email) VALUES ('test@example.com')")
        .await;
    assert!(
        result.is_err(),
        "Unique constraint should prevent duplicate emails"
    );
}

#[tokio::test]
async fn test_timestamps() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("articles", |table| {
            table.id();
            table.string("title");
            table.timestamps();
        })
        .await
        .unwrap();

    // Verify created_at and updated_at columns exist
    let result = db
        .execute_unprepared(
            "INSERT INTO articles (title, created_at, updated_at) VALUES ('Test', datetime('now'), datetime('now'))",
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_soft_deletes() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("users", |table| {
            table.id();
            table.string("email");
            table.timestamps();
            table.soft_deletes();
        })
        .await
        .unwrap();

    // Verify deleted_at column exists and accepts NULL
    let result = db
        .execute_unprepared(
            "INSERT INTO users (email, created_at, updated_at, deleted_at) VALUES ('test@example.com', datetime('now'), datetime('now'), NULL)",
        )
        .await;
    assert!(result.is_ok());

    // Update with deleted_at timestamp (soft delete)
    let result = db
        .execute_unprepared(
            "UPDATE users SET deleted_at = datetime('now') WHERE email = 'test@example.com'",
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_foreign_key() {
    let (db, schema) = setup_test_db().await;

    // Create parent table
    schema
        .create("users", |table| {
            table.id();
            table.string("email");
        })
        .await
        .unwrap();

    // Create child table with foreign key
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.big_integer("user_id").unsigned();
            table
                .foreign("user_id")
                .references("id")
                .on("users")
                .on_delete("cascade");
        })
        .await
        .unwrap();

    // Insert parent
    db.execute_unprepared("INSERT INTO users (email) VALUES ('user@example.com')")
        .await
        .unwrap();

    // Insert child referencing parent
    let result = db
        .execute_unprepared("INSERT INTO posts (title, user_id) VALUES ('Test Post', 1)")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_single_column_index() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("posts", |table| {
            table.id();
            table.string("slug");
            table.boolean("published");
            table.index("slug");
            table.index("published");
        })
        .await
        .unwrap();

    // SQLite doesn't have a direct way to query indexes easily,
    // but we can verify the table was created successfully
    let result = db.execute_unprepared("SELECT * FROM posts").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_composite_index() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("posts", |table| {
            table.id();
            table.big_integer("user_id");
            table.boolean("published");
            table.timestamp("created_at");
            table.index(&["user_id", "published"]);
            table.index(&["published", "created_at"]);
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_composite_unique() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("user_roles", |table| {
            table.id();
            table.big_integer("user_id");
            table.big_integer("role_id");
            table.unique(&["user_id", "role_id"]);
        })
        .await
        .unwrap();

    // Insert first combination
    db.execute_unprepared("INSERT INTO user_roles (user_id, role_id) VALUES (1, 1)")
        .await
        .unwrap();

    // Same combination should fail
    let result = db
        .execute_unprepared("INSERT INTO user_roles (user_id, role_id) VALUES (1, 1)")
        .await;
    assert!(
        result.is_err(),
        "Composite unique should prevent duplicates"
    );

    // Different combination should succeed
    let result = db
        .execute_unprepared("INSERT INTO user_roles (user_id, role_id) VALUES (1, 2)")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_drop_table() {
    let (db, schema) = setup_test_db().await;

    // Create table
    schema
        .create("temp_table", |table| {
            table.id();
            table.string("name");
        })
        .await
        .unwrap();

    // Verify it exists
    let result = db.execute_unprepared("SELECT * FROM temp_table").await;
    assert!(result.is_ok());

    // Drop it
    schema.drop("temp_table").await.unwrap();

    // Verify it's gone
    let result = db.execute_unprepared("SELECT * FROM temp_table").await;
    assert!(result.is_err(), "Table should no longer exist after drop");
}

#[tokio::test]
async fn test_drop_if_exists() {
    let (db, schema) = setup_test_db().await;

    // Drop non-existent table should succeed
    let result = schema.drop_if_exists("non_existent_table").await;
    assert!(result.is_ok());

    // Create and drop should also succeed
    schema
        .create("temp_table", |table| {
            table.id();
            table.string("name");
        })
        .await
        .unwrap();

    let result = schema.drop_if_exists("temp_table").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_alter_table_add_column() {
    let (db, schema) = setup_test_db().await;

    // Create initial table
    schema
        .create("users", |table| {
            table.id();
            table.string("email");
        })
        .await
        .unwrap();

    // Add new column
    schema
        .table("users", |table| {
            table.string("phone").nullable();
        })
        .await
        .unwrap();

    // Verify new column exists
    let result = db
        .execute_unprepared(
            "INSERT INTO users (email, phone) VALUES ('test@example.com', '123-456-7890')",
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_alter_table_add_index() {
    let (db, schema) = setup_test_db().await;

    // Create initial table
    schema
        .create("posts", |table| {
            table.id();
            table.string("title");
            table.boolean("published");
        })
        .await
        .unwrap();

    // Add index to existing table
    schema
        .table("posts", |table| {
            table.index("published");
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_complete_blog_schema() {
    let (db, schema) = setup_test_db().await;

    // Users table
    schema
        .create("users", |table| {
            table.id();
            table.string("email").unique();
            table.string("username").unique();
            table.string("password");
            table.string("name");
            table.text("bio").nullable();
            table.timestamps();
            table.soft_deletes();
        })
        .await
        .unwrap();

    // Posts table
    schema
        .create("posts", |table| {
            table.id();
            table.big_integer("user_id").unsigned();
            table.string("title");
            table.string("slug").unique();
            table.text("body");
            table.text("excerpt").nullable();
            table.boolean("published").default("false");
            table.integer("views").default("0").unsigned();
            table.timestamp("published_at").nullable();
            table.timestamps();
            table.soft_deletes();

            table
                .foreign("user_id")
                .references("id")
                .on("users")
                .on_delete("cascade");

            table.index("published");
            table.index(&["user_id", "published"]);
        })
        .await
        .unwrap();

    // Comments table
    schema
        .create("comments", |table| {
            table.id();
            table.big_integer("post_id").unsigned();
            table.big_integer("user_id").unsigned();
            table.text("body");
            table.boolean("approved").default("false");
            table.timestamps();
            table.soft_deletes();

            table
                .foreign("post_id")
                .references("id")
                .on("posts")
                .on_delete("cascade");

            table
                .foreign("user_id")
                .references("id")
                .on("users")
                .on_delete("cascade");

            table.index(&["post_id", "approved"]);
        })
        .await
        .unwrap();

    // Tags table
    schema
        .create("tags", |table| {
            table.id();
            table.string("name").unique();
            table.string("slug").unique();
            table.timestamps();
        })
        .await
        .unwrap();

    // Post-Tag pivot table
    schema
        .create("post_tag", |table| {
            table.id();
            table.big_integer("post_id").unsigned();
            table.big_integer("tag_id").unsigned();
            table.timestamps();

            table
                .foreign("post_id")
                .references("id")
                .on("posts")
                .on_delete("cascade");

            table
                .foreign("tag_id")
                .references("id")
                .on("tags")
                .on_delete("cascade");

            table.unique(&["post_id", "tag_id"]);
        })
        .await
        .unwrap();

    // Insert test data
    db.execute_unprepared(
        "INSERT INTO users (email, username, password, name) VALUES ('john@example.com', 'john', 'hashed', 'John Doe')",
    )
    .await
    .unwrap();

    db.execute_unprepared(
        "INSERT INTO posts (user_id, title, slug, body, published) VALUES (1, 'Test Post', 'test-post', 'Body content', true)",
    )
    .await
    .unwrap();

    db.execute_unprepared(
        "INSERT INTO comments (post_id, user_id, body) VALUES (1, 1, 'Great post!')",
    )
    .await
    .unwrap();

    db.execute_unprepared("INSERT INTO tags (name, slug) VALUES ('Rust', 'rust')")
        .await
        .unwrap();

    db.execute_unprepared("INSERT INTO post_tag (post_id, tag_id) VALUES (1, 1)")
        .await
        .unwrap();

    // Verify relationships work
    use sea_orm::FromQueryResult;

    #[derive(Debug, FromQueryResult)]
    struct UserPost {
        name: String,
        title: String,
    }

    let row = UserPost::find_by_statement(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id WHERE u.id = 1"
            .to_string(),
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap();

    assert_eq!(row.name, "John Doe");
    assert_eq!(row.title, "Test Post");
}

#[tokio::test]
async fn test_json_columns() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("documents", |table| {
            table.id();
            table.string("title");
            table.json("metadata");
            table.json("settings").nullable();
        })
        .await
        .unwrap();

    // Insert JSON data
    let result = db
        .execute_unprepared(
            r#"INSERT INTO documents (title, metadata, settings)
               VALUES ('Doc1', '{"author": "John", "version": 1}', '{"theme": "dark"}')"#,
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_decimal_precision() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("financials", |table| {
            table.id();
            table.decimal("amount", 10, 2);
            table.decimal("rate", 5, 4);
        })
        .await
        .unwrap();

    // Insert decimal values
    let result = db
        .execute_unprepared("INSERT INTO financials (amount, rate) VALUES (12345.67, 0.0825)")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_default_values_with_quotes() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("settings", |table| {
            table.id();
            table.string("key");
            table.string("value").default("'default_value'");
            table.string("status").default("'active'");
        })
        .await
        .unwrap();

    // Insert without providing defaults
    db.execute_unprepared("INSERT INTO settings (key) VALUES ('test_key')")
        .await
        .unwrap();

    // Verify defaults were applied
    use sea_orm::FromQueryResult;

    #[derive(Debug, FromQueryResult)]
    struct SettingRow {
        value: String,
        status: String,
    }

    let row = SettingRow::find_by_statement(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT value, status FROM settings WHERE key = 'test_key'".to_string(),
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap();

    assert_eq!(row.value, "default_value");
    assert_eq!(row.status, "active");
}

#[tokio::test]
async fn test_unsigned_integers() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("counters", |table| {
            table.id();
            table.integer("count").unsigned().default("0");
            table.big_integer("total").unsigned().default("0");
        })
        .await
        .unwrap();

    // Insert test data
    let result = db
        .execute_unprepared("INSERT INTO counters (count, total) VALUES (100, 1000000)")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_indexes_on_table() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("products", |table| {
            table.id();
            table.string("sku").unique();
            table.string("name");
            table.integer("category_id");
            table.boolean("active");
            table.timestamp("created_at");

            table.index("name");
            table.index("category_id");
            table.index("active");
            table.index(&["category_id", "active"]);
            table.index(&["active", "created_at"]);
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_foreign_key_actions() {
    let (db, schema) = setup_test_db().await;

    // Parent table
    schema
        .create("categories", |table| {
            table.id();
            table.string("name");
        })
        .await
        .unwrap();

    // Child with cascade delete
    schema
        .create("products", |table| {
            table.id();
            table.big_integer("category_id").unsigned();
            table.string("name");

            table
                .foreign("category_id")
                .references("id")
                .on("categories")
                .on_delete("cascade")
                .on_update("cascade");
        })
        .await
        .unwrap();

    // Insert parent
    db.execute_unprepared("INSERT INTO categories (name) VALUES ('Electronics')")
        .await
        .unwrap();

    // Insert child
    db.execute_unprepared("INSERT INTO products (category_id, name) VALUES (1, 'Laptop')")
        .await
        .unwrap();

    // Delete parent should cascade to child
    db.execute_unprepared("DELETE FROM categories WHERE id = 1")
        .await
        .unwrap();

    // Verify child was deleted
    use sea_orm::FromQueryResult;

    #[derive(Debug, FromQueryResult)]
    struct CountRow {
        count: i32,
    }

    let row = CountRow::find_by_statement(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT COUNT(*) as count FROM products".to_string(),
    ))
    .one(&db)
    .await
    .unwrap()
    .unwrap();

    assert_eq!(row.count, 0, "Child records should be cascade deleted");
}

#[tokio::test]
async fn test_string_length_variations() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("strings", |table| {
            table.id();
            table.string("short_code"); // Default 255
            table.string_with_length("custom_short", 50);
            table.string_with_length("custom_long", 500);
            table.text("unlimited");
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_all_integer_types() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("integers", |table| {
            table.id();
            table.tiny_integer("tiny").default("0");
            table.integer("normal").default("0");
            table.big_integer("big").default("0");
            table.tiny_integer("unsigned_tiny").unsigned();
            table.integer("unsigned_normal").unsigned();
            table.big_integer("unsigned_big").unsigned();
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_datetime_types() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("events", |table| {
            table.id();
            table.string("name");
            table.date("event_date");
            table.datetime("starts_at");
            table.datetime("ends_at").nullable();
            table.timestamp("registered_at");
        })
        .await
        .unwrap();

    // Insert with datetime values
    let result = db
        .execute_unprepared(
            "INSERT INTO events (name, event_date, starts_at, registered_at)
             VALUES ('Conference', '2024-12-01', '2024-12-01 09:00:00', datetime('now'))",
        )
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_chainable_modifiers() {
    let (db, schema) = setup_test_db().await;

    schema
        .create("chained", |table| {
            table.id();
            table.string("email").unique().comment("User email address");
            table
                .integer("score")
                .default("0")
                .unsigned()
                .index()
                .comment("User score");
            table.string("status").default("'active'").index();
        })
        .await
        .unwrap();
}
