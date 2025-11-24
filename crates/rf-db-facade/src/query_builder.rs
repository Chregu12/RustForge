//! Query builder for constructing database queries

use serde_json::Value;

/// Query builder for fluent database queries
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: String,
    wheres: Vec<(String, String, Value)>,
    limit_value: Option<usize>,
    offset_value: Option<usize>,
    order_by: Vec<(String, String)>,
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
        }
    }

    /// Add a where clause
    pub fn where_clause(mut self, column: impl Into<String>, operator: impl Into<String>, value: Value) -> Self {
        self.wheres.push((column.into(), operator.into(), value));
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

    /// Execute the query and get results
    pub async fn get(self) -> Result<Vec<Value>, String> {
        // Mock implementation - returns empty result
        Ok(vec![])
    }

    /// Get the first result
    pub async fn first(self) -> Result<Option<Value>, String> {
        let mut results = self.get().await?;
        Ok(results.pop())
    }

    /// Count the results
    pub async fn count(self) -> Result<usize, String> {
        Ok(0)
    }

    /// Check if any records exist
    pub async fn exists(self) -> Result<bool, String> {
        Ok(false)
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
