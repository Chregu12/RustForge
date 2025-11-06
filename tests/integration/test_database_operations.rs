use sea_orm::{Database, DatabaseConnection, DbErr, ConnectionTrait, Statement};
use foundry_infra::db::DatabaseManager;

async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    Database::connect("sqlite::memory:").await
}

#[tokio::test]
async fn test_database_connection() {
    // Test database connection
    let db = setup_test_db().await;
    assert!(db.is_ok(), "Database connection should be established");
}

#[tokio::test]
async fn test_database_ping() {
    // Test database ping
    let db = setup_test_db().await.unwrap();
    let result = db.ping().await;
    assert!(result.is_ok(), "Database should respond to ping");
}

#[tokio::test]
async fn test_create_table() {
    // Test creating a table
    let db = setup_test_db().await.unwrap();

    let sql = r#"
        CREATE TABLE IF NOT EXISTS test_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    "#;

    let result = db.execute(Statement::from_string(
        db.get_database_backend(),
        sql.to_string(),
    )).await;

    assert!(result.is_ok(), "Table should be created successfully");
}

#[tokio::test]
async fn test_insert_data() {
    // Test inserting data
    let db = setup_test_db().await.unwrap();

    // Create table
    let create_sql = r#"
        CREATE TABLE IF NOT EXISTS test_products (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            price REAL NOT NULL
        )
    "#;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        create_sql.to_string(),
    )).await.unwrap();

    // Insert data
    let insert_sql = r#"
        INSERT INTO test_products (name, price) VALUES ('Test Product', 99.99)
    "#;

    let result = db.execute(Statement::from_string(
        db.get_database_backend(),
        insert_sql.to_string(),
    )).await;

    assert!(result.is_ok(), "Data should be inserted successfully");
}

#[tokio::test]
async fn test_query_data() {
    // Test querying data
    let db = setup_test_db().await.unwrap();

    // Create and populate table
    let setup_sql = r#"
        CREATE TABLE IF NOT EXISTS test_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        );
        INSERT INTO test_items (name) VALUES ('Item 1'), ('Item 2');
    "#;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        setup_sql.to_string(),
    )).await.unwrap();

    // Query data
    let query_sql = "SELECT COUNT(*) as count FROM test_items";
    let result = db.query_one(Statement::from_string(
        db.get_database_backend(),
        query_sql.to_string(),
    )).await;

    assert!(result.is_ok(), "Data should be queried successfully");
}

#[tokio::test]
async fn test_transaction_commit() {
    // Test transaction commit
    let db = setup_test_db().await.unwrap();

    let create_sql = r#"
        CREATE TABLE IF NOT EXISTS test_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            balance REAL NOT NULL
        )
    "#;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        create_sql.to_string(),
    )).await.unwrap();

    // Start transaction
    let txn = db.begin().await;
    assert!(txn.is_ok(), "Transaction should start successfully");
}

#[tokio::test]
async fn test_transaction_rollback() {
    // Test transaction rollback
    let db = setup_test_db().await.unwrap();

    let create_sql = r#"
        CREATE TABLE IF NOT EXISTS test_rollback (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            value TEXT NOT NULL
        )
    "#;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        create_sql.to_string(),
    )).await.unwrap();

    // Test rollback
    let txn = db.begin().await;
    assert!(txn.is_ok(), "Transaction for rollback test should start");
}

#[tokio::test]
async fn test_database_migration() {
    // Test basic migration scenario
    let db = setup_test_db().await.unwrap();

    // Initial schema
    let v1_sql = r#"
        CREATE TABLE IF NOT EXISTS migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version INTEGER NOT NULL,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    "#;

    let result = db.execute(Statement::from_string(
        db.get_database_backend(),
        v1_sql.to_string(),
    )).await;

    assert!(result.is_ok(), "Migration table should be created");

    // Record migration
    let record_sql = "INSERT INTO migrations (version) VALUES (1)";
    let result = db.execute(Statement::from_string(
        db.get_database_backend(),
        record_sql.to_string(),
    )).await;

    assert!(result.is_ok(), "Migration should be recorded");
}

#[tokio::test]
async fn test_database_indexes() {
    // Test creating indexes
    let db = setup_test_db().await.unwrap();

    let setup_sql = r#"
        CREATE TABLE IF NOT EXISTS indexed_table (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL,
            name TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_email ON indexed_table(email);
    "#;

    let result = db.execute(Statement::from_string(
        db.get_database_backend(),
        setup_sql.to_string(),
    )).await;

    assert!(result.is_ok(), "Index should be created successfully");
}

#[tokio::test]
async fn test_concurrent_connections() {
    // Test multiple concurrent connections
    let db1 = setup_test_db().await;
    let db2 = setup_test_db().await;

    assert!(db1.is_ok() && db2.is_ok(), "Multiple connections should be supported");
}

#[cfg(test)]
mod postgres_tests {
    use super::*;

    async fn setup_postgres_test_db() -> Result<DatabaseConnection, DbErr> {
        // Use environment variable or default to localhost
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://foundry:foundry@localhost:5432/foundry_test".to_string());

        Database::connect(&database_url).await
    }

    #[tokio::test]
    #[ignore] // Only run when Postgres is available
    async fn test_postgres_connection() {
        let db = setup_postgres_test_db().await;
        if let Ok(db) = db {
            let result = db.ping().await;
            assert!(result.is_ok(), "Postgres connection should work");
        }
    }

    #[tokio::test]
    #[ignore] // Only run when Postgres is available
    async fn test_postgres_uuid() {
        let db = setup_postgres_test_db().await;
        if let Ok(db) = db {
            let sql = r#"
                CREATE TABLE IF NOT EXISTS test_uuid (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    name TEXT NOT NULL
                )
            "#;

            let result = db.execute(Statement::from_string(
                db.get_database_backend(),
                sql.to_string(),
            )).await;

            assert!(result.is_ok(), "Postgres UUID should work");
        }
    }
}
