//! Automatic eager loading detection to prevent N+1 queries
//!
//! This module tracks query patterns and automatically suggests or enables
//! eager loading when N+1 query patterns are detected.
//!
//! # Example
//!
//! ```rust,no_run
//! use rf_eloquent::auto_eager_load::QueryTracker;
//!
//! let tracker = QueryTracker::new(3); // Threshold of 3 queries
//!
//! // These queries will be tracked
//! // for post in posts {
//! //     let user = post.user().get().await?;  // Triggers N+1 detection
//! // }
//!
//! // Check for N+1 patterns
//! let patterns = tracker.detect_n_plus_one();
//! for pattern in patterns {
//!     println!("N+1 detected: {:?}", pattern);
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Query log entry
#[derive(Debug, Clone)]
pub struct QueryLog {
    /// Model name being queried
    pub model: String,

    /// Relationship name (if accessing a relationship)
    pub relation: Option<String>,

    /// Number of times this query was executed
    pub count: usize,

    /// When this query was first logged
    pub first_seen: Instant,

    /// When this query was last logged
    pub last_seen: Instant,

    /// SQL query (optional, for debugging)
    pub sql: Option<String>,
}

impl QueryLog {
    fn new(model: String, relation: Option<String>, sql: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            model,
            relation,
            count: 1,
            first_seen: now,
            last_seen: now,
            sql,
        }
    }

    fn increment(&mut self) {
        self.count += 1;
        self.last_seen = Instant::now();
    }
}

/// N+1 query pattern detection
#[derive(Debug, Clone)]
pub struct NPlusOnePattern {
    /// Model with the N+1 issue
    pub model: String,

    /// Relationship causing the issue
    pub relation: String,

    /// Number of individual queries made
    pub query_count: usize,

    /// Suggested eager loading
    pub suggestion: String,

    /// Timeframe of the pattern
    pub duration: Duration,
}

impl NPlusOnePattern {
    /// Format as a warning message
    pub fn warning_message(&self) -> String {
        format!(
            "N+1 query detected: {} queries for {}.{} in {:?}. \
             Consider using: Model::with(\"{}\").get()",
            self.query_count, self.model, self.relation, self.duration, self.relation
        )
    }
}

/// Query tracker for detecting N+1 patterns
pub struct QueryTracker {
    /// Tracked queries
    queries: Arc<Mutex<Vec<QueryLog>>>,

    /// Grouped queries by model:relation
    grouped: Arc<Mutex<HashMap<String, QueryLog>>>,

    /// Threshold for N+1 detection (default: 5)
    threshold: usize,

    /// Auto-suggest eager loading
    auto_suggest: bool,

    /// Detected patterns cache
    patterns: Arc<Mutex<Vec<NPlusOnePattern>>>,
}

impl QueryTracker {
    /// Create a new query tracker
    ///
    /// # Arguments
    ///
    /// * `threshold` - Number of queries before considering it an N+1 pattern
    pub fn new(threshold: usize) -> Self {
        Self {
            queries: Arc::new(Mutex::new(Vec::new())),
            grouped: Arc::new(Mutex::new(HashMap::new())),
            threshold,
            auto_suggest: true,
            patterns: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create with default threshold (5 queries)
    pub fn default() -> Self {
        Self::new(5)
    }

    /// Enable or disable auto-suggestion
    pub fn set_auto_suggest(&mut self, enabled: bool) {
        self.auto_suggest = enabled;
    }

    /// Log a query
    ///
    /// # Arguments
    ///
    /// * `model` - Model name (e.g., "User")
    /// * `relation` - Relationship name if applicable (e.g., Some("posts"))
    /// * `sql` - Optional SQL query for debugging
    pub fn log_query(&self, model: &str, relation: Option<&str>, sql: Option<String>) {
        let key = format!("{}:{}", model, relation.unwrap_or("_primary_"));

        // Update grouped queries
        {
            let mut grouped = self.grouped.lock().unwrap();
            grouped
                .entry(key.clone())
                .and_modify(|log| log.increment())
                .or_insert_with(|| {
                    QueryLog::new(
                        model.to_string(),
                        relation.map(|s| s.to_string()),
                        sql.clone(),
                    )
                });
        }

        // Add to query log
        {
            let mut queries = self.queries.lock().unwrap();
            queries.push(QueryLog::new(
                model.to_string(),
                relation.map(|s| s.to_string()),
                sql,
            ));
        }

        // Check for N+1 patterns if auto-suggest is enabled
        if self.auto_suggest {
            self.check_and_warn(&key);
        }
    }

    /// Check if a specific model:relation combination should be eager loaded
    fn check_and_warn(&self, key: &str) {
        let should_warn = {
            let grouped = self.grouped.lock().unwrap();
            grouped
                .get(key)
                .map(|log| log.count >= self.threshold)
                .unwrap_or(false)
        };

        if should_warn {
            let pattern = {
                let grouped = self.grouped.lock().unwrap();
                grouped.get(key).and_then(Self::create_pattern_from)
            };
            if let Some(pattern) = pattern {
                tracing::warn!("{}", pattern.warning_message());

                let mut patterns = self.patterns.lock().unwrap();
                // Only add if not already detected
                if !patterns
                    .iter()
                    .any(|p| p.model == pattern.model && p.relation == pattern.relation)
                {
                    patterns.push(pattern);
                }
            }
        }
    }

    /// Create an N+1 pattern from a query log entry
    fn create_pattern_from(log: &QueryLog) -> Option<NPlusOnePattern> {
        let relation = log.relation.as_ref()?;

        let duration = log.last_seen.duration_since(log.first_seen);

        Some(NPlusOnePattern {
            model: log.model.clone(),
            relation: relation.clone(),
            query_count: log.count,
            suggestion: format!("with(\"{}\")", relation),
            duration,
        })
    }

    /// Detect all N+1 patterns
    pub fn detect_n_plus_one(&self) -> Vec<NPlusOnePattern> {
        let grouped = self.grouped.lock().unwrap();
        let mut patterns = Vec::new();

        for (_key, log) in grouped.iter() {
            if log.count >= self.threshold && log.relation.is_some() {
                if let Some(pattern) = Self::create_pattern_from(log) {
                    patterns.push(pattern);
                }
            }
        }

        patterns
    }

    /// Check if a specific model:relation should be eager loaded
    pub fn should_eager_load(&self, model: &str, relation: &str) -> bool {
        let key = format!("{}:{}", model, relation);
        let grouped = self.grouped.lock().unwrap();

        grouped
            .get(&key)
            .map(|log| log.count >= self.threshold)
            .unwrap_or(false)
    }

    /// Get all detected patterns
    pub fn get_patterns(&self) -> Vec<NPlusOnePattern> {
        self.patterns.lock().unwrap().clone()
    }

    /// Clear all tracked queries
    pub fn clear(&self) {
        self.queries.lock().unwrap().clear();
        self.grouped.lock().unwrap().clear();
        self.patterns.lock().unwrap().clear();
    }

    /// Get query statistics
    pub fn stats(&self) -> QueryStats {
        let total_queries = self.queries.lock().unwrap().len();
        let unique_patterns = self.grouped.lock().unwrap().len();
        let detected_patterns = self.detect_n_plus_one().len();

        QueryStats {
            total_queries,
            unique_patterns,
            detected_n_plus_one: detected_patterns,
            threshold: self.threshold,
        }
    }
}

/// Query statistics
#[derive(Debug, Clone)]
pub struct QueryStats {
    /// Total number of queries tracked
    pub total_queries: usize,

    /// Number of unique query patterns
    pub unique_patterns: usize,

    /// Number of detected N+1 patterns
    pub detected_n_plus_one: usize,

    /// Threshold used for detection
    pub threshold: usize,
}

impl QueryStats {
    /// Check if performance is good (no N+1 detected)
    pub fn is_healthy(&self) -> bool {
        self.detected_n_plus_one == 0
    }

    /// Get efficiency ratio (unique patterns / total queries)
    pub fn efficiency_ratio(&self) -> f64 {
        if self.total_queries == 0 {
            return 1.0;
        }
        self.unique_patterns as f64 / self.total_queries as f64
    }
}

/// Global query tracker instance
static GLOBAL_TRACKER: once_cell::sync::Lazy<QueryTracker> =
    once_cell::sync::Lazy::new(|| QueryTracker::new(5));

/// Get the global query tracker
pub fn global() -> &'static QueryTracker {
    &GLOBAL_TRACKER
}

/// Log a query to the global tracker
pub fn log_query(model: &str, relation: Option<&str>, sql: Option<String>) {
    global().log_query(model, relation, sql);
}

/// Detect N+1 patterns in the global tracker
pub fn detect_n_plus_one() -> Vec<NPlusOnePattern> {
    global().detect_n_plus_one()
}

/// Check if eager loading should be used
pub fn should_eager_load(model: &str, relation: &str) -> bool {
    global().should_eager_load(model, relation)
}

/// Clear the global tracker
pub fn clear() {
    global().clear();
}

/// Get global tracker statistics
pub fn stats() -> QueryStats {
    global().stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_n_plus_one_detection() {
        let tracker = QueryTracker::new(3);

        // Simulate N+1 pattern: loading posts, then loading user for each post
        tracker.log_query("Post", None, None); // Load all posts

        // Load user for each post (N+1 pattern)
        for _ in 0..5 {
            tracker.log_query(
                "User",
                Some("posts"),
                Some("SELECT * FROM users WHERE id = ?".to_string()),
            );
        }

        let patterns = tracker.detect_n_plus_one();
        assert_eq!(patterns.len(), 1);

        let pattern = &patterns[0];
        assert_eq!(pattern.model, "User");
        assert_eq!(pattern.relation, "posts");
        assert_eq!(pattern.query_count, 5);
    }

    #[test]
    fn test_should_eager_load() {
        let tracker = QueryTracker::new(3);

        assert!(!tracker.should_eager_load("User", "posts"));

        // Log queries
        for _ in 0..4 {
            tracker.log_query("User", Some("posts"), None);
        }

        assert!(tracker.should_eager_load("User", "posts"));
    }

    #[test]
    fn test_query_stats() {
        let tracker = QueryTracker::new(5);

        tracker.log_query("Post", None, None);
        tracker.log_query("User", Some("posts"), None);
        tracker.log_query("User", Some("posts"), None);
        tracker.log_query("Comment", Some("user"), None);

        let stats = tracker.stats();
        assert_eq!(stats.total_queries, 4);
        assert_eq!(stats.unique_patterns, 3);
        assert!(stats.is_healthy()); // No N+1 yet (below threshold)
    }

    #[test]
    fn test_clear() {
        let tracker = QueryTracker::new(3);

        tracker.log_query("User", Some("posts"), None);
        tracker.log_query("User", Some("posts"), None);

        let stats = tracker.stats();
        assert_eq!(stats.total_queries, 2);

        tracker.clear();

        let stats = tracker.stats();
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.unique_patterns, 0);
    }
}
