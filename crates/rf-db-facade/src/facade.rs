//! DB facade providing Laravel-style static database API

use crate::manager::GLOBAL_DB;
use crate::query_builder::QueryBuilder;
use rf_orm::{DbError, DbResult};
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
/// let users = DB::select("SELECT * FROM users", &[])?;
///
/// // Insert a record
/// let id = DB::insert("INSERT INTO users (name) VALUES (?)",
///     &["John".into()])?;
///
/// // Update records
/// let affected = DB::update("UPDATE users SET active = ?",
///     &[true.into()])?;
///
/// // Delete records
/// let deleted = DB::delete("DELETE FROM users WHERE id = ?",
///     &[id.into()])?;
///
/// // Use query builder
/// let users = DB::table("users")
///     .where_clause("active", "=", true.into())
///     .get()
///     .await?;
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
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let users = DB::select("SELECT * FROM users WHERE active = ?",
    ///     &[true.into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn select(query: &str, bindings: &[Value]) -> DbResult<Vec<Value>> {
        let manager = GLOBAL_DB.read().unwrap();
        manager.select(query, bindings).map_err(DbError::Other)
    }

    /// Execute an insert query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let id = DB::insert("INSERT INTO users (name, email) VALUES (?, ?)",
    ///     &["John".into(), "john@example.com".into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert(query: &str, bindings: &[Value]) -> DbResult<u64> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.insert(query, bindings).map_err(DbError::Other)
    }

    /// Execute an update query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let affected = DB::update("UPDATE users SET active = ? WHERE id = ?",
    ///     &[true.into(), 1.into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update(query: &str, bindings: &[Value]) -> DbResult<u64> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.update(query, bindings).map_err(DbError::Other)
    }

    /// Execute a delete query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = DB::delete("DELETE FROM users WHERE id = ?",
    ///     &[1.into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete(query: &str, bindings: &[Value]) -> DbResult<u64> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.delete(query, bindings).map_err(DbError::Other)
    }

    /// Execute a statement
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_db_facade::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// DB::statement("CREATE TABLE users (id INT, name VARCHAR(255))")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn statement(query: &str) -> DbResult<bool> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.statement(query).map_err(DbError::Other)
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
    ///     .get()
    ///     .await?;
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
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// DB::begin_transaction()?;
    /// // ... perform database operations
    /// DB::commit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn begin_transaction() -> DbResult<()> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.begin_transaction().map_err(DbError::Other)
    }

    /// Commit the current transaction
    pub fn commit() -> DbResult<()> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.commit().map_err(DbError::Other)
    }

    /// Rollback the current transaction
    pub fn rollback() -> DbResult<()> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.rollback().map_err(DbError::Other)
    }

    /// Set the database connection to use
    pub fn connection(name: &str) -> DbResult<()> {
        let mut manager = GLOBAL_DB.write().unwrap();
        manager.set_connection(name.to_string());
        Ok(())
    }

    /// Get the current connection name
    pub fn connection_name() -> String {
        let manager = GLOBAL_DB.read().unwrap();
        manager.connection_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_insert() {
        let bindings = vec![
            serde_json::json!("John"),
            serde_json::json!("john@example.com")
        ];

        let result = DB::insert("INSERT INTO users (name, email) VALUES (?, ?)", &bindings);
        assert!(result.is_ok());
    }

    #[test]
    fn test_db_select() {
        let result = DB::select("SELECT * FROM users", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_db_update() {
        let bindings = vec![serde_json::json!(true)];
        let result = DB::update("UPDATE users SET active = ?", &bindings);
        assert!(result.is_ok());
    }

    #[test]
    fn test_db_delete() {
        let bindings = vec![serde_json::json!(1)];
        let result = DB::delete("DELETE FROM users WHERE id = ?", &bindings);
        assert!(result.is_ok());
    }

    #[test]
    fn test_db_statement() {
        let result = DB::statement("CREATE TABLE users (id INT)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_db_table() {
        let builder = DB::table("users");
        assert_eq!(builder.table_name(), "users");
    }

    #[test]
    fn test_db_transaction() {
        assert!(DB::begin_transaction().is_ok());
        assert!(DB::commit().is_ok());
        assert!(DB::rollback().is_ok());
    }

    #[test]
    fn test_db_connection() {
        assert!(DB::connection("mysql").is_ok());
        let name = DB::connection_name();
        assert_eq!(name, "mysql");
    }

    #[test]
    fn test_db_query_builder_chaining() {
        let builder = DB::table("users")
            .where_clause("active", "=", serde_json::json!(true))
            .limit(10);

        assert_eq!(builder.table_name(), "users");
        assert_eq!(builder.limit_val(), Some(10));
    }
}
