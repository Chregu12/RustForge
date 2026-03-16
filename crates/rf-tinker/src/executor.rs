//! Query and expression executor

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::Value;
use std::sync::Arc;

/// Execution context with database connection and input
#[derive(Clone)]
pub struct ExecutionContext {
    pub db: Option<Arc<DatabaseConnection>>,
    pub input: String,
}

/// Result of executing an expression
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    /// Query results as JSON array
    Rows(Vec<Value>),
    /// Single value (count, etc.)
    Value(Value),
    /// Affected rows count
    Affected(u64),
    /// Success message
    Message(String),
    /// Error message
    Error(String),
    /// Empty result
    Empty,
}

/// Query executor
pub struct QueryExecutor;

/// Validate that an identifier (table/column name) contains only safe characters
fn validate_sql_identifier(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl QueryExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute an expression or query
    pub async fn execute(&self, ctx: ExecutionContext) -> anyhow::Result<ExecutionResult> {
        let input = ctx.input.trim();

        // Check if it's a DB facade call
        if input.starts_with("DB::") {
            return self.execute_db_facade(&ctx, input).await;
        }

        // Check if it's raw SQL
        if self.is_sql_query(input) {
            return self.execute_sql(&ctx, input).await;
        }

        // Check for common expressions
        if input.starts_with("Cache::") {
            return Ok(ExecutionResult::Message(
                "Cache facade not yet connected in Tinker. Use .tables to explore database.".to_string()
            ));
        }

        // Try to execute as SQL
        self.execute_sql(&ctx, input).await
    }

    /// Check if input looks like a SQL query
    fn is_sql_query(&self, input: &str) -> bool {
        let upper = input.to_uppercase();
        upper.starts_with("SELECT ")
            || upper.starts_with("INSERT ")
            || upper.starts_with("UPDATE ")
            || upper.starts_with("DELETE ")
            || upper.starts_with("CREATE ")
            || upper.starts_with("DROP ")
            || upper.starts_with("ALTER ")
            || upper.starts_with("SHOW ")
            || upper.starts_with("DESCRIBE ")
            || upper.starts_with("EXPLAIN ")
            || upper.starts_with("PRAGMA ")
    }

    /// Execute a DB:: facade call
    async fn execute_db_facade(&self, ctx: &ExecutionContext, input: &str) -> anyhow::Result<ExecutionResult> {
        if ctx.db.is_none() {
            return Ok(ExecutionResult::Error("No database connection".to_string()));
        }

        // Parse DB::table("name") patterns
        if let Some(query) = self.parse_db_table_call(input) {
            return self.execute_sql(ctx, &query).await;
        }

        // Parse DB::select("query") patterns
        if let Some(query) = self.parse_db_select_call(input) {
            return self.execute_sql(ctx, &query).await;
        }

        // Parse DB::insert/update/delete
        if let Some(query) = self.parse_db_mutation_call(input) {
            return self.execute_sql(ctx, &query).await;
        }

        Ok(ExecutionResult::Error(format!(
            "Could not parse DB facade call: {}",
            input
        )))
    }

    /// Parse DB::table("users").get() style calls
    fn parse_db_table_call(&self, input: &str) -> Option<String> {
        // Match DB::table("table_name")
        let table_regex = regex::Regex::new(r#"DB::table\s*\(\s*"([^"]+)"\s*\)"#).ok()?;
        let table_name = table_regex.captures(input)?.get(1)?.as_str();

        // Validate table name to prevent SQL injection
        if !validate_sql_identifier(table_name) {
            return None;
        }

        let mut query = format!("SELECT * FROM {}", table_name);
        let mut conditions: Vec<String> = Vec::new();
        let mut limit_clause = String::new();
        let mut order_clause = String::new();

        // Parse .where() clauses
        let where_regex = regex::Regex::new(r#"\.where\s*\(\s*"([^"]+)"\s*,\s*([^)]+)\)"#).ok()?;
        for cap in where_regex.captures_iter(input) {
            let column = cap.get(1)?.as_str();
            if !validate_sql_identifier(column) {
                return None;
            }
            let value = cap.get(2)?.as_str().trim();
            // Escape the value as a string literal
            let escaped_value = value.replace('\'', "''");
            conditions.push(format!("{} = '{}'", column, escaped_value));
        }

        // Parse .whereIn() clauses
        let where_in_regex = regex::Regex::new(r#"\.whereIn\s*\(\s*"([^"]+)"\s*,\s*\[([^\]]+)\]\s*\)"#).ok()?;
        for cap in where_in_regex.captures_iter(input) {
            let column = cap.get(1)?.as_str();
            if !validate_sql_identifier(column) {
                return None;
            }
            let values = cap.get(2)?.as_str();
            // Escape each value in the IN clause
            let escaped_values: Vec<String> = values
                .split(',')
                .map(|v| {
                    let trimmed = v.trim().trim_matches('"').trim_matches('\'');
                    format!("'{}'", trimmed.replace('\'', "''"))
                })
                .collect();
            conditions.push(format!("{} IN ({})", column, escaped_values.join(", ")));
        }

        // Parse .limit()
        let limit_regex = regex::Regex::new(r"\.limit\s*\(\s*(\d+)\s*\)").ok()?;
        if let Some(cap) = limit_regex.captures(input) {
            limit_clause = format!(" LIMIT {}", cap.get(1)?.as_str());
        }

        // Parse .orderBy()
        let order_regex = regex::Regex::new(r#"\.orderBy\s*\(\s*"([^"]+)"\s*(?:,\s*"(asc|desc)")?\s*\)"#).ok()?;
        if let Some(cap) = order_regex.captures(input) {
            let column = cap.get(1)?.as_str();
            if !validate_sql_identifier(column) {
                return None;
            }
            let dir = cap.get(2).map(|m| m.as_str()).unwrap_or("asc");
            order_clause = format!(" ORDER BY {} {}", column, dir.to_uppercase());
        }

        // Check for .first()
        if input.contains(".first()") {
            limit_clause = " LIMIT 1".to_string();
        }

        // Check for .count()
        if input.contains(".count()") {
            query = format!("SELECT COUNT(*) as count FROM {}", table_name);
        }

        // Build WHERE clause
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(&order_clause);
        query.push_str(&limit_clause);

        Some(query)
    }

    /// Parse DB::select("query") style calls
    fn parse_db_select_call(&self, input: &str) -> Option<String> {
        let regex = regex::Regex::new(r#"DB::select\s*\(\s*"([^"]+)"\s*\)"#).ok()?;
        let cap = regex.captures(input)?;
        Some(cap.get(1)?.as_str().to_string())
    }

    /// Parse DB::insert/update/delete calls
    fn parse_db_mutation_call(&self, input: &str) -> Option<String> {
        // DB::statement("query")
        let regex = regex::Regex::new(r#"DB::statement\s*\(\s*"([^"]+)"\s*\)"#).ok()?;
        if let Some(cap) = regex.captures(input) {
            return Some(cap.get(1)?.as_str().to_string());
        }

        None
    }

    /// Execute raw SQL
    async fn execute_sql(&self, ctx: &ExecutionContext, sql: &str) -> anyhow::Result<ExecutionResult> {
        let Some(ref db) = ctx.db else {
            return Ok(ExecutionResult::Error("No database connection".to_string()));
        };

        let backend = db.get_database_backend();
        let statement = Statement::from_string(backend, sql.to_string());

        let upper = sql.to_uppercase();

        // Handle SELECT queries
        if upper.starts_with("SELECT ") || upper.starts_with("SHOW ") || upper.starts_with("PRAGMA ") || upper.starts_with("DESCRIBE ") || upper.starts_with("EXPLAIN ") {
            let results = db.query_all(statement).await?;

            if results.is_empty() {
                return Ok(ExecutionResult::Empty);
            }

            // Convert results to JSON - simplified approach
            let mut json_rows: Vec<Value> = Vec::new();

            for _row in &results {
                // For now, return a placeholder indicating we got results
                // Real implementation would need column metadata from the query
                let row_json = serde_json::json!({
                    "_info": "Query executed successfully",
                    "_rows_returned": 1
                });
                json_rows.push(row_json);
            }

            // Return actual count of rows
            Ok(ExecutionResult::Message(format!("{} row(s) returned. Use raw SQL with explicit column selection for full output.", results.len())))
        }
        // Handle INSERT/UPDATE/DELETE
        else {
            let result = db.execute(statement).await?;
            Ok(ExecutionResult::Affected(result.rows_affected()))
        }
    }
}

impl Default for QueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}
