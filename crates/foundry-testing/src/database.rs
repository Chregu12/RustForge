use sea_orm::{Database, DatabaseConnection, DbErr, Statement};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test database helper for creating isolated test databases
pub struct TestDatabase {
    connection: DatabaseConnection,
    _temp_file: Option<tempfile::TempPath>,
}

impl TestDatabase {
    /// Create a new in-memory SQLite database for testing
    pub async fn new() -> Result<Self, DbErr> {
        let connection = Database::connect("sqlite::memory:").await?;
        Ok(Self {
            connection,
            _temp_file: None,
        })
    }

    /// Create a new file-based SQLite database for testing
    pub async fn new_file() -> Result<Self, DbErr> {
        let temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| DbErr::Custom(format!("Failed to create temp file: {}", e)))?;
        let path = temp_file.path().to_str().unwrap();
        let url = format!("sqlite://{}?mode=rwc", path);

        let connection = Database::connect(&url).await?;
        let temp_path = temp_file.into_temp_path();

        Ok(Self {
            connection,
            _temp_file: Some(temp_path),
        })
    }

    /// Create a test database with a specific URL (for PostgreSQL/MySQL testing)
    pub async fn with_url(url: &str) -> Result<Self, DbErr> {
        let connection = Database::connect(url).await?;
        Ok(Self {
            connection,
            _temp_file: None,
        })
    }

    /// Get the database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Execute a raw SQL statement
    pub async fn execute_sql(&self, sql: &str) -> Result<(), DbErr> {
        let backend = self.connection.get_database_backend();
        self.connection
            .execute(Statement::from_string(backend, sql.to_string()))
            .await?;
        Ok(())
    }

    /// Run multiple SQL statements (separated by semicolons)
    pub async fn execute_migration(&self, sql: &str) -> Result<(), DbErr> {
        for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
            self.execute_sql(statement).await?;
        }
        Ok(())
    }

    /// Create a standard test schema with products and accounts tables
    pub async fn create_test_schema(&self) -> Result<(), DbErr> {
        let schema = r#"
            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                price DECIMAL(10,2) NOT NULL,
                stock INTEGER NOT NULL,
                sku TEXT NOT NULL UNIQUE,
                active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            );

            CREATE TABLE IF NOT EXISTS accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                role TEXT NOT NULL,
                active BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            );
        "#;

        self.execute_migration(schema).await
    }

    /// Truncate all tables (for cleanup between tests)
    pub async fn truncate_all(&self) -> Result<(), DbErr> {
        self.execute_sql("DELETE FROM products").await?;
        self.execute_sql("DELETE FROM accounts").await?;
        Ok(())
    }

    /// Create a shared test database instance
    pub async fn shared() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new().await.expect("Failed to create test database")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_database() {
        let db = TestDatabase::new().await.unwrap();
        assert!(db.connection().ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_execute_sql() {
        let db = TestDatabase::new().await.unwrap();
        db.execute_sql("CREATE TABLE test (id INTEGER PRIMARY KEY)")
            .await
            .unwrap();

        // Verify table was created
        let result = db.execute_sql("SELECT * FROM test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_test_schema() {
        let db = TestDatabase::new().await.unwrap();
        db.create_test_schema().await.unwrap();

        // Verify tables were created
        assert!(db.execute_sql("SELECT * FROM products").await.is_ok());
        assert!(db.execute_sql("SELECT * FROM accounts").await.is_ok());
    }
}
