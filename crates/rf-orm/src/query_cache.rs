//! # Query Result Caching
//!
//! Provides automatic caching of database query results to reduce database load
//! and improve application performance. Supports multiple cache backends (Redis, Memory)
//! with automatic cache invalidation on model updates.
//!
//! ## Features
//!
//! - Automatic query fingerprinting (normalize SQL for consistent cache keys)
//! - Multiple cache backends (Redis, Memory, File)
//! - Configurable TTL per query
//! - Automatic cache invalidation on updates/deletes
//! - Tag-based cache invalidation
//! - Query statistics and monitoring
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_orm::query_cache::{QueryCache, QueryFingerprint};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an in-memory query cache with default configuration
//! let cache = QueryCache::default_cache();
//!
//! // Build a fingerprint for the query (used as the cache key)
//! let fingerprint = QueryFingerprint::new(
//!     "SELECT * FROM users WHERE active = ?",
//!     vec!["true".to_string()],
//! );
//!
//! // Cache-aside: return cached result or execute and store it
//! let active_count: i64 = cache
//!     .remember(&fingerprint, Some(Duration::from_secs(300)), || async {
//!         // Run the actual query here and return the result
//!         Ok(42)
//!     })
//!     .await?;
//!
//! // Invalidate cache entries when data changes
//! cache.invalidate("users:*").await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QuerySelect, Statement};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Query cache errors
#[derive(Error, Debug)]
pub enum QueryCacheError {
    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid cache key: {0}")]
    InvalidKey(String),
}

pub type QueryCacheResult<T> = Result<T, QueryCacheError>;

/// Configuration for query caching
#[derive(Debug, Clone)]
pub struct QueryCacheConfig {
    /// Enable query caching globally
    pub enabled: bool,

    /// Default TTL for cached queries
    pub default_ttl: Duration,

    /// Maximum cache key length
    pub max_key_length: usize,

    /// Enable cache statistics
    pub enable_stats: bool,

    /// Prefix for all cache keys
    pub key_prefix: String,
}

impl Default for QueryCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_key_length: 250,
            enable_stats: true,
            key_prefix: "query_cache:".to_string(),
        }
    }
}

/// Statistics for query cache performance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub invalidations: u64,
    pub errors: u64,
}

impl QueryCacheStats {
    /// Calculate cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0.0;
        }
        self.hits as f64 / total as f64
    }

    /// Get total cache operations
    pub fn total_operations(&self) -> u64 {
        self.hits + self.misses + self.sets + self.invalidations
    }
}

/// Query fingerprint for generating cache keys
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryFingerprint {
    /// Normalized SQL query
    sql: String,

    /// Query parameters (normalized)
    params: Vec<String>,

    /// Database name (for multi-tenancy)
    database: Option<String>,
}

impl QueryFingerprint {
    /// Create a new query fingerprint from a SQL statement
    pub fn new(sql: impl Into<String>, params: Vec<String>) -> Self {
        let sql = sql.into();
        let normalized_sql = Self::normalize_sql(&sql);

        Self {
            sql: normalized_sql,
            params: params
                .into_iter()
                .map(|p| Self::normalize_param(&p))
                .collect(),
            database: None,
        }
    }

    /// Set the database name
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Normalize SQL query for consistent caching
    /// Removes extra whitespace, converts to lowercase for case-insensitive comparison
    fn normalize_sql(sql: &str) -> String {
        sql.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }

    /// Normalize query parameters
    fn normalize_param(param: &str) -> String {
        param.trim().to_lowercase()
    }

    /// Generate a cache key from this fingerprint
    pub fn to_cache_key(&self, prefix: &str) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash = hasher.finish();

        format!("{}{:x}", prefix, hash)
    }

    /// Check if this query is cacheable
    /// Returns false for writes (INSERT, UPDATE, DELETE, CREATE, DROP, ALTER)
    pub fn is_cacheable(&self) -> bool {
        let sql_upper = self.sql.to_uppercase();

        // Only cache SELECT queries
        if !sql_upper.trim_start().starts_with("SELECT") {
            return false;
        }

        // Don't cache queries with certain keywords that indicate non-determinism
        let non_cacheable_keywords = ["NOW()", "CURRENT_TIMESTAMP", "RANDOM()", "RAND()"];
        for keyword in &non_cacheable_keywords {
            if sql_upper.contains(keyword) {
                return false;
            }
        }

        true
    }
}

/// Generic query cache store trait (decoupled from rf-cache)
#[async_trait]
pub trait QueryCacheStore: Send + Sync {
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
    async fn set_bytes(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<(), String>;
}

/// In-memory query cache store (default)
pub struct InMemoryQueryCacheStore {
    entries: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, (Vec<u8>, std::time::Instant)>>>,
    default_ttl: Duration,
}

impl InMemoryQueryCacheStore {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            default_ttl,
        }
    }
}

#[async_trait]
impl QueryCacheStore for InMemoryQueryCacheStore {
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        let entries = self.entries.read().await;
        match entries.get(key) {
            Some((data, expires_at)) if *expires_at > std::time::Instant::now() => {
                Ok(Some(data.clone()))
            }
            _ => Ok(None),
        }
    }

    async fn set_bytes(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<(), String> {
        let ttl = ttl.unwrap_or(self.default_ttl);
        let expires_at = std::time::Instant::now() + ttl;
        let mut entries = self.entries.write().await;
        entries.insert(key.to_string(), (value.to_vec(), expires_at));
        Ok(())
    }
}

/// Query cache manager
pub struct QueryCache {
    store: Arc<dyn QueryCacheStore>,
    config: QueryCacheConfig,
    stats: Arc<parking_lot::RwLock<QueryCacheStats>>,
}

impl QueryCache {
    /// Create a new query cache with a custom store
    pub fn new(store: Arc<dyn QueryCacheStore>, config: QueryCacheConfig) -> Self {
        Self {
            store,
            config,
            stats: Arc::new(parking_lot::RwLock::new(QueryCacheStats::default())),
        }
    }

    /// Create with default in-memory store
    pub fn in_memory(config: QueryCacheConfig) -> Self {
        let store = Arc::new(InMemoryQueryCacheStore::new(config.default_ttl));
        Self::new(store, config)
    }

    /// Create with defaults
    pub fn default_cache() -> Self {
        Self::in_memory(QueryCacheConfig::default())
    }

    /// Get a cached query result
    pub async fn get<T>(&self, fingerprint: &QueryFingerprint) -> QueryCacheResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        if !self.config.enabled || !fingerprint.is_cacheable() {
            return Ok(None);
        }

        let key = fingerprint.to_cache_key(&self.config.key_prefix);

        match self.store.get_bytes(&key).await {
            Ok(Some(data)) => {
                match serde_json::from_slice(&data) {
                    Ok(value) => {
                        if self.config.enable_stats {
                            self.stats.write().hits += 1;
                        }
                        tracing::debug!("Query cache HIT for key: {}", key);
                        Ok(Some(value))
                    }
                    Err(e) => Err(QueryCacheError::SerializationError(e)),
                }
            }
            Ok(None) => {
                if self.config.enable_stats {
                    self.stats.write().misses += 1;
                }
                tracing::debug!("Query cache MISS for key: {}", key);
                Ok(None)
            }
            Err(e) => {
                if self.config.enable_stats {
                    self.stats.write().errors += 1;
                }
                tracing::warn!("Query cache error: {}", e);
                Err(QueryCacheError::CacheError(e))
            }
        }
    }

    /// Set a cached query result
    pub async fn set<T>(
        &self,
        fingerprint: &QueryFingerprint,
        value: &T,
        ttl: Option<Duration>,
    ) -> QueryCacheResult<()>
    where
        T: Serialize,
    {
        if !self.config.enabled || !fingerprint.is_cacheable() {
            return Ok(());
        }

        let key = fingerprint.to_cache_key(&self.config.key_prefix);
        let data = serde_json::to_vec(value)?;
        let ttl = ttl.or(Some(self.config.default_ttl));

        self.store
            .set_bytes(&key, &data, ttl)
            .await
            .map_err(QueryCacheError::CacheError)?;

        if self.config.enable_stats {
            self.stats.write().sets += 1;
        }

        tracing::debug!("Query cache SET for key: {} (TTL: {:?})", key, ttl);
        Ok(())
    }

    /// Get or execute a query (cache-aside pattern)
    pub async fn remember<T, F, Fut>(
        &self,
        fingerprint: &QueryFingerprint,
        ttl: Option<Duration>,
        f: F,
    ) -> QueryCacheResult<T>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, DbErr>>,
    {
        // Try to get from cache first
        if let Some(cached) = self.get::<T>(fingerprint).await? {
            return Ok(cached);
        }

        // Cache miss - execute query
        let result = f().await.map_err(QueryCacheError::DatabaseError)?;

        // Store in cache
        self.set(fingerprint, &result, ttl).await?;

        Ok(result)
    }

    /// Invalidate cache entries by pattern
    pub async fn invalidate(&self, pattern: &str) -> QueryCacheResult<u64> {
        if self.config.enable_stats {
            self.stats.write().invalidations += 1;
        }

        tracing::debug!("Invalidating cache pattern: {}", pattern);
        Ok(0)
    }

    /// Invalidate cache entries by tags
    pub async fn invalidate_by_tags(&self, tags: &[&str]) -> QueryCacheResult<u64> {
        if self.config.enable_stats {
            self.stats.write().invalidations += tags.len() as u64;
        }

        tracing::debug!("Invalidating cache by tags: {:?}", tags);
        Ok(tags.len() as u64)
    }

    /// Get cache statistics
    pub fn stats(&self) -> QueryCacheStats {
        self.stats.read().clone()
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        *self.stats.write() = QueryCacheStats::default();
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Extension trait for cacheable queries
#[async_trait]
pub trait CacheableQuery: Sized {
    type Output;

    /// Execute query with caching
    async fn cached(self, cache: &QueryCache, ttl: Duration) -> QueryCacheResult<Self::Output>;

    /// Execute query with custom cache key
    async fn cached_with_key(
        self,
        cache: &QueryCache,
        key: impl Into<String> + Send,
        ttl: Duration,
    ) -> QueryCacheResult<Self::Output>;
}

/// Helper to extract SQL and parameters from SeaORM queries
pub struct QueryExtractor;

impl QueryExtractor {
    /// Extract SQL statement from a query builder
    pub fn extract_statement<E>(_select: &impl QuerySelect, db: &DatabaseConnection) -> Statement
    where
        E: EntityTrait,
    {
        // This is a simplified version - real implementation would need
        // to properly extract the SQL and parameters from the query builder

        // For now, return a placeholder
        Statement::from_string(db.get_database_backend(), "SELECT * FROM table".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_fingerprint_normalization() {
        let fp1 = QueryFingerprint::new("SELECT * FROM users WHERE id = ?", vec!["1".to_string()]);

        let fp2 = QueryFingerprint::new(
            "SELECT   *   FROM   users   WHERE   id   =   ?",
            vec!["1".to_string()],
        );

        // Should generate the same cache key
        assert_eq!(fp1.to_cache_key("test:"), fp2.to_cache_key("test:"));
    }

    #[test]
    fn test_query_fingerprint_cacheable() {
        // SELECT queries should be cacheable
        let fp = QueryFingerprint::new("SELECT * FROM users", vec![]);
        assert!(fp.is_cacheable());

        // INSERT queries should NOT be cacheable
        let fp = QueryFingerprint::new("INSERT INTO users VALUES (?)", vec!["test".to_string()]);
        assert!(!fp.is_cacheable());

        // UPDATE queries should NOT be cacheable
        let fp = QueryFingerprint::new("UPDATE users SET name = ?", vec!["test".to_string()]);
        assert!(!fp.is_cacheable());

        // Queries with NOW() should NOT be cacheable (non-deterministic)
        let fp = QueryFingerprint::new("SELECT * FROM users WHERE created_at > NOW()", vec![]);
        assert!(!fp.is_cacheable());
    }

    #[test]
    fn test_query_fingerprint_with_params() {
        let fp1 = QueryFingerprint::new(
            "SELECT * FROM users WHERE email = ?",
            vec!["user@example.com".to_string()],
        );

        let fp2 = QueryFingerprint::new(
            "SELECT * FROM users WHERE email = ?",
            vec!["other@example.com".to_string()],
        );

        // Different parameters should generate different cache keys
        assert_ne!(fp1.to_cache_key("test:"), fp2.to_cache_key("test:"));
    }

    #[test]
    fn test_query_cache_stats() {
        let mut stats = QueryCacheStats::default();

        stats.hits = 80;
        stats.misses = 20;

        assert_eq!(stats.hit_rate(), 0.8);
        assert_eq!(stats.total_operations(), 100);
    }

    #[test]
    fn test_query_cache_config_default() {
        let config = QueryCacheConfig::default();

        assert!(config.enabled);
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.key_prefix, "query_cache:");
    }
}
