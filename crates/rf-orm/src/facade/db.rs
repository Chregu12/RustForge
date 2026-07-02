//! DB facade providing Laravel-style static database API

use crate::facade::db_manager::GLOBAL_DB;
use crate::facade::query_builder::QueryBuilder;
use serde_json::Value;

/// The DB facade providing a static-like API for database operations.
///
/// This is the main entry point for database operations in your application.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_orm::DB;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
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
/// // Use query builder (call .await on terminal methods like .get())
/// let query = DB::table("users")
///     .where_clause("active", "=", true.into());
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
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let users = DB::select("SELECT * FROM users WHERE active = ?",
    ///     &[true.into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn select(query: &str, bindings: &[Value]) -> Result<Vec<Value>, String> {
        let manager = GLOBAL_DB.lock().unwrap();
        manager.select(query, bindings)
    }

    /// Execute an insert query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let id = DB::insert("INSERT INTO users (name, email) VALUES (?, ?)",
    ///     &["John".into(), "john@example.com".into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert(query: &str, bindings: &[Value]) -> Result<u64, String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.insert(query, bindings)
    }

    /// Execute an update query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let affected = DB::update("UPDATE users SET active = ? WHERE id = ?",
    ///     &[true.into(), 1.into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update(query: &str, bindings: &[Value]) -> Result<u64, String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.update(query, bindings)
    }

    /// Execute a delete query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = DB::delete("DELETE FROM users WHERE id = ?",
    ///     &[1.into()])?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete(query: &str, bindings: &[Value]) -> Result<u64, String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.delete(query, bindings)
    }

    /// Execute a statement
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// DB::statement("CREATE TABLE users (id INT, name VARCHAR(255))")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn statement(query: &str) -> Result<bool, String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.statement(query)
    }

    /// Get a query builder for a table
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
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
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// DB::begin_transaction()?;
    /// // ... perform database operations
    /// DB::commit()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn begin_transaction() -> Result<(), String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.begin_transaction()
    }

    /// Commit the current transaction
    pub fn commit() -> Result<(), String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.commit()
    }

    /// Rollback the current transaction
    pub fn rollback() -> Result<(), String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.rollback()
    }

    /// Set the database connection to use
    pub fn connection(name: &str) -> Result<(), String> {
        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.set_connection(name.to_string());
        Ok(())
    }

    /// Get the current connection name
    pub fn connection_name() -> String {
        let manager = GLOBAL_DB.lock().unwrap();
        manager.connection_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_db_facade_real_roundtrip() {
        // The DB facade talks to a single process-global SQLite connection, so this
        // test uses its own dedicated table to stay independent of other tests.
        DB::statement(
            "CREATE TABLE IF NOT EXISTS facade_users (id INTEGER PRIMARY KEY, name TEXT, active INTEGER)",
        )
        .unwrap();
        DB::statement("DELETE FROM facade_users").unwrap();

        let id = DB::insert(
            "INSERT INTO facade_users (name, active) VALUES (?, ?)",
            &[json!("John"), json!(true)],
        )
        .unwrap();
        assert_eq!(id, 1);

        let rows = DB::select("SELECT name FROM facade_users WHERE id = ?", &[json!(1)]).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], json!("John"));

        let updated = DB::update(
            "UPDATE facade_users SET active = ? WHERE id = ?",
            &[json!(false), json!(1)],
        )
        .unwrap();
        assert_eq!(updated, 1);

        let deleted = DB::delete("DELETE FROM facade_users WHERE id = ?", &[json!(1)]).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(DB::select("SELECT id FROM facade_users", &[]).unwrap().len(), 0);
    }

    #[test]
    fn test_db_select_on_missing_table_errors() {
        assert!(DB::select("SELECT * FROM definitely_missing_table", &[]).is_err());
    }

    #[test]
    fn test_db_table_builder_is_pure() {
        let builder = DB::table("users")
            .where_clause("active", "=", json!(true))
            .limit(10);

        assert_eq!(builder.table_name(), "users");
        assert_eq!(builder.limit_val(), Some(10));
    }

    #[test]
    fn test_db_connection_name_default() {
        assert_eq!(DB::connection_name(), "default");
    }
}
