//! DB facade providing Laravel-style static database API

use crate::manager::GLOBAL_DB;
use crate::query_builder::QueryBuilder;
use serde_json::Value;

/// The DB facade providing a static-like API for database operations.
///
/// This is the main entry point for database operations in your application.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_db_facade::DB;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Select records
/// let users = DB::select("SELECT * FROM users", &[]).await?;
///
/// // Insert a record
/// let id = DB::insert("INSERT INTO users (name) VALUES (?)",
///     &["John".into()]).await?;
///
/// // Update records
/// let affected = DB::update("UPDATE users SET active = ?",
///     &[true.into()]).await?;
///
/// // Delete records
/// let deleted = DB::delete("DELETE FROM users WHERE id = ?",
///     &[id.into()]).await?;
///
/// // Use query builder
/// let users = DB::table("users")
///     .where_clause("active", "=", true.into())
///     .get().await?;
/// # Ok(())
/// # }
/// ```
pub struct DB;

impl DB {
    /// Execute a select query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let users = DB::select("SELECT * FROM users WHERE active = ?",
    ///     &[true.into()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn select(query: &str, bindings: &[Value]) -> Result<Vec<Value>, String> {
        let manager = GLOBAL_DB.read();
        manager.select(query, bindings)
    }

    /// Execute an insert query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let id = DB::insert("INSERT INTO users (name, email) VALUES (?, ?)",
    ///     &["John".into(), "john@example.com".into()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn insert(query: &str, bindings: &[Value]) -> Result<u64, String> {
        let mut manager = GLOBAL_DB.write();
        manager.insert(query, bindings)
    }

    /// Execute an update query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let affected = DB::update("UPDATE users SET active = ? WHERE id = ?",
    ///     &[true.into(), 1.into()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update(query: &str, bindings: &[Value]) -> Result<u64, String> {
        let mut manager = GLOBAL_DB.write();
        manager.update(query, bindings)
    }

    /// Execute a delete query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = DB::delete("DELETE FROM users WHERE id = ?",
    ///     &[1.into()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(query: &str, bindings: &[Value]) -> Result<u64, String> {
        let mut manager = GLOBAL_DB.write();
        manager.delete(query, bindings)
    }

    /// Execute a statement
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// DB::statement("CREATE TABLE users (id INT, name VARCHAR(255))").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn statement(query: &str) -> Result<bool, String> {
        let mut manager = GLOBAL_DB.write();
        manager.statement(query)
    }

    /// Get a query builder for a table
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let users = DB::table("users")
    ///     .where_clause("active", "=", true.into())
    ///     .limit(10)
    ///     .get().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn table(name: &str) -> QueryBuilder {
        QueryBuilder::new(name)
    }

    /// Begin a database transaction
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// DB::begin_transaction().await?;
    /// // ... perform database operations
    /// DB::commit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn begin_transaction() -> Result<(), String> {
        let mut manager = GLOBAL_DB.write();
        manager.begin_transaction()
    }

    /// Commit the current transaction
    pub async fn commit() -> Result<(), String> {
        let mut manager = GLOBAL_DB.write();
        manager.commit()
    }

    /// Rollback the current transaction
    pub async fn rollback() -> Result<(), String> {
        let mut manager = GLOBAL_DB.write();
        manager.rollback()
    }

    /// Set the database connection to use
    pub async fn connection(name: &str) -> Result<(), String> {
        let mut manager = GLOBAL_DB.write();
        manager.set_connection(name.to_string());
        Ok(())
    }

    /// Get the current connection name
    pub async fn connection_name() -> String {
        let manager = GLOBAL_DB.read();
        manager.connection_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_insert() {
        let bindings = vec![
            serde_json::json!("John"),
            serde_json::json!("john@example.com")
        ];

        let result = DB::insert("INSERT INTO users (name, email) VALUES (?, ?)", &bindings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_db_select() {
        let result = DB::select("SELECT * FROM users", &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_db_update() {
        let bindings = vec![serde_json::json!(true)];
        let result = DB::update("UPDATE users SET active = ?", &bindings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_db_delete() {
        let bindings = vec![serde_json::json!(1)];
        let result = DB::delete("DELETE FROM users WHERE id = ?", &bindings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_db_statement() {
        let result = DB::statement("CREATE TABLE users (id INT)").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_db_table() {
        let builder = DB::table("users");
        assert_eq!(builder.table_name(), "users");
    }

    #[tokio::test]
    async fn test_db_transaction() {
        assert!(DB::begin_transaction().await.is_ok());
        assert!(DB::commit().await.is_ok());
        assert!(DB::rollback().await.is_ok());
    }

    #[tokio::test]
    async fn test_db_connection() {
        assert!(DB::connection("mysql").await.is_ok());
        let name = DB::connection_name().await;
        assert_eq!(name, "mysql");
    }

    #[tokio::test]
    async fn test_db_query_builder_chaining() {
        let builder = DB::table("users")
            .where_clause("active", "=", serde_json::json!(true))
            .limit(10);

        assert_eq!(builder.table_name(), "users");
        assert_eq!(builder.limit_val(), Some(10));
    }
}
