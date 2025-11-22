//! Query watcher for database query monitoring

use crate::{Entry, EntryType, Storage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Database query information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryInfo {
    pub sql: String,
    pub bindings: Vec<String>,
    pub duration_ms: f64,
    pub connection: String,
    pub executed_at: DateTime<Utc>,
}

impl QueryInfo {
    /// Create a new query info
    pub fn new(sql: impl Into<String>, connection: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            bindings: Vec::new(),
            duration_ms: 0.0,
            connection: connection.into(),
            executed_at: Utc::now(),
        }
    }

    /// Add a binding value
    pub fn with_binding(mut self, binding: impl Into<String>) -> Self {
        self.bindings.push(binding.into());
        self
    }

    /// Add multiple bindings
    pub fn with_bindings(mut self, bindings: Vec<String>) -> Self {
        self.bindings = bindings;
        self
    }

    /// Set execution duration
    pub fn with_duration(mut self, duration_ms: f64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Check if query is slow (over threshold)
    pub fn is_slow(&self, threshold_ms: f64) -> bool {
        self.duration_ms > threshold_ms
    }

    /// Get formatted query with bindings
    pub fn formatted(&self) -> String {
        let mut query = self.sql.clone();
        for binding in self.bindings.iter() {
            query = query.replacen('?', &format!("'{}'", binding), 1);
        }
        query
    }
}

/// Query watcher for monitoring database queries
#[derive(Clone)]
pub struct QueryWatcher {
    storage: Storage,
    slow_query_threshold_ms: f64,
}

impl QueryWatcher {
    /// Create a new query watcher
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            slow_query_threshold_ms: 1000.0, // 1 second default
        }
    }

    /// Set slow query threshold
    pub fn with_slow_threshold(mut self, threshold_ms: f64) -> Self {
        self.slow_query_threshold_ms = threshold_ms;
        self
    }

    /// Record a query
    pub async fn record(&self, info: QueryInfo) {
        let is_slow = info.is_slow(self.slow_query_threshold_ms);

        let entry = Entry::new(
            EntryType::Query,
            json!({
                "sql": info.sql,
                "bindings": info.bindings,
                "duration_ms": info.duration_ms,
                "connection": info.connection,
                "executed_at": info.executed_at,
                "is_slow": is_slow,
            }),
        )
        .with_tag(format!("connection:{}", info.connection));

        if is_slow {
            self.storage.store(entry.with_tag("slow")).await;
        } else {
            self.storage.store(entry).await;
        }
    }

    /// Get all recorded queries
    pub async fn all(&self) -> Vec<Entry> {
        self.storage.by_type(EntryType::Query).await
    }

    /// Get slow queries
    pub async fn slow_queries(&self) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| entry.tags.contains(&"slow".to_string()))
            .collect()
    }

    /// Get queries for a specific connection
    pub async fn by_connection(&self, connection: &str) -> Vec<Entry> {
        let all = self.all().await;
        let tag = format!("connection:{}", connection);
        all.into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect()
    }

    /// Get query statistics
    pub async fn statistics(&self) -> QueryStatistics {
        let queries = self.all().await;

        let total_queries = queries.len();
        let slow_queries = queries
            .iter()
            .filter(|e| e.tags.contains(&"slow".to_string()))
            .count();

        let durations: Vec<f64> = queries
            .iter()
            .filter_map(|e| e.content.get("duration_ms").and_then(|v| v.as_f64()))
            .collect();

        let average_duration = if !durations.is_empty() {
            durations.iter().sum::<f64>() / durations.len() as f64
        } else {
            0.0
        };

        let max_duration = durations.iter().cloned().fold(0.0, f64::max);
        let min_duration = durations.iter().cloned().fold(f64::MAX, f64::min);

        QueryStatistics {
            total_queries,
            slow_queries,
            average_duration_ms: average_duration,
            max_duration_ms: max_duration,
            min_duration_ms: if min_duration == f64::MAX {
                0.0
            } else {
                min_duration
            },
        }
    }

    /// Detect duplicate queries (same SQL executed multiple times)
    pub async fn duplicate_queries(&self) -> Vec<DuplicateQuery> {
        use std::collections::HashMap;

        let queries = self.all().await;
        let mut query_counts: HashMap<String, Vec<Entry>> = HashMap::new();

        // Group queries by SQL
        for entry in queries {
            if let Some(sql) = entry.content.get("sql").and_then(|s| s.as_str()) {
                query_counts
                    .entry(sql.to_string())
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }

        // Find duplicates (executed more than once)
        query_counts
            .into_iter()
            .filter(|(_, entries)| entries.len() > 1)
            .map(|(sql, entries)| {
                let count = entries.len();
                let total_duration: f64 = entries
                    .iter()
                    .filter_map(|e| e.content.get("duration_ms").and_then(|d| d.as_f64()))
                    .sum();

                DuplicateQuery {
                    sql,
                    count,
                    total_duration_ms: total_duration,
                    entries,
                }
            })
            .collect()
    }

    /// Find N+1 query patterns (similar queries with different bindings)
    pub async fn n_plus_one_patterns(&self) -> Vec<NPlusOnePattern> {
        use std::collections::HashMap;

        let queries = self.all().await;
        let mut patterns: HashMap<String, Vec<Entry>> = HashMap::new();

        // Normalize SQL (remove specific values) to detect patterns
        for entry in queries {
            if let Some(sql) = entry.content.get("sql").and_then(|s| s.as_str()) {
                let normalized = normalize_sql(sql);
                patterns
                    .entry(normalized)
                    .or_insert_with(Vec::new)
                    .push(entry);
            }
        }

        // Find patterns executed many times
        patterns
            .into_iter()
            .filter(|(_, entries)| entries.len() > 5) // Threshold for N+1 detection
            .map(|(pattern, entries)| {
                let count = entries.len();
                let total_duration: f64 = entries
                    .iter()
                    .filter_map(|e| e.content.get("duration_ms").and_then(|d| d.as_f64()))
                    .sum();

                NPlusOnePattern {
                    pattern,
                    count,
                    total_duration_ms: total_duration,
                    could_be_eager_loaded: true,
                }
            })
            .collect()
    }
}

/// Normalize SQL to detect patterns (remove specific values)
fn normalize_sql(sql: &str) -> String {
    // Simple normalization: replace numbers and strings with placeholders
    let mut normalized = sql.to_string();

    // Replace string literals
    normalized = regex::Regex::new(r"'[^']*'")
        .unwrap()
        .replace_all(&normalized, "'?'")
        .to_string();

    // Replace numbers
    normalized = regex::Regex::new(r"\b\d+\b")
        .unwrap()
        .replace_all(&normalized, "?")
        .to_string();

    normalized
}

/// Query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    pub total_queries: usize,
    pub slow_queries: usize,
    pub average_duration_ms: f64,
    pub max_duration_ms: f64,
    pub min_duration_ms: f64,
}

/// Duplicate query information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateQuery {
    pub sql: String,
    pub count: usize,
    pub total_duration_ms: f64,
    #[serde(skip)]
    pub entries: Vec<crate::Entry>,
}

/// N+1 query pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPlusOnePattern {
    pub pattern: String,
    pub count: usize,
    pub total_duration_ms: f64,
    pub could_be_eager_loaded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_info_creation() {
        let info = QueryInfo::new("SELECT * FROM users WHERE id = ?", "postgres")
            .with_binding("123")
            .with_duration(25.5);

        assert_eq!(info.sql, "SELECT * FROM users WHERE id = ?");
        assert_eq!(info.bindings, vec!["123"]);
        assert_eq!(info.duration_ms, 25.5);
        assert_eq!(info.connection, "postgres");
    }

    #[tokio::test]
    async fn test_query_formatting() {
        let info = QueryInfo::new("SELECT * FROM users WHERE id = ? AND name = ?", "postgres")
            .with_binding("123")
            .with_binding("John");

        let formatted = info.formatted();
        assert_eq!(
            formatted,
            "SELECT * FROM users WHERE id = '123' AND name = 'John'"
        );
    }

    #[tokio::test]
    async fn test_query_watcher_record() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage);

        let info = QueryInfo::new("SELECT * FROM users", "postgres").with_duration(15.0);

        watcher.record(info).await;

        let queries = watcher.all().await;
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].content["sql"], "SELECT * FROM users");
    }

    #[tokio::test]
    async fn test_slow_query_detection() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage).with_slow_threshold(100.0);

        // Fast query
        watcher
            .record(QueryInfo::new("SELECT 1", "postgres").with_duration(50.0))
            .await;

        // Slow query
        watcher
            .record(QueryInfo::new("SELECT * FROM large_table", "postgres").with_duration(1500.0))
            .await;

        let slow = watcher.slow_queries().await;
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].content["sql"], "SELECT * FROM large_table");
    }

    #[tokio::test]
    async fn test_query_by_connection() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage);

        watcher.record(QueryInfo::new("SELECT 1", "postgres")).await;
        watcher.record(QueryInfo::new("SELECT 2", "mysql")).await;
        watcher.record(QueryInfo::new("SELECT 3", "postgres")).await;

        let pg_queries = watcher.by_connection("postgres").await;
        assert_eq!(pg_queries.len(), 2);
    }

    #[tokio::test]
    async fn test_query_statistics() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage).with_slow_threshold(100.0);

        watcher
            .record(QueryInfo::new("SELECT 1", "postgres").with_duration(50.0))
            .await;
        watcher
            .record(QueryInfo::new("SELECT 2", "postgres").with_duration(150.0))
            .await;
        watcher
            .record(QueryInfo::new("SELECT 3", "postgres").with_duration(200.0))
            .await;

        let stats = watcher.statistics().await;
        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.slow_queries, 2);
        assert_eq!(stats.average_duration_ms, 133.33333333333334);
        assert_eq!(stats.max_duration_ms, 200.0);
        assert_eq!(stats.min_duration_ms, 50.0);
    }

    #[tokio::test]
    async fn test_duplicate_query_detection() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage);

        // Record the same query multiple times
        let sql = "SELECT * FROM users WHERE id = ?";
        watcher
            .record(QueryInfo::new(sql, "postgres").with_duration(10.0))
            .await;
        watcher
            .record(QueryInfo::new(sql, "postgres").with_duration(15.0))
            .await;
        watcher
            .record(QueryInfo::new(sql, "postgres").with_duration(20.0))
            .await;

        // Record a different query
        watcher
            .record(QueryInfo::new("SELECT * FROM posts", "postgres"))
            .await;

        let duplicates = watcher.duplicate_queries().await;
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0].sql, sql);
        assert_eq!(duplicates[0].count, 3);
        assert_eq!(duplicates[0].total_duration_ms, 45.0);
    }

    #[tokio::test]
    async fn test_n_plus_one_pattern_detection() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage);

        // Simulate N+1 pattern: same query with different IDs
        for i in 1..=10 {
            watcher
                .record(QueryInfo::new(
                    format!("SELECT * FROM posts WHERE user_id = {}", i),
                    "postgres",
                ))
                .await;
        }

        let patterns = watcher.n_plus_one_patterns().await;
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].count, 10);
        assert!(patterns[0].could_be_eager_loaded);
    }

    #[tokio::test]
    async fn test_sql_normalization() {
        let sql1 = "SELECT * FROM users WHERE id = 123";
        let sql2 = "SELECT * FROM users WHERE id = 456";
        let sql3 = "SELECT * FROM users WHERE name = 'John'";

        let norm1 = normalize_sql(sql1);
        let norm2 = normalize_sql(sql2);
        let norm3 = normalize_sql(sql3);

        // Same pattern should normalize to same string
        assert_eq!(norm1, norm2);
        // Different values should normalize to same placeholders
        assert_eq!(norm1, "SELECT * FROM users WHERE id = ?");
        assert_eq!(norm3, "SELECT * FROM users WHERE name = '?'");
    }

    #[tokio::test]
    async fn test_query_with_multiple_bindings() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage);

        let info = QueryInfo::new(
            "SELECT * FROM users WHERE id = ? AND status = ?",
            "postgres",
        )
        .with_bindings(vec!["123".to_string(), "active".to_string()])
        .with_duration(25.0);

        watcher.record(info).await;

        let queries = watcher.all().await;
        assert_eq!(queries.len(), 1);

        let bindings = queries[0].content["bindings"].as_array().unwrap();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0], "123");
        assert_eq!(bindings[1], "active");
    }

    #[tokio::test]
    async fn test_slow_threshold_customization() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage).with_slow_threshold(50.0);

        watcher
            .record(QueryInfo::new("SELECT 1", "postgres").with_duration(60.0))
            .await;
        watcher
            .record(QueryInfo::new("SELECT 2", "postgres").with_duration(40.0))
            .await;

        let slow = watcher.slow_queries().await;
        assert_eq!(slow.len(), 1);
        assert_eq!(slow[0].content["duration_ms"], 60.0);
    }

    #[tokio::test]
    async fn test_empty_statistics() {
        let storage = Storage::new();
        let watcher = QueryWatcher::new(storage);

        let stats = watcher.statistics().await;
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.slow_queries, 0);
        assert_eq!(stats.average_duration_ms, 0.0);
        assert_eq!(stats.max_duration_ms, 0.0);
        assert_eq!(stats.min_duration_ms, 0.0);
    }
}
