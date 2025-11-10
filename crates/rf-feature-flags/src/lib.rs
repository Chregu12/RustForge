//! Feature Flags for RustForge
//!
//! This crate provides dynamic feature toggling and A/B testing.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::RwLock;

/// Feature flag errors
#[derive(Debug, Error)]
pub enum FeatureFlagError {
    #[error("Flag not found: {0}")]
    FlagNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Invalid percentage: {0}")]
    InvalidPercentage(f64),
}

pub type FeatureFlagResult<T> = Result<T, FeatureFlagError>;

/// Feature flag configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagConfig {
    /// Flag name
    pub name: String,

    /// Is the flag enabled for all users
    pub enabled: bool,

    /// Percentage rollout (0.0 to 100.0)
    pub percentage: Option<f64>,

    /// Specific user IDs that have access
    pub user_ids: Vec<String>,

    /// Specific user groups that have access
    pub groups: Vec<String>,
}

impl FlagConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: false,
            percentage: None,
            user_ids: Vec::new(),
            groups: Vec::new(),
        }
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn percentage(mut self, percentage: f64) -> Self {
        self.percentage = Some(percentage);
        self
    }

    pub fn for_users(mut self, user_ids: Vec<String>) -> Self {
        self.user_ids = user_ids;
        self
    }

    pub fn for_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }
}

/// Feature flag storage trait
#[async_trait]
pub trait FlagStorage: Send + Sync {
    /// Get a flag configuration
    async fn get(&self, name: &str) -> FeatureFlagResult<Option<FlagConfig>>;

    /// Set a flag configuration
    async fn set(&self, config: FlagConfig) -> FeatureFlagResult<()>;

    /// Delete a flag
    async fn delete(&self, name: &str) -> FeatureFlagResult<()>;

    /// List all flags
    async fn list(&self) -> FeatureFlagResult<Vec<FlagConfig>>;
}

/// In-memory flag storage
pub struct MemoryStorage {
    flags: Arc<RwLock<HashMap<String, FlagConfig>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            flags: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FlagStorage for MemoryStorage {
    async fn get(&self, name: &str) -> FeatureFlagResult<Option<FlagConfig>> {
        let flags = self.flags.read().await;
        Ok(flags.get(name).cloned())
    }

    async fn set(&self, config: FlagConfig) -> FeatureFlagResult<()> {
        let mut flags = self.flags.write().await;
        flags.insert(config.name.clone(), config);
        Ok(())
    }

    async fn delete(&self, name: &str) -> FeatureFlagResult<()> {
        let mut flags = self.flags.write().await;
        flags.remove(name);
        Ok(())
    }

    async fn list(&self) -> FeatureFlagResult<Vec<FlagConfig>> {
        let flags = self.flags.read().await;
        Ok(flags.values().cloned().collect())
    }
}

/// Feature flags manager
pub struct FeatureFlags {
    storage: Arc<dyn FlagStorage>,
}

impl FeatureFlags {
    /// Create a new feature flags manager with memory storage
    pub fn new() -> Self {
        Self {
            storage: Arc::new(MemoryStorage::new()),
        }
    }

    /// Create a feature flags manager with custom storage
    pub fn with_storage(storage: Arc<dyn FlagStorage>) -> Self {
        Self { storage }
    }

    /// Check if a flag is enabled for all
    pub async fn is_enabled(&self, flag: &str) -> FeatureFlagResult<bool> {
        let config = self.storage.get(flag).await?;

        match config {
            Some(config) => Ok(config.enabled),
            None => Ok(false), // Flags default to disabled
        }
    }

    /// Check if a flag is enabled for a specific percentage
    pub async fn is_enabled_for_percentage(&self, flag: &str, user_id: &str) -> FeatureFlagResult<bool> {
        let config = self.storage.get(flag).await?;

        match config {
            Some(config) => {
                if config.enabled {
                    return Ok(true);
                }

                if let Some(percentage) = config.percentage {
                    // Use consistent hashing to determine if user is in rollout
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    format!("{}:{}", flag, user_id).hash(&mut hasher);
                    let hash = hasher.finish();

                    let user_percentage = (hash % 100) as f64;
                    return Ok(user_percentage < percentage);
                }

                Ok(false)
            }
            None => Ok(false),
        }
    }

    /// Check if a flag is enabled for a specific user
    pub async fn is_enabled_for_user(&self, flag: &str, user_id: &str) -> FeatureFlagResult<bool> {
        let config = self.storage.get(flag).await?;

        match config {
            Some(config) => {
                if config.enabled {
                    return Ok(true);
                }

                if config.user_ids.contains(&user_id.to_string()) {
                    return Ok(true);
                }

                // Check percentage rollout
                if let Some(percentage) = config.percentage {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    format!("{}:{}", flag, user_id).hash(&mut hasher);
                    let hash = hasher.finish();

                    let user_percentage = (hash % 100) as f64;
                    return Ok(user_percentage < percentage);
                }

                Ok(false)
            }
            None => Ok(false),
        }
    }

    /// Check if a flag is enabled for a user group
    pub async fn is_enabled_for_group(&self, flag: &str, group: &str) -> FeatureFlagResult<bool> {
        let config = self.storage.get(flag).await?;

        match config {
            Some(config) => {
                if config.enabled {
                    return Ok(true);
                }

                Ok(config.groups.contains(&group.to_string()))
            }
            None => Ok(false),
        }
    }

    /// Enable a flag for all users
    pub async fn enable(&self, flag: &str) -> FeatureFlagResult<()> {
        let config = FlagConfig::new(flag).enable();
        self.storage.set(config).await
    }

    /// Disable a flag for all users
    pub async fn disable(&self, flag: &str) -> FeatureFlagResult<()> {
        let config = FlagConfig::new(flag).disable();
        self.storage.set(config).await
    }

    /// Set percentage rollout
    pub async fn set_percentage(&self, flag: &str, percentage: f64) -> FeatureFlagResult<()> {
        if !(0.0..=100.0).contains(&percentage) {
            return Err(FeatureFlagError::InvalidPercentage(percentage));
        }

        let config = FlagConfig::new(flag).percentage(percentage);
        self.storage.set(config).await
    }

    /// Enable for specific users
    pub async fn enable_for_users(&self, flag: &str, user_ids: Vec<String>) -> FeatureFlagResult<()> {
        let config = FlagConfig::new(flag).for_users(user_ids);
        self.storage.set(config).await
    }

    /// Enable for specific groups
    pub async fn enable_for_groups(&self, flag: &str, groups: Vec<String>) -> FeatureFlagResult<()> {
        let config = FlagConfig::new(flag).for_groups(groups);
        self.storage.set(config).await
    }

    /// Get flag configuration
    pub async fn get_config(&self, flag: &str) -> FeatureFlagResult<Option<FlagConfig>> {
        self.storage.get(flag).await
    }

    /// Set flag configuration
    pub async fn set_config(&self, config: FlagConfig) -> FeatureFlagResult<()> {
        self.storage.set(config).await
    }

    /// Delete a flag
    pub async fn delete(&self, flag: &str) -> FeatureFlagResult<()> {
        self.storage.delete(flag).await
    }

    /// List all flags
    pub async fn list(&self) -> FeatureFlagResult<Vec<FlagConfig>> {
        self.storage.list().await
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_flag_enable_disable() {
        let flags = FeatureFlags::new();

        assert!(!flags.is_enabled("new_feature").await.unwrap());

        flags.enable("new_feature").await.unwrap();
        assert!(flags.is_enabled("new_feature").await.unwrap());

        flags.disable("new_feature").await.unwrap();
        assert!(!flags.is_enabled("new_feature").await.unwrap());
    }

    #[tokio::test]
    async fn test_percentage_rollout() {
        let flags = FeatureFlags::new();
        flags.set_percentage("beta_feature", 50.0).await.unwrap();

        // Test multiple users to verify percentage works
        let mut enabled_count = 0;
        for i in 0..100 {
            let user_id = format!("user_{}", i);
            if flags.is_enabled_for_percentage("beta_feature", &user_id).await.unwrap() {
                enabled_count += 1;
            }
        }

        // Should be roughly 50% (allow some variance)
        assert!(enabled_count > 40 && enabled_count < 60);
    }

    #[tokio::test]
    async fn test_user_targeting() {
        let flags = FeatureFlags::new();
        flags
            .enable_for_users("premium_feature", vec!["user_1".to_string(), "user_2".to_string()])
            .await
            .unwrap();

        assert!(flags.is_enabled_for_user("premium_feature", "user_1").await.unwrap());
        assert!(flags.is_enabled_for_user("premium_feature", "user_2").await.unwrap());
        assert!(!flags.is_enabled_for_user("premium_feature", "user_3").await.unwrap());
    }

    #[tokio::test]
    async fn test_group_targeting() {
        let flags = FeatureFlags::new();
        flags
            .enable_for_groups("beta_feature", vec!["beta_testers".to_string()])
            .await
            .unwrap();

        assert!(flags.is_enabled_for_group("beta_feature", "beta_testers").await.unwrap());
        assert!(!flags.is_enabled_for_group("beta_feature", "regular_users").await.unwrap());
    }

    #[tokio::test]
    async fn test_flag_config() {
        let flags = FeatureFlags::new();
        let config = FlagConfig::new("test_flag")
            .enable()
            .percentage(25.0)
            .for_users(vec!["user_1".to_string()]);

        flags.set_config(config.clone()).await.unwrap();

        let retrieved = flags.get_config("test_flag").await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "test_flag");
        assert!(retrieved.enabled);
        assert_eq!(retrieved.percentage, Some(25.0));
        assert_eq!(retrieved.user_ids, vec!["user_1"]);
    }

    #[tokio::test]
    async fn test_list_flags() {
        let flags = FeatureFlags::new();

        flags.enable("flag_1").await.unwrap();
        flags.enable("flag_2").await.unwrap();
        flags.enable("flag_3").await.unwrap();

        let list = flags.list().await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_flag() {
        let flags = FeatureFlags::new();
        flags.enable("test_flag").await.unwrap();

        assert!(flags.is_enabled("test_flag").await.unwrap());

        flags.delete("test_flag").await.unwrap();
        assert!(!flags.is_enabled("test_flag").await.unwrap());
    }

    #[tokio::test]
    async fn test_invalid_percentage() {
        let flags = FeatureFlags::new();
        let result = flags.set_percentage("test", 150.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enabled_overrides_all() {
        let flags = FeatureFlags::new();

        // Set up a complex config
        let config = FlagConfig::new("complex_flag")
            .enable()
            .percentage(10.0)
            .for_users(vec!["user_1".to_string()]);

        flags.set_config(config).await.unwrap();

        // Even if user is not in the 10% or user_ids, enabled=true should make it available
        assert!(flags.is_enabled_for_user("complex_flag", "any_user").await.unwrap());
    }

    #[tokio::test]
    async fn test_consistent_hashing() {
        let flags = FeatureFlags::new();
        flags.set_percentage("feature", 50.0).await.unwrap();

        // Same user should get same result consistently
        let user_id = "consistent_user";
        let result1 = flags.is_enabled_for_percentage("feature", user_id).await.unwrap();
        let result2 = flags.is_enabled_for_percentage("feature", user_id).await.unwrap();
        let result3 = flags.is_enabled_for_percentage("feature", user_id).await.unwrap();

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }
}
