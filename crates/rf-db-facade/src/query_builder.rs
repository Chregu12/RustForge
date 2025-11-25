//! Query builder for constructing database queries
//!
//! Provides a Laravel-style fluent query builder for database operations.

use serde_json::Value;

/// Query builder for fluent database queries
///
/// # Examples
///
/// ```rust,no_run
/// use rf_db_facade::DB;
///
/// # async fn example() -> Result<(), String> {
/// // Select with conditions
/// let users = DB::table("users")
///     .where_clause("active", "=", true.into())
///     .order_by("name", "asc")
///     .limit(10)
///     .get().await?;
///
/// // Insert
/// let id = DB::table("users").insert(json!({
///     "name": "John",
///     "email": "john@example.com"
/// })).await?;
///
/// // Update
/// DB::table("users")
///     .where_clause("id", "=", 1.into())
///     .update(json!({"active": true})).await?;
///
/// // Delete
/// DB::table("users")
///     .where_clause("id", "=", 1.into())
///     .delete().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: String,
    wheres: Vec<(String, String, Value)>,
    limit_value: Option<usize>,
    offset_value: Option<usize>,
    order_by: Vec<(String, String)>,
    select_columns: Vec<String>,
}

impl QueryBuilder {
    /// Create a new query builder for a table
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            wheres: Vec::new(),
            limit_value: None,
            offset_value: None,
            order_by: Vec::new(),
            select_columns: Vec::new(),
        }
    }

    /// Select specific columns
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .select(&["id", "name", "email"])
    ///     .get().await?;
    /// ```
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select_columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a where clause - Laravel-style!
    ///
    /// With 2 arguments: `where("column", value)` means `column = value`
    /// With 3 arguments: `where_op("column", ">=", value)`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Simple equality (like Laravel!)
    /// let users = DB::table("users")
    ///     .r#where("active", true)
    ///     .r#where("role", "admin")
    ///     .get().await?;
    /// ```
    pub fn r#where<V: Into<Value>>(mut self, column: impl Into<String>, value: V) -> Self {
        self.wheres.push((column.into(), "=".to_string(), value.into()));
        self
    }

    /// Alias for `r#where` - more readable without the r# prefix
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Clean syntax without r# prefix!
    /// let users = DB::table("users")
    ///     .filter("active", true)
    ///     .filter("role", "admin")
    ///     .get().await?;
    /// ```
    pub fn filter<V: Into<Value>>(self, column: impl Into<String>, value: V) -> Self {
        self.r#where(column, value)
    }

    /// Where with custom operator
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .where_op("age", ">=", 18)
    ///     .where_op("score", "<", 100)
    ///     .get().await?;
    /// ```
    pub fn where_op<V: Into<Value>>(mut self, column: impl Into<String>, operator: impl Into<String>, value: V) -> Self {
        self.wheres.push((column.into(), operator.into(), value.into()));
        self
    }

    /// Legacy method - use `r#where` instead
    #[deprecated(note = "Use `r#where` for Laravel-style syntax")]
    pub fn where_clause(mut self, column: impl Into<String>, operator: impl Into<String>, value: Value) -> Self {
        self.wheres.push((column.into(), operator.into(), value));
        self
    }

    /// Shorthand for where equals (same as `r#where`)
    pub fn where_eq<V: Into<Value>>(self, column: impl Into<String>, value: V) -> Self {
        self.r#where(column, value)
    }

    /// Where column is null
    pub fn where_null(mut self, column: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "IS".to_string(), Value::Null));
        self
    }

    /// Where column is not null
    pub fn where_not_null(mut self, column: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "IS NOT".to_string(), Value::Null));
        self
    }

    /// Where column is in a list of values
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .where_in("id", vec![1, 2, 3])
    ///     .get().await?;
    /// ```
    pub fn where_in<V: Into<Value>>(mut self, column: impl Into<String>, values: Vec<V>) -> Self {
        let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
        self.wheres.push((column.into(), "IN".to_string(), Value::Array(values)));
        self
    }

    /// Where column is not in a list
    pub fn where_not_in<V: Into<Value>>(mut self, column: impl Into<String>, values: Vec<V>) -> Self {
        let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
        self.wheres.push((column.into(), "NOT IN".to_string(), Value::Array(values)));
        self
    }

    /// Where column is between two values
    pub fn where_between<V: Into<Value>>(mut self, column: impl Into<String>, min: V, max: V) -> Self {
        let col = column.into();
        self.wheres.push((col.clone(), ">=".to_string(), min.into()));
        self.wheres.push((col, "<=".to_string(), max.into()));
        self
    }

    /// Where column is like a pattern
    pub fn where_like(mut self, column: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "LIKE".to_string(), Value::String(pattern.into())));
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Add order by clause
    pub fn order_by(mut self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by.push((column.into(), direction.into()));
        self
    }

    /// Laravel-style orderBy (camelCase alias)
    #[allow(non_snake_case)]
    pub fn orderBy(self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by(column, direction)
    }

    /// Order by ascending
    pub fn order_by_asc(self, column: impl Into<String>) -> Self {
        self.order_by(column, "ASC")
    }

    /// Order by descending
    pub fn order_by_desc(self, column: impl Into<String>) -> Self {
        self.order_by(column, "DESC")
    }

    /// Execute the query and get all results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .where_clause("active", "=", true.into())
    ///     .get().await?;
    /// ```
    pub async fn get(self) -> Result<Vec<Value>, String> {
        // Mock implementation - in production this executes against DB
        Ok(vec![])
    }

    /// Get the first result
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = DB::table("users")
    ///     .where_clause("id", "=", 1.into())
    ///     .first().await?;
    /// ```
    pub async fn first(self) -> Result<Option<Value>, String> {
        let results = self.limit(1).get().await?;
        Ok(results.into_iter().next())
    }

    /// Find a record by ID - Laravel-style!
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = DB::table("users").find(1).await?;
    /// let post = DB::table("posts").find(42).await?;
    /// ```
    pub async fn find<V: Into<Value>>(self, id: V) -> Result<Option<Value>, String> {
        self.r#where("id", id).first().await
    }

    /// Insert a new record and return the ID
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let id = DB::table("users").insert(json!({
    ///     "name": "John",
    ///     "email": "john@example.com"
    /// })).await?;
    /// ```
    pub async fn insert(self, _data: Value) -> Result<u64, String> {
        // Mock implementation - returns fake ID
        Ok(1)
    }

    /// Create a record and return it - Laravel-style!
    ///
    /// This is the preferred method for creating records.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Just like Laravel's User::create()!
    /// let user = DB::table("users").create(json!({
    ///     "name": "John",
    ///     "email": "john@example.com"
    /// })).await?;
    ///
    /// println!("Created user: {}", user["name"]);
    /// ```
    pub async fn create(self, data: Value) -> Result<Value, String> {
        let table = self.table.clone();
        let id = self.insert(data.clone()).await?;

        // Return the created record with ID
        let mut result = data;
        if let Value::Object(ref mut map) = result {
            map.insert("id".to_string(), Value::Number(id.into()));
        }
        Ok(result)
    }

    /// Insert multiple records
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// DB::table("users").insert_many(vec![
    ///     json!({"name": "John"}),
    ///     json!({"name": "Jane"}),
    /// ]).await?;
    /// ```
    pub async fn insert_many(self, data: Vec<Value>) -> Result<u64, String> {
        Ok(data.len() as u64)
    }

    /// Update records matching the where clauses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let affected = DB::table("users")
    ///     .where_clause("id", "=", 1.into())
    ///     .update(json!({"active": true})).await?;
    /// ```
    pub async fn update(self, data: Value) -> Result<u64, String> {
        // Mock implementation
        Ok(1)
    }

    /// Delete records matching the where clauses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let deleted = DB::table("users")
    ///     .where_clause("id", "=", 1.into())
    ///     .delete().await?;
    /// ```
    pub async fn delete(self) -> Result<u64, String> {
        // Mock implementation
        Ok(1)
    }

    /// Count the results
    pub async fn count(self) -> Result<usize, String> {
        Ok(0)
    }

    /// Check if any records exist
    pub async fn exists(self) -> Result<bool, String> {
        let count = self.count().await?;
        Ok(count > 0)
    }

    /// Paginate results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let page = DB::table("users")
    ///     .where_clause("active", "=", true.into())
    ///     .paginate(15, 1).await?;
    /// ```
    pub async fn paginate(self, per_page: usize, page: usize) -> Result<PaginatedResult, String> {
        let offset = (page.saturating_sub(1)) * per_page;
        let data = self.clone().limit(per_page).offset(offset).get().await?;
        let total = self.count().await?;

        Ok(PaginatedResult {
            data,
            total,
            per_page,
            current_page: page,
            last_page: (total + per_page - 1) / per_page,
        })
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Get the where clauses
    pub fn where_clauses(&self) -> &[(String, String, Value)] {
        &self.wheres
    }

    /// Get the limit value
    pub fn limit_val(&self) -> Option<usize> {
        self.limit_value
    }
}

/// Paginated result set
#[derive(Debug, Clone)]
pub struct PaginatedResult {
    pub data: Vec<Value>,
    pub total: usize,
    pub per_page: usize,
    pub current_page: usize,
    pub last_page: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_new() {
        let builder = QueryBuilder::new("users");
        assert_eq!(builder.table_name(), "users");
        assert_eq!(builder.where_clauses().len(), 0);
    }

    #[test]
    fn test_query_builder_where() {
        let builder = QueryBuilder::new("users")
            .where_clause("active", "=", serde_json::json!(true));

        assert_eq!(builder.where_clauses().len(), 1);
        assert_eq!(builder.where_clauses()[0].0, "active");
        assert_eq!(builder.where_clauses()[0].1, "=");
    }

    #[test]
    fn test_query_builder_limit() {
        let builder = QueryBuilder::new("users")
            .limit(10);

        assert_eq!(builder.limit_val(), Some(10));
    }

    #[test]
    fn test_query_builder_chaining() {
        let builder = QueryBuilder::new("users")
            .where_clause("active", "=", serde_json::json!(true))
            .where_clause("verified", "=", serde_json::json!(true))
            .limit(10)
            .offset(5)
            .order_by("created_at", "desc");

        assert_eq!(builder.where_clauses().len(), 2);
        assert_eq!(builder.limit_val(), Some(10));
    }

    #[tokio::test]
    async fn test_query_builder_get() {
        let builder = QueryBuilder::new("users");
        let result = builder.get().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_builder_count() {
        let builder = QueryBuilder::new("users");
        let count = builder.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_query_builder_exists() {
        let builder = QueryBuilder::new("users");
        let exists = builder.exists().await.unwrap();
        assert!(!exists);
    }
}
