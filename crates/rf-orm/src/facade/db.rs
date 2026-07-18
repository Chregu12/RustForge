//! DB facade providing Laravel-style static database API

use crate::facade::db_manager::{
    global_begin_transaction, global_commit, global_connection_name, global_delete,
    global_insert, global_refresh, global_rollback, global_select, global_set_connection,
    global_statement, global_update,
};
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
        global_select(query, bindings)
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
        global_insert(query, bindings)
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
        global_update(query, bindings)
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
        global_delete(query, bindings)
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
        global_statement(query)
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

    /// Reset the database to an empty schema for per-test isolation.
    ///
    /// The `DB` facade is backed by a single process-global SQLite connection, so
    /// tables and rows created by one test stay visible to the next. This is the
    /// RustForge equivalent of Laravel's `RefreshDatabase`: call it at the top of
    /// a `#[tokio::test]` so the test starts from a guaranteed-empty database.
    ///
    /// # Isolation guarantee
    ///
    /// This performs a **full schema reset** (table-clear), not a transaction
    /// rollback: every user table is `DROP`ped, so both its rows and its
    /// definition are gone. The test is then responsible for (re)creating the
    /// tables it needs. Running it on an already-empty database is a no-op.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // At the top of a test: start from a clean database.
    /// DB::refresh()?;
    /// DB::statement("CREATE TABLE tasks (id INTEGER PRIMARY KEY, title TEXT)")?;
    /// // ... the test now sees only its own rows.
    /// # Ok(())
    /// # }
    /// ```
    pub fn refresh() -> Result<(), String> {
        global_refresh()
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
        global_begin_transaction()
    }

    /// Commit the current transaction
    pub fn commit() -> Result<(), String> {
        global_commit()
    }

    /// Rollback the current transaction
    pub fn rollback() -> Result<(), String> {
        global_rollback()
    }

    /// Set the database connection to use
    pub fn connection(name: &str) -> Result<(), String> {
        global_set_connection(name.to_string());
        Ok(())
    }

    /// Get the current connection name
    pub fn connection_name() -> String {
        global_connection_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Keep GLOBAL_DB in scope so the type is still reachable from tests that
    // reference it (e.g. to confirm it compiles as RwLock<ConcurrentDB>).
    #[allow(unused_imports)]
    use crate::facade::db_manager::GLOBAL_DB;

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
