//! # Sharding Strategies
//!
//! Different algorithms for distributing data across shards.

use super::manager::{ShardError, ShardResult, ShardStrategy};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Hash-based sharding using consistent hashing
///
/// Distributes data evenly across shards based on a hash function.
/// Good for general-purpose sharding when data should be distributed uniformly.
///
/// # Example
///
/// ```rust
/// use rf_orm::sharding::strategies::HashStrategy;
///
/// let strategy = HashStrategy::new(vec![
///     "shard_1".to_string(),
///     "shard_2".to_string(),
///     "shard_3".to_string(),
/// ]);
/// ```
pub struct HashStrategy {
    shard_count: usize,
    shard_names: Vec<String>,
}

impl HashStrategy {
    /// Create a new hash-based sharding strategy
    ///
    /// # Arguments
    ///
    /// * `shard_names` - List of shard identifiers
    ///
    /// # Panics
    ///
    /// Panics if `shard_names` is empty
    pub fn new(shard_names: Vec<String>) -> Self {
        assert!(!shard_names.is_empty(), "At least one shard required");
        let shard_count = shard_names.len();
        Self {
            shard_count,
            shard_names,
        }
    }

    /// Hash a key to determine shard index
    fn hash_key(&self, key: &str) -> usize {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }
}

#[async_trait]
impl ShardStrategy for HashStrategy {
    async fn get_shard(&self, key: &str) -> ShardResult<String> {
        if key.is_empty() {
            return Err(ShardError::InvalidKey("Key cannot be empty".to_string()));
        }

        let index = self.hash_key(key);
        Ok(self.shard_names[index].clone())
    }

    async fn get_all_shards(&self) -> Vec<String> {
        self.shard_names.clone()
    }
}

/// Range-based sharding using ID ranges
///
/// Routes data to shards based on numeric ranges.
/// Good for time-series data or when you want explicit control over data distribution.
///
/// # Example
///
/// ```rust
/// use rf_orm::sharding::strategies::RangeStrategy;
///
/// let strategy = RangeStrategy::new(vec![
///     (1, 1000000, "shard_1".to_string()),      // IDs 1-1M
///     (1000001, 2000000, "shard_2".to_string()), // IDs 1M-2M
///     (2000001, 3000000, "shard_3".to_string()), // IDs 2M-3M
/// ]);
/// ```
pub struct RangeStrategy {
    ranges: Vec<(i64, i64, String)>, // (min, max, shard_name)
}

impl RangeStrategy {
    /// Create a new range-based sharding strategy
    ///
    /// # Arguments
    ///
    /// * `ranges` - List of (min, max, shard_name) tuples
    ///
    /// # Panics
    ///
    /// Panics if `ranges` is empty
    pub fn new(ranges: Vec<(i64, i64, String)>) -> Self {
        assert!(!ranges.is_empty(), "At least one range required");
        Self { ranges }
    }

    /// Add a new range to the strategy
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum value (inclusive)
    /// * `max` - Maximum value (inclusive)
    /// * `shard_name` - Shard identifier
    pub fn add_range(&mut self, min: i64, max: i64, shard_name: String) {
        self.ranges.push((min, max, shard_name));
    }
}

#[async_trait]
impl ShardStrategy for RangeStrategy {
    async fn get_shard(&self, key: &str) -> ShardResult<String> {
        let id: i64 = key
            .parse()
            .map_err(|_| ShardError::InvalidKey(format!("Key '{}' is not a valid integer", key)))?;

        for (min, max, shard_name) in &self.ranges {
            if id >= *min && id <= *max {
                return Ok(shard_name.clone());
            }
        }

        Err(ShardError::StrategyError(format!(
            "No shard found for key '{}'",
            key
        )))
    }

    async fn get_all_shards(&self) -> Vec<String> {
        self.ranges
            .iter()
            .map(|(_, _, name)| name.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}

/// Tenant-based sharding using explicit tenant mapping
///
/// Maps specific tenants to specific shards.
/// Perfect for multi-tenant applications where you want explicit control
/// over tenant placement (e.g., premium customers on dedicated shards).
///
/// # Example
///
/// ```rust
/// use rf_orm::sharding::strategies::TenantStrategy;
/// use std::collections::HashMap;
///
/// let mut mapping = HashMap::new();
/// mapping.insert("tenant_acme".to_string(), "shard_premium".to_string());
/// mapping.insert("tenant_widgets".to_string(), "shard_standard".to_string());
///
/// let strategy = TenantStrategy::new(mapping);
/// ```
pub struct TenantStrategy {
    tenant_map: HashMap<String, String>, // tenant_id → shard_name
    default_shard: Option<String>,
}

impl TenantStrategy {
    /// Create a new tenant-based sharding strategy
    ///
    /// # Arguments
    ///
    /// * `tenant_map` - Mapping of tenant IDs to shard names
    pub fn new(tenant_map: HashMap<String, String>) -> Self {
        Self {
            tenant_map,
            default_shard: None,
        }
    }

    /// Create a new tenant strategy with a default shard
    ///
    /// If a tenant is not found in the mapping, the default shard will be used.
    ///
    /// # Arguments
    ///
    /// * `tenant_map` - Mapping of tenant IDs to shard names
    /// * `default_shard` - Default shard for unmapped tenants
    pub fn with_default(tenant_map: HashMap<String, String>, default_shard: String) -> Self {
        Self {
            tenant_map,
            default_shard: Some(default_shard),
        }
    }

    /// Add a tenant mapping
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Tenant identifier
    /// * `shard_name` - Shard to assign this tenant to
    pub fn add_tenant(&mut self, tenant_id: String, shard_name: String) {
        self.tenant_map.insert(tenant_id, shard_name);
    }

    /// Remove a tenant mapping
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - Tenant identifier to remove
    pub fn remove_tenant(&mut self, tenant_id: &str) -> Option<String> {
        self.tenant_map.remove(tenant_id)
    }

    /// Set the default shard for unmapped tenants
    ///
    /// # Arguments
    ///
    /// * `shard_name` - Default shard name
    pub fn set_default_shard(&mut self, shard_name: String) {
        self.default_shard = Some(shard_name);
    }

    /// Get the number of mapped tenants
    pub fn tenant_count(&self) -> usize {
        self.tenant_map.len()
    }
}

#[async_trait]
impl ShardStrategy for TenantStrategy {
    async fn get_shard(&self, key: &str) -> ShardResult<String> {
        if let Some(shard) = self.tenant_map.get(key) {
            Ok(shard.clone())
        } else if let Some(ref default) = self.default_shard {
            Ok(default.clone())
        } else {
            Err(ShardError::StrategyError(format!(
                "No shard mapping found for tenant '{}'",
                key
            )))
        }
    }

    async fn get_all_shards(&self) -> Vec<String> {
        let mut shards: HashSet<String> = self.tenant_map.values().cloned().collect();
        if let Some(ref default) = self.default_shard {
            shards.insert(default.clone());
        }
        shards.into_iter().collect()
    }
}

/// Geographic sharding using region mapping
///
/// Routes data based on geographic regions.
/// Useful for GDPR compliance or reducing latency.
///
/// # Example
///
/// ```rust
/// use rf_orm::sharding::strategies::GeographicStrategy;
/// use std::collections::HashMap;
///
/// let mut mapping = HashMap::new();
/// mapping.insert("US".to_string(), "shard_us_east".to_string());
/// mapping.insert("EU".to_string(), "shard_eu_west".to_string());
/// mapping.insert("APAC".to_string(), "shard_asia_pacific".to_string());
///
/// let strategy = GeographicStrategy::new(mapping);
/// ```
pub struct GeographicStrategy {
    region_map: HashMap<String, String>, // region → shard_name
    default_shard: Option<String>,
}

impl GeographicStrategy {
    /// Create a new geographic sharding strategy
    ///
    /// # Arguments
    ///
    /// * `region_map` - Mapping of region codes to shard names
    pub fn new(region_map: HashMap<String, String>) -> Self {
        Self {
            region_map,
            default_shard: None,
        }
    }

    /// Create with a default shard for unmapped regions
    pub fn with_default(region_map: HashMap<String, String>, default_shard: String) -> Self {
        Self {
            region_map,
            default_shard: Some(default_shard),
        }
    }

    /// Add a region mapping
    pub fn add_region(&mut self, region: String, shard_name: String) {
        self.region_map.insert(region, shard_name);
    }
}

#[async_trait]
impl ShardStrategy for GeographicStrategy {
    async fn get_shard(&self, key: &str) -> ShardResult<String> {
        if let Some(shard) = self.region_map.get(key) {
            Ok(shard.clone())
        } else if let Some(ref default) = self.default_shard {
            Ok(default.clone())
        } else {
            Err(ShardError::StrategyError(format!(
                "No shard mapping found for region '{}'",
                key
            )))
        }
    }

    async fn get_all_shards(&self) -> Vec<String> {
        let mut shards: HashSet<String> = self.region_map.values().cloned().collect();
        if let Some(ref default) = self.default_shard {
            shards.insert(default.clone());
        }
        shards.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_strategy() {
        let strategy = HashStrategy::new(vec!["shard1".into(), "shard2".into(), "shard3".into()]);

        let shard1 = strategy.get_shard("user_123").await.unwrap();
        let shard2 = strategy.get_shard("user_123").await.unwrap();
        assert_eq!(shard1, shard2, "Same key should always return same shard");

        let all = strategy.get_all_shards().await;
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_hash_strategy_empty_key() {
        let strategy = HashStrategy::new(vec!["shard1".into()]);
        let result = strategy.get_shard("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_range_strategy() {
        let strategy = RangeStrategy::new(vec![
            (1, 1000, "shard1".into()),
            (1001, 2000, "shard2".into()),
            (2001, 3000, "shard3".into()),
        ]);

        assert_eq!(strategy.get_shard("500").await.unwrap(), "shard1");
        assert_eq!(strategy.get_shard("1500").await.unwrap(), "shard2");
        assert_eq!(strategy.get_shard("2500").await.unwrap(), "shard3");

        // Boundary tests
        assert_eq!(strategy.get_shard("1").await.unwrap(), "shard1");
        assert_eq!(strategy.get_shard("1000").await.unwrap(), "shard1");
        assert_eq!(strategy.get_shard("1001").await.unwrap(), "shard2");
    }

    #[tokio::test]
    async fn test_range_strategy_invalid_key() {
        let strategy = RangeStrategy::new(vec![(1, 1000, "shard1".into())]);

        let result = strategy.get_shard("not_a_number").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_range_strategy_out_of_range() {
        let strategy = RangeStrategy::new(vec![(1, 1000, "shard1".into())]);

        let result = strategy.get_shard("5000").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_strategy() {
        let mut map = HashMap::new();
        map.insert("tenant_a".to_string(), "shard1".to_string());
        map.insert("tenant_b".to_string(), "shard2".to_string());

        let strategy = TenantStrategy::new(map);

        assert_eq!(strategy.get_shard("tenant_a").await.unwrap(), "shard1");
        assert_eq!(strategy.get_shard("tenant_b").await.unwrap(), "shard2");
    }

    #[tokio::test]
    async fn test_tenant_strategy_with_default() {
        let mut map = HashMap::new();
        map.insert("tenant_a".to_string(), "shard1".to_string());

        let strategy = TenantStrategy::with_default(map, "default_shard".to_string());

        assert_eq!(strategy.get_shard("tenant_a").await.unwrap(), "shard1");
        assert_eq!(
            strategy.get_shard("unknown_tenant").await.unwrap(),
            "default_shard"
        );
    }

    #[tokio::test]
    async fn test_tenant_strategy_no_default() {
        let map = HashMap::new();
        let strategy = TenantStrategy::new(map);

        let result = strategy.get_shard("unknown_tenant").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tenant_strategy_add_remove() {
        let mut strategy = TenantStrategy::new(HashMap::new());

        strategy.add_tenant("tenant_x".to_string(), "shard_x".to_string());
        assert_eq!(strategy.get_shard("tenant_x").await.unwrap(), "shard_x");

        strategy.remove_tenant("tenant_x");
        assert!(strategy.get_shard("tenant_x").await.is_err());
    }

    #[tokio::test]
    async fn test_geographic_strategy() {
        let mut map = HashMap::new();
        map.insert("US".to_string(), "shard_us".to_string());
        map.insert("EU".to_string(), "shard_eu".to_string());

        let strategy = GeographicStrategy::new(map);

        assert_eq!(strategy.get_shard("US").await.unwrap(), "shard_us");
        assert_eq!(strategy.get_shard("EU").await.unwrap(), "shard_eu");
    }

    #[tokio::test]
    async fn test_geographic_strategy_with_default() {
        let mut map = HashMap::new();
        map.insert("US".to_string(), "shard_us".to_string());

        let strategy = GeographicStrategy::with_default(map, "shard_global".to_string());

        assert_eq!(strategy.get_shard("US").await.unwrap(), "shard_us");
        assert_eq!(strategy.get_shard("APAC").await.unwrap(), "shard_global");
    }

    #[tokio::test]
    async fn test_hash_consistency() {
        let strategy = HashStrategy::new(vec!["shard1".into(), "shard2".into()]);

        // Same key should always hash to same shard
        let results: Vec<_> = (0..100).map(|_| strategy.hash_key("test_key")).collect();

        assert!(results.iter().all(|&x| x == results[0]));
    }

    #[tokio::test]
    async fn test_range_strategy_add_range() {
        let mut strategy = RangeStrategy::new(vec![(1, 1000, "shard1".into())]);

        strategy.add_range(1001, 2000, "shard2".into());

        assert_eq!(strategy.get_shard("500").await.unwrap(), "shard1");
        assert_eq!(strategy.get_shard("1500").await.unwrap(), "shard2");
    }
}
