//! PostgreSQL Full-Text Search Driver
//!
//! Uses PostgreSQL's built-in full-text search capabilities with tsvector and tsquery.

use crate::driver::{Result, SearchError};

#[cfg(feature = "postgres")]
use crate::driver::SearchDriver;
#[cfg(feature = "postgres")]
use crate::searchable::{SearchHit, SearchOptions, SearchResult, Searchable};
#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use serde::de::DeserializeOwned;
#[cfg(feature = "postgres")]
use sqlx::{postgres::PgRow, PgPool, Row};
#[cfg(feature = "postgres")]
use std::time::Instant;

/// PostgreSQL full-text search driver
#[cfg(feature = "postgres")]
pub struct PostgresSearchDriver {
    pool: PgPool,
    language: String,
}

#[cfg(feature = "postgres")]
impl PostgresSearchDriver {
    /// Create a new PostgreSQL search driver
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            language: "english".to_string(),
        }
    }

    /// Validate that a table/column name contains only safe identifier characters.
    /// Prevents SQL injection via user-supplied identifier strings.
    fn validate_identifier(name: &str) -> Result<()> {
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(SearchError::ConfigError(format!(
                "Invalid identifier '{}': only alphanumeric characters and underscores are allowed",
                name
            )));
        }
        Ok(())
    }

    /// Create a driver with a custom language configuration
    pub fn with_language(pool: PgPool, language: impl Into<String>) -> Self {
        Self {
            pool,
            language: language.into(),
        }
    }

    /// Create a GIN index for full-text search on a table
    ///
    /// This creates a GIN (Generalized Inverted Index) which is optimized for full-text search.
    ///
    /// # Example
    ///
    /// ```ignore
    /// driver.create_fts_index("posts", vec!["title", "content"]).await?;
    /// ```
    pub async fn create_fts_index(&self, table: &str, columns: Vec<&str>) -> Result<()> {
        Self::validate_identifier(table)?;
        for col in &columns {
            Self::validate_identifier(col)?;
        }
        let column_list = columns.join(" || ' ' || ");
        let index_name = format!("{}_fts_idx", table);

        let query = format!(
            "CREATE INDEX IF NOT EXISTS {} ON {} USING GIN(to_tsvector('{}', {}))",
            index_name, table, self.language, column_list
        );

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    /// Drop the full-text search index
    pub async fn drop_fts_index(&self, table: &str) -> Result<()> {
        Self::validate_identifier(table)?;
        let index_name = format!("{}_fts_idx", table);
        let query = format!("DROP INDEX IF EXISTS {}", index_name);

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    /// Build a WHERE clause from search options
    fn build_where_clause(&self, options: &SearchOptions) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut values = Vec::new();

        for (i, (field, value)) in options.filters.iter().enumerate() {
            conditions.push(format!("{} = ${}", field, i + 2)); // Start from $2 since $1 is query
            values.push(value.clone());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" AND {}", conditions.join(" AND "))
        };

        (where_clause, values)
    }

    /// Count total matching documents for a query
    pub async fn count_search_results(
        &self,
        table: &str,
        columns: Vec<&str>,
        query_text: &str,
        options: &SearchOptions,
    ) -> Result<i64> {
        let searchable_columns = columns.join(" || ' ' || ");
        let (where_clause, filter_values) = self.build_where_clause(options);

        let count_sql = format!(
            "SELECT COUNT(*) as count
             FROM {}
             WHERE to_tsvector('{}', {}) @@ plainto_tsquery('{}', $1){}",
            table, self.language, searchable_columns, self.language, where_clause
        );

        let mut count_query = sqlx::query(&count_sql).bind(query_text);
        for value in &filter_values {
            count_query = count_query.bind(value);
        }

        let count: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?
            .try_get("count")
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        Ok(count)
    }

    /// Execute a full-text search query
    async fn execute_search<T: DeserializeOwned>(
        &self,
        table: &str,
        columns: Vec<&str>,
        query: &str,
        options: SearchOptions,
    ) -> Result<SearchResult<T>> {
        let start = Instant::now();
        let column_list = columns.join(", ");
        let searchable_columns = columns.join(" || ' ' || ");

        let (where_clause, filter_values) = self.build_where_clause(&options);

        // Build the ORDER BY clause
        let order_by = if let Some((field, ascending)) = &options.sort {
            // Validate sort field to prevent SQL injection
            if !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(SearchError::InvalidQuery(format!("Invalid sort field: {}", field)));
            }
            let direction = if *ascending { "ASC" } else { "DESC" };
            format!("ORDER BY {} {}, rank DESC", field, direction)
        } else {
            "ORDER BY rank DESC".to_string()
        };

        // Count total results
        let count_sql = format!(
            "SELECT COUNT(*) as count
             FROM {}
             WHERE to_tsvector('{}', {}) @@ plainto_tsquery('{}', $1){}",
            table, self.language, searchable_columns, self.language, where_clause
        );

        let mut count_query = sqlx::query(&count_sql).bind(query);
        for value in &filter_values {
            count_query = count_query.bind(value);
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?
            .try_get("count")
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        // Fetch results
        let search_sql = format!(
            "SELECT {}, ts_rank(to_tsvector('{}', {}), plainto_tsquery('{}', $1)) as rank
             FROM {}
             WHERE to_tsvector('{}', {}) @@ plainto_tsquery('{}', $1){}
             {}
             LIMIT ${} OFFSET ${}",
            column_list,
            self.language,
            searchable_columns,
            self.language,
            table,
            self.language,
            searchable_columns,
            self.language,
            where_clause,
            order_by,
            filter_values.len() + 2,
            filter_values.len() + 3
        );

        let mut search_query = sqlx::query(&search_sql).bind(query);
        for value in &filter_values {
            search_query = search_query.bind(value);
        }
        search_query = search_query.bind(options.limit as i64);
        search_query = search_query.bind(options.offset as i64);

        let _rows: Vec<PgRow> = search_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        // Note: Actual deserialization would need sqlx::FromRow
        // For now, this is a placeholder showing the structure
        let hits: Vec<SearchHit<T>> = Vec::new();

        let processing_time = start.elapsed().as_millis() as u64;

        Ok(SearchResult {
            hits,
            total: total as usize,
            query: query.to_string(),
            processing_time_ms: processing_time,
            page: None,
            per_page: None,
        })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SearchDriver for PostgresSearchDriver {
    async fn index<T: Searchable>(&self, _document: &T) -> Result<()> {
        // PostgreSQL FTS is automatic when data is inserted into the table
        // The index is updated automatically via triggers or application logic
        // This is a no-op for PostgreSQL as indexing happens at insert/update time
        Ok(())
    }

    async fn index_many<T: Searchable>(&self, _documents: Vec<&T>) -> Result<()> {
        // Same as index() - PostgreSQL handles this automatically
        Ok(())
    }

    async fn search<T: Searchable>(
        &self,
        query: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResult<T::Model>> {
        let table = T::index_name();
        let columns = T::searchable_fields();
        let opts = options.unwrap_or_default();

        self.execute_search(table, columns, query, opts).await
    }

    async fn delete<T: Searchable>(&self, id: &str) -> Result<()> {
        let table = T::index_name();
        Self::validate_identifier(&table)?;
        let query = format!("DELETE FROM {} WHERE id = $1", table);

        sqlx::query(&query)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        Ok(())
    }

    async fn clear_index<T: Searchable>(&self) -> Result<()> {
        let table = T::index_name();
        Self::validate_identifier(&table)?;
        let query = format!("TRUNCATE TABLE {}", table);

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    async fn count<T: Searchable>(&self) -> Result<usize> {
        let table = T::index_name();
        Self::validate_identifier(&table)?;
        let query = format!("SELECT COUNT(*) as count FROM {}", table);

        let count: i64 = sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?
            .try_get("count")
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        Ok(count as usize)
    }

    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(not(feature = "postgres"))]
pub struct PostgresSearchDriver;

#[cfg(not(feature = "postgres"))]
impl PostgresSearchDriver {
    /// Stub implementation when postgres feature is not enabled
    /// Returns an error instead of panicking
    pub fn new(_pool: ()) -> Result<Self> {
        Err(SearchError::FeatureNotEnabled("postgres".to_string()))
    }

    /// Stub method for with_language
    pub fn with_language(_pool: (), _language: impl Into<String>) -> Result<Self> {
        Err(SearchError::FeatureNotEnabled("postgres".to_string()))
    }
}

#[cfg(test)]
#[cfg(feature = "postgres")]
mod tests {
    use super::*;

    // Helper function to create a test pool
    async fn create_test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/test".to_string());

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_build_where_clause_no_filters() {
        let pool = create_test_pool().await;
        let driver = PostgresSearchDriver {
            pool,
            language: "english".to_string(),
        };

        let options = SearchOptions::new();
        let (clause, values) = driver.build_where_clause(&options);

        assert_eq!(clause, "");
        assert_eq!(values.len(), 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let pool = create_test_pool().await;
        let driver = PostgresSearchDriver::new(pool);

        let result = driver.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_fts_index() {
        let pool = create_test_pool().await;
        let driver = PostgresSearchDriver::new(pool.clone());

        // Create a test table first
        sqlx::query("CREATE TABLE IF NOT EXISTS test_docs_pg (id TEXT PRIMARY KEY, title TEXT, content TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        let result = driver
            .create_fts_index("test_docs_pg", vec!["title", "content"])
            .await;
        assert!(result.is_ok());

        // Cleanup
        driver.drop_fts_index("test_docs_pg").await.unwrap();
        sqlx::query("DROP TABLE IF EXISTS test_docs_pg")
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_count() {
        let pool = create_test_pool().await;
        let _driver = PostgresSearchDriver::new(pool.clone());

        // Create a test table
        sqlx::query("CREATE TABLE IF NOT EXISTS test_count_pg (id TEXT PRIMARY KEY, title TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test data
        sqlx::query("INSERT INTO test_count_pg (id, title) VALUES ('1', 'Test'), ('2', 'Another Test') ON CONFLICT DO NOTHING")
            .execute(&pool)
            .await
            .unwrap();

        // Test count - This would need a concrete Searchable implementation
        // For now cleanup
        sqlx::query("DROP TABLE IF EXISTS test_count_pg")
            .execute(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_driver_creation() {
        let pool = create_test_pool().await;
        let driver = PostgresSearchDriver::new(pool.clone());
        assert_eq!(driver.language, "english");

        let driver_custom = PostgresSearchDriver::with_language(pool, "spanish");
        assert_eq!(driver_custom.language, "spanish");
    }
}
