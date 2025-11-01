//! Configuration repository trait

use async_trait::async_trait;
use crate::Result;

#[async_trait]
pub trait ConfigRepository: Send + Sync {
    async fn get(&self, key: &str, environment: &str) -> Result<Option<serde_json::Value>>;
    async fn set(&self, key: String, value: serde_json::Value, environment: &str) -> Result<()>;
    async fn delete(&self, key: &str, environment: &str) -> Result<()>;
}

pub struct DatabaseConfigRepository {
    // Database connection
}

impl DatabaseConfigRepository {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for DatabaseConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConfigRepository for DatabaseConfigRepository {
    async fn get(&self, _key: &str, _environment: &str) -> Result<Option<serde_json::Value>> {
        Ok(None)
    }

    async fn set(&self, _key: String, _value: serde_json::Value, _environment: &str) -> Result<()> {
        Ok(())
    }

    async fn delete(&self, _key: &str, _environment: &str) -> Result<()> {
        Ok(())
    }
}
