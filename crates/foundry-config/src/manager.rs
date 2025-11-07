//! Configuration manager

use std::sync::{Arc, RwLock};
use crate::{ConfigCache, ConfigRepository, Result};

pub struct ConfigManager<R: ConfigRepository> {
    repository: R,
    cache: Arc<RwLock<ConfigCache>>,
    environment: String,
}

impl<R: ConfigRepository> ConfigManager<R> {
    pub fn new(repository: R, environment: String) -> Self {
        Self {
            repository,
            cache: Arc::new(RwLock::new(ConfigCache::new())),
            environment,
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        // Check cache first
        if let Some(value) = self.cache.read().unwrap().get(key) {
            return Ok(Some(value.clone()));
        }

        // Load from repository
        let value = self.repository.get(key, &self.environment).await?;

        // Cache it
        if let Some(ref v) = value {
            self.cache.write().unwrap().set(key.to_string(), v.clone());
        }

        Ok(value)
    }

    pub async fn set(&self, key: String, value: serde_json::Value) -> Result<()> {
        self.repository.set(key.clone(), value.clone(), &self.environment).await?;
        self.cache.write().unwrap().set(key, value);
        Ok(())
    }

    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }
}
