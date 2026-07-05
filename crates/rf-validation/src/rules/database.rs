//! Database validation rules
//!
//! Provides validation rules that query the database to validate data,
//! including uniqueness and existence checks.

use crate::validator::{Rule, RuleResult};
use async_trait::async_trait;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait};
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

// ============================================================================
// ValidatableEntity Trait
// ============================================================================

/// Trait for entities that can be validated against the database
///
/// This trait allows database validation rules to work with concrete entity types
/// while using dynamic dispatch. Entities implement this trait to enable
/// existence and uniqueness checks.
///
/// # Example
///
/// ```ignore
/// impl ValidatableEntity for user::Entity {
///     async fn exists_in_column(
///         db: &DatabaseConnection,
///         column: &str,
///         value: &Value,
///     ) -> Result<bool, DbErr> {
///         let id = value.as_i64().ok_or_else(|| {
///             DbErr::Custom("Invalid value type".to_string())
///         })?;
///
///         let count = Entity::find()
///             .filter(Column::Id.eq(id))
///             .count(db)
///             .await?;
///
///         Ok(count > 0)
///     }
///
///     async fn unique_in_column(
///         db: &DatabaseConnection,
///         column: &str,
///         value: &Value,
///         ignore_id: Option<i64>,
///     ) -> Result<bool, DbErr> {
///         let email = value.as_str().ok_or_else(|| {
///             DbErr::Custom("Invalid value type".to_string())
///         })?;
///
///         let mut query = Entity::find().filter(Column::Email.eq(email));
///
///         if let Some(id) = ignore_id {
///             query = query.filter(Column::Id.ne(id));
///         }
///
///         let count = query.count(db).await?;
///         Ok(count == 0)
///     }
///
///     fn table_name() -> &'static str {
///         "users"
///     }
/// }
/// ```
#[async_trait]
pub trait ValidatableEntity: Send + Sync {
    /// Check if a value exists in the specified column
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `column` - Column name to check
    /// * `value` - Value to look for
    ///
    /// # Returns
    ///
    /// Returns Ok(true) if the value exists, Ok(false) if not found,
    /// or Err if there's a database error
    async fn exists_in_column(
        db: &DatabaseConnection,
        column: &str,
        value: &Value,
    ) -> Result<bool, DbErr>;

    /// Check if a value is unique in the specified column
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    /// * `column` - Column name to check
    /// * `value` - Value to check for uniqueness
    /// * `ignore_id` - Optional ID to ignore (for updates)
    ///
    /// # Returns
    ///
    /// Returns Ok(true) if the value is unique, Ok(false) if duplicate found,
    /// or Err if there's a database error
    async fn unique_in_column(
        db: &DatabaseConnection,
        column: &str,
        value: &Value,
        ignore_id: Option<i64>,
    ) -> Result<bool, DbErr>;

    /// Get the table name for this entity
    fn table_name() -> &'static str;
}

// ============================================================================
// Exists Rule
// ============================================================================

/// Validates that a value exists in a database table
///
/// This rule uses the ValidatableEntity trait to perform actual database queries
/// to verify that a value exists in a specified column.
///
/// # Example
///
/// ```ignore
/// use rf_validation::rules::database::{ExistsRule, ValidatableEntity};
/// use std::sync::Arc;
///
/// // Assuming user::Entity implements ValidatableEntity
/// let exists_rule = ExistsRule::<user::Entity>::new(
///     Arc::new(db),
///     "id"
/// );
///
/// validator.rules(hashmap! {
///     "user_id" => vec![Box::new(exists_rule) as Box<dyn Rule>],
/// });
/// ```
pub struct ExistsRule<E: ValidatableEntity> {
    db: Arc<DatabaseConnection>,
    column: String,
    _phantom: PhantomData<E>,
}

impl<E: ValidatableEntity> ExistsRule<E> {
    /// Create a new exists rule
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection wrapped in Arc
    /// * `column` - Column name to check for existence
    pub fn new(db: Arc<DatabaseConnection>, column: impl Into<String>) -> Self {
        Self {
            db,
            column: column.into(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<E: ValidatableEntity + 'static> Rule for ExistsRule<E> {
    fn name(&self) -> &str {
        "exists"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        // Use ValidatableEntity trait to check existence
        match E::exists_in_column(&self.db, &self.column, value).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(self.message()),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    fn message(&self) -> String {
        format!(
            "The selected value does not exist in {}.{}",
            E::table_name(),
            self.column
        )
    }
}

// ============================================================================
// Unique Rule
// ============================================================================

/// Validates that a value is unique in a database table
///
/// This rule uses the ValidatableEntity trait to perform actual database queries
/// to verify that a value is unique in a specified column.
/// Optionally ignores a specific ID (useful for updates).
///
/// # Example
///
/// ```ignore
/// use rf_validation::rules::database::{UniqueRule, ValidatableEntity};
/// use std::sync::Arc;
///
/// // Assuming user::Entity implements ValidatableEntity
/// let unique_rule = UniqueRule::<user::Entity>::new(
///     Arc::new(db),
///     "email"
/// ).except(user_id); // Ignore current user ID when updating
///
/// validator.rules(hashmap! {
///     "email" => vec![Box::new(unique_rule) as Box<dyn Rule>],
/// });
/// ```
pub struct UniqueRule<E: ValidatableEntity> {
    db: Arc<DatabaseConnection>,
    column: String,
    ignore_id: Option<i64>,
    _phantom: PhantomData<E>,
}

impl<E: ValidatableEntity> UniqueRule<E> {
    /// Create a new unique rule
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection wrapped in Arc
    /// * `column` - Column name to check for uniqueness
    pub fn new(db: Arc<DatabaseConnection>, column: impl Into<String>) -> Self {
        Self {
            db,
            column: column.into(),
            ignore_id: None,
            _phantom: PhantomData,
        }
    }

    /// Exclude a specific ID from the uniqueness check (useful for updates)
    ///
    /// # Arguments
    ///
    /// * `id` - ID to ignore during uniqueness check
    pub fn except(mut self, id: i64) -> Self {
        self.ignore_id = Some(id);
        self
    }
}

#[async_trait]
impl<E: ValidatableEntity + 'static> Rule for UniqueRule<E> {
    fn name(&self) -> &str {
        "unique"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        // Use ValidatableEntity trait to check uniqueness
        match E::unique_in_column(&self.db, &self.column, value, self.ignore_id).await {
            Ok(true) => Ok(()),               // Value is unique
            Ok(false) => Err(self.message()), // Value is not unique
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    fn message(&self) -> String {
        format!(
            "The {} has already been taken",
            self.column.replace('_', " ")
        )
    }
}

// ============================================================================
// Simple String-based Database Rules (for dynamic use)
// ============================================================================

/// Validate that a SQL identifier (table/column name) contains only safe characters.
/// Prevents SQL injection through identifier names.
fn validate_sql_identifier(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("SQL identifier cannot be empty".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!(
            "SQL identifier '{}' contains invalid characters",
            name
        ));
    }
    Ok(())
}

/// Simple exists rule using raw SQL queries
///
/// This is a simplified version that can work without concrete entity types,
/// useful for dynamic validation scenarios.
///
/// # Example
///
/// ```ignore
/// let exists_rule = SimpleExistsRule::new(
///     db,
///     "users",
///     "id"
/// );
/// ```
pub struct SimpleExistsRule {
    db: DatabaseConnection,
    table: String,
    column: String,
}

impl SimpleExistsRule {
    pub fn new(
        db: DatabaseConnection,
        table: impl Into<String>,
        column: impl Into<String>,
    ) -> Self {
        Self {
            db,
            table: table.into(),
            column: column.into(),
        }
    }
}

#[async_trait]
impl Rule for SimpleExistsRule {
    fn name(&self) -> &str {
        "exists"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        // Validate identifiers to prevent SQL injection
        validate_sql_identifier(&self.table)?;
        validate_sql_identifier(&self.column)?;

        // Extract value and build query
        use sea_orm::{DbBackend, Statement, TryGetable};

        // Detect database backend
        let backend = self.db.get_database_backend();
        let placeholder = match backend {
            DbBackend::Postgres => "$1",
            DbBackend::MySql => "?",
            DbBackend::Sqlite => "?",
        };

        let (query, value_param) = match value {
            Value::String(s) => (
                format!(
                    "SELECT COUNT(*) as count FROM {} WHERE {} = {}",
                    self.table, self.column, placeholder
                ),
                sea_orm::Value::from(s.clone()),
            ),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    (
                        format!(
                            "SELECT COUNT(*) as count FROM {} WHERE {} = {}",
                            self.table, self.column, placeholder
                        ),
                        sea_orm::Value::from(i),
                    )
                } else if let Some(f) = n.as_f64() {
                    (
                        format!(
                            "SELECT COUNT(*) as count FROM {} WHERE {} = {}",
                            self.table, self.column, placeholder
                        ),
                        sea_orm::Value::from(f),
                    )
                } else {
                    return Err("Invalid number format".to_string());
                }
            }
            _ => return Err("Value must be a string or number".to_string()),
        };

        // Execute query
        let stmt = Statement::from_sql_and_values(backend, &query, vec![value_param]);

        match self.db.query_one(stmt).await {
            Ok(Some(result)) => {
                let count: i64 = result
                    .try_get("", "count")
                    .map_err(|e| format!("Database error: {}", e))?;

                if count == 0 {
                    Err(self.message())
                } else {
                    Ok(())
                }
            }
            Ok(None) => Err(self.message()),
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    fn message(&self) -> String {
        format!(
            "The selected value does not exist in {}.{}",
            self.table, self.column
        )
    }
}

/// Simple unique rule using raw SQL queries
///
/// # Example
///
/// ```ignore
/// let unique_rule = SimpleUniqueRule::new(
///     db,
///     "users",
///     "email",
///     Some(5), // Ignore user with ID 5
/// );
/// ```
pub struct SimpleUniqueRule {
    db: DatabaseConnection,
    table: String,
    column: String,
    ignore_id: Option<i64>,
    id_column: String,
}

impl SimpleUniqueRule {
    pub fn new(
        db: DatabaseConnection,
        table: impl Into<String>,
        column: impl Into<String>,
    ) -> Self {
        Self {
            db,
            table: table.into(),
            column: column.into(),
            ignore_id: None,
            id_column: "id".to_string(),
        }
    }

    /// Exclude a specific ID from the uniqueness check (useful for updates)
    pub fn except(mut self, id: i64) -> Self {
        self.ignore_id = Some(id);
        self
    }

    /// Specify a custom ID column name (default is "id")
    pub fn with_id_column(mut self, id_column: impl Into<String>) -> Self {
        self.id_column = id_column.into();
        self
    }
}

#[async_trait]
impl Rule for SimpleUniqueRule {
    fn name(&self) -> &str {
        "unique"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }

        // Validate identifiers to prevent SQL injection
        validate_sql_identifier(&self.table)?;
        validate_sql_identifier(&self.column)?;
        validate_sql_identifier(&self.id_column)?;

        // Extract value and build query
        use sea_orm::{DbBackend, Statement, TryGetable};

        // Detect database backend
        let backend = self.db.get_database_backend();
        let (placeholder1, placeholder2) = match backend {
            DbBackend::Postgres => ("$1", "$2"),
            DbBackend::MySql => ("?", "?"),
            DbBackend::Sqlite => ("?", "?"),
        };

        let (query, mut values) = match value {
            Value::String(s) => {
                let mut q = format!(
                    "SELECT COUNT(*) as count FROM {} WHERE {} = {}",
                    self.table, self.column, placeholder1
                );
                let mut vals = vec![sea_orm::Value::from(s.clone())];

                // Add exception for updates
                if let Some(id) = self.ignore_id {
                    q.push_str(&format!(" AND {} != {}", self.id_column, placeholder2));
                    vals.push(sea_orm::Value::from(id));
                }

                (q, vals)
            }
            Value::Number(n) => {
                let mut q = format!(
                    "SELECT COUNT(*) as count FROM {} WHERE {} = {}",
                    self.table, self.column, placeholder1
                );

                let value_param = if let Some(i) = n.as_i64() {
                    sea_orm::Value::from(i)
                } else if let Some(f) = n.as_f64() {
                    sea_orm::Value::from(f)
                } else {
                    return Err("Invalid number format".to_string());
                };

                let mut vals = vec![value_param];

                // Add exception for updates
                if let Some(id) = self.ignore_id {
                    q.push_str(&format!(" AND {} != {}", self.id_column, placeholder2));
                    vals.push(sea_orm::Value::from(id));
                }

                (q, vals)
            }
            _ => return Err("Value must be a string or number".to_string()),
        };

        // Execute query
        let stmt = Statement::from_sql_and_values(backend, &query, values);

        match self.db.query_one(stmt).await {
            Ok(Some(result)) => {
                let count: i64 = result
                    .try_get("", "count")
                    .map_err(|e| format!("Database error: {}", e))?;

                if count > 0 {
                    Err(self.message())
                } else {
                    Ok(())
                }
            }
            Ok(None) => Ok(()), // No records found, value is unique
            Err(e) => Err(format!("Database error: {}", e)),
        }
    }

    fn message(&self) -> String {
        format!(
            "The {} has already been taken",
            self.column.replace('_', " ")
        )
    }
}

// ============================================================================
// Facade-backed Database Rules (used by the `validate!` DSL)
// ============================================================================
//
// The `SimpleExistsRule`/`SimpleUniqueRule` above require a caller-provided
// `sea_orm::DatabaseConnection`. The `validate!` macro has no such handle, so
// the rules below run their `COUNT(*)` query through the process-global
// `rf_orm::DB` facade (the real rusqlite-backed connection). The DB facade is
// sync, so these rules simply call it inline from within `validate` — no extra
// runtime blocking machinery is needed.

/// Run `SELECT COUNT(*) ... WHERE <column> = ?` through the `rf_orm::DB` facade
/// and return the matched row count. Identifiers are validated up-front to
/// prevent SQL injection through the table/column names.
fn facade_count(table: &str, column: &str, value: &Value) -> Result<i64, String> {
    validate_sql_identifier(table)?;
    validate_sql_identifier(column)?;

    let query = format!(
        "SELECT COUNT(*) AS rf_count FROM {} WHERE {} = ?",
        table, column
    );
    let rows = rf_orm::DB::select(&query, std::slice::from_ref(value))
        .map_err(|e| format!("Database error: {}", e))?;

    let count = rows
        .first()
        .and_then(|row| row.get("rf_count"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Ok(count)
}

/// Validates that a field's value is UNIQUE in `<table>.<column>` by running a
/// real `COUNT(*)` against the `rf_orm::DB` facade. Passes when zero rows match.
///
/// Wired into the `validate!` DSL as `email: email.unique("users", "email")`.
pub struct DbUniqueRule {
    table: String,
    column: String,
}

impl DbUniqueRule {
    pub fn new(table: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            column: column.into(),
        }
    }
}

#[async_trait]
impl Rule for DbUniqueRule {
    fn name(&self) -> &str {
        "unique"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }
        let count = facade_count(&self.table, &self.column, value)?;
        if count == 0 {
            Ok(())
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        format!("The {} has already been taken", self.column.replace('_', " "))
    }
}

/// Validates that a field's value EXISTS in `<table>.<column>` by running a real
/// `COUNT(*)` against the `rf_orm::DB` facade. Passes when at least one row matches.
///
/// Wired into the `validate!` DSL as `user_id: int.exists("users", "id")`.
pub struct DbExistsRule {
    table: String,
    column: String,
}

impl DbExistsRule {
    pub fn new(table: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            column: column.into(),
        }
    }
}

#[async_trait]
impl Rule for DbExistsRule {
    fn name(&self) -> &str {
        "exists"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        if value.is_null() {
            return Ok(());
        }
        let count = facade_count(&self.table, &self.column, value)?;
        if count >= 1 {
            Ok(())
        } else {
            Err(self.message())
        }
    }

    fn message(&self) -> String {
        format!(
            "The selected value does not exist in {}.{}",
            self.table, self.column
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: The entity-based rules require actual database connections and
    // entities, exercised via integration tests / sandbox probes.

    #[tokio::test]
    async fn test_simple_exists_rule_placeholder() {
        // This is a placeholder test
        // In practice, you would set up a test database and verify the behavior
        assert!(true);
    }

    #[tokio::test]
    async fn test_simple_unique_rule_placeholder() {
        // This is a placeholder test
        // In practice, you would set up a test database and verify the behavior
        assert!(true);
    }

    // Facade-backed rules run against the real process-global rf_orm::DB
    // (rusqlite). One dedicated table keeps this independent of other tests.
    #[tokio::test]
    async fn test_facade_unique_and_exists_rules() {
        rf_orm::DB::statement(
            "CREATE TABLE IF NOT EXISTS rf_val_users (id INTEGER PRIMARY KEY, email TEXT)",
        )
        .unwrap();
        rf_orm::DB::statement("DELETE FROM rf_val_users").unwrap();
        rf_orm::DB::insert(
            "INSERT INTO rf_val_users (id, email) VALUES (?, ?)",
            &[serde_json::json!(1), serde_json::json!("taken@example.com")],
        )
        .unwrap();

        let data = HashMap::new();
        let unique = DbUniqueRule::new("rf_val_users", "email");
        // Present value fails unique, absent value passes.
        assert!(unique
            .validate(&serde_json::json!("taken@example.com"), &data)
            .await
            .is_err());
        assert!(unique
            .validate(&serde_json::json!("fresh@example.com"), &data)
            .await
            .is_ok());

        let exists = DbExistsRule::new("rf_val_users", "id");
        // Present id passes exists, absent id fails.
        assert!(exists.validate(&serde_json::json!(1), &data).await.is_ok());
        assert!(exists
            .validate(&serde_json::json!(999), &data)
            .await
            .is_err());
    }
}
