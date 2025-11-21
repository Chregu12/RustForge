//! # Shard Manager
//!
//! Manages multiple database connections and routes queries to the correct shard.

use async_trait::async_trait;
use futures::future::BoxFuture;
use sea_orm::{DatabaseConnection, DbErr};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Sharding error types
#[derive(Debug, Error)]
pub enum ShardError {
    /// Shard not found
    #[error("Shard '{0}' not found")]
    ShardNotFound(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    /// Invalid shard key
    #[error("Invalid shard key: {0}")]
    InvalidKey(String),

    /// Strategy error
    #[error("Strategy error: {0}")]
    StrategyError(String),

    /// No shards configured
    #[error("No shards configured")]
    NoShards,
}

/// Result type for shard operations
pub type ShardResult<T> = Result<T, ShardError>;

/// Trait for shard selection strategies
///
/// Implement this trait to define custom sharding logic.
#[async_trait]
pub trait ShardStrategy: Send + Sync {
    /// Determine which shard to use for the given key
    ///
    /// # Arguments
    ///
    /// * `key` - Shard key (e.g., user ID, tenant ID)
    ///
    /// # Returns
    ///
    /// Name of the shard to use
    async fn get_shard(&self, key: &str) -> ShardResult<String>;

    /// Get all shard names
    ///
    /// Used for operations that need to query all shards.
    async fn get_all_shards(&self) -> Vec<String>;
}

/// Manages database shards and routes queries
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::sharding::*;
/// use std::sync::Arc;
///
/// # async fn example() -> ShardResult<()> {
/// let strategy = HashStrategy::new(vec!["shard1".into(), "shard2".into()]);
/// let mut manager = ShardManager::new(Arc::new(strategy));
///
/// manager.add_shard("shard1".into(), Arc::new(db1));
/// manager.add_shard("shard2".into(), Arc::new(db2));
///
/// // Route to correct shard
/// let db = manager.connection_for("user_123").await?;
/// # Ok(())
/// # }
/// ```
pub struct ShardManager {
    shards: HashMap<String, Arc<DatabaseConnection>>,
    strategy: Arc<dyn ShardStrategy>,
}

impl ShardManager {
    /// Create a new shard manager with the given strategy
    ///
    /// # Arguments
    ///
    /// * `strategy` - Sharding strategy to use
    pub fn new(strategy: Arc<dyn ShardStrategy>) -> Self {
        Self {
            shards: HashMap::new(),
            strategy,
        }
    }

    /// Register a database shard
    ///
    /// # Arguments
    ///
    /// * `name` - Unique shard identifier
    /// * `connection` - Database connection for this shard
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::sharding::*;
    /// # use std::sync::Arc;
    /// # async fn example(manager: &mut ShardManager, db: DatabaseConnection) {
    /// manager.add_shard("shard_us_east".to_string(), Arc::new(db));
    /// # }
    /// ```
    pub fn add_shard(&mut self, name: String, connection: Arc<DatabaseConnection>) {
        self.shards.insert(name, connection);
    }

    /// Remove a shard by name
    ///
    /// # Arguments
    ///
    /// * `name` - Shard identifier to remove
    pub fn remove_shard(&mut self, name: &str) -> Option<Arc<DatabaseConnection>> {
        self.shards.remove(name)
    }

    /// Get connection for a specific shard key
    ///
    /// # Arguments
    ///
    /// * `key` - Shard key to route
    ///
    /// # Returns
    ///
    /// Database connection for the appropriate shard
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::sharding::*;
    /// # async fn example(manager: &ShardManager) -> ShardResult<()> {
    /// let user_id = "12345";
    /// let db = manager.connection_for(user_id).await?;
    /// // Use db for queries...
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connection_for(&self, key: &str) -> ShardResult<Arc<DatabaseConnection>> {
        #[cfg(feature = "metrics")]
        {
            rf_metrics::SHARD_QUERIES.inc();
            let timer = rf_metrics::SHARD_QUERY_DURATION
                .with_label_values(&[key])
                .start_timer();

            let result = async {
                let shard_name = self.strategy.get_shard(key).await?;
                self.shards
                    .get(&shard_name)
                    .cloned()
                    .ok_or_else(|| ShardError::ShardNotFound(shard_name))
            }.await;

            if result.is_err() {
                rf_metrics::SHARD_QUERY_ERRORS.inc();
            }

            drop(timer);
            result
        }

        #[cfg(not(feature = "metrics"))]
        {
            let shard_name = self.strategy.get_shard(key).await?;
            self.shards
                .get(&shard_name)
                .cloned()
                .ok_or_else(|| ShardError::ShardNotFound(shard_name))
        }
    }

    /// Get a specific shard by name
    ///
    /// # Arguments
    ///
    /// * `name` - Shard identifier
    pub fn get_shard(&self, name: &str) -> ShardResult<Arc<DatabaseConnection>> {
        self.shards
            .get(name)
            .cloned()
            .ok_or_else(|| ShardError::ShardNotFound(name.to_string()))
    }

    /// Get all shard connections
    ///
    /// Returns a vector of tuples (shard_name, connection)
    pub fn all_shards(&self) -> Vec<(String, Arc<DatabaseConnection>)> {
        self.shards
            .iter()
            .map(|(name, conn)| (name.clone(), conn.clone()))
            .collect()
    }

    /// Execute a query on all shards
    ///
    /// Useful for global operations like counting total users across all shards.
    ///
    /// # Arguments
    ///
    /// * `f` - Async function to execute on each shard
    ///
    /// # Returns
    ///
    /// Vector of results from each shard
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::sharding::*;
    /// # use sea_orm::*;
    /// # async fn example(manager: &ShardManager) -> ShardResult<()> {
    /// let counts: Vec<i64> = manager.execute_on_all(|db| {
    ///     Box::pin(async move {
    ///         // Count users on this shard
    ///         let count = User::find().count(db).await?;
    ///         Ok(count as i64)
    ///     })
    /// }).await?;
    ///
    /// let total: i64 = counts.iter().sum();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_on_all<F, T>(&self, f: F) -> ShardResult<Vec<T>>
    where
        F: Fn(&DatabaseConnection) -> BoxFuture<'_, Result<T, DbErr>> + Send + Sync,
        T: Send,
    {
        if self.shards.is_empty() {
            return Err(ShardError::NoShards);
        }

        let mut results = Vec::new();
        for (_name, shard) in &self.shards {
            let result = f(shard.as_ref()).await?;
            results.push(result);
        }
        Ok(results)
    }

    /// Execute a query on specific shards
    ///
    /// # Arguments
    ///
    /// * `shard_names` - Names of shards to query
    /// * `f` - Async function to execute on each shard
    pub async fn execute_on_shards<F, T>(
        &self,
        shard_names: Vec<String>,
        f: F,
    ) -> ShardResult<Vec<T>>
    where
        F: Fn(&DatabaseConnection) -> BoxFuture<'_, Result<T, DbErr>> + Send + Sync,
        T: Send,
    {
        let mut results = Vec::new();
        for name in shard_names {
            let shard = self.get_shard(&name)?;
            let result = f(shard.as_ref()).await?;
            results.push(result);
        }
        Ok(results)
    }

    /// Get the number of registered shards
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Check if a shard exists
    ///
    /// # Arguments
    ///
    /// * `name` - Shard identifier to check
    pub fn has_shard(&self, name: &str) -> bool {
        self.shards.contains_key(name)
    }

    /// Get list of all shard names
    pub fn shard_names(&self) -> Vec<String> {
        self.shards.keys().cloned().collect()
    }

    /// Get the sharding strategy
    pub fn strategy(&self) -> &Arc<dyn ShardStrategy> {
        &self.strategy
    }

    /// Execute function with automatic shard selection based on key
    ///
    /// Convenience method that combines connection_for and query execution.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::sharding::*;
    /// # use sea_orm::*;
    /// # async fn example(manager: &ShardManager) -> ShardResult<()> {
    /// let user = manager.execute_with_key("user_123", |db| {
    ///     Box::pin(async move {
    ///         User::find_by_id(123).one(db).await
    ///     })
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_with_key<F, T>(&self, key: &str, f: F) -> ShardResult<T>
    where
        F: Fn(&DatabaseConnection) -> BoxFuture<'_, Result<T, DbErr>> + Send + Sync,
        T: Send,
    {
        let db = self.connection_for(key).await?;
        let result = f(db.as_ref()).await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharding::strategies::HashStrategy;

    #[test]
    fn test_shard_manager_creation() {
        let strategy = Arc::new(HashStrategy::new(vec!["shard1".into()]));
        let manager = ShardManager::new(strategy);
        assert_eq!(manager.shard_count(), 0);
    }

    #[test]
    fn test_shard_names() {
        let strategy = Arc::new(HashStrategy::new(vec!["shard1".into(), "shard2".into()]));
        let mut manager = ShardManager::new(strategy);

        // Add mock shards (using Arc<DatabaseConnection> placeholders)
        // In real tests, we'd use actual database connections

        assert_eq!(manager.shard_count(), 0);
        assert!(manager.shard_names().is_empty());
    }

    #[test]
    fn test_has_shard() {
        let strategy = Arc::new(HashStrategy::new(vec!["shard1".into()]));
        let manager = ShardManager::new(strategy);

        assert!(!manager.has_shard("shard1"));
        assert!(!manager.has_shard("nonexistent"));
    }
}
