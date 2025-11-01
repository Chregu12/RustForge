use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use foundry_plugins::{CachePort, CommandError};
use serde_json::Value;
use tokio::sync::RwLock;

struct CacheEntry {
    value: Value,
    expires_at: Option<Instant>,
}

#[derive(Clone, Default)]
pub struct InMemoryCacheStore {
    inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

#[async_trait]
impl CachePort for InMemoryCacheStore {
    async fn get(&self, key: &str) -> Result<Option<Value>, CommandError> {
        let mut guard = self.inner.write().await;
        if let Some(entry) = guard.get(key) {
            if let Some(expiry) = entry.expires_at {
                if Instant::now() > expiry {
                    guard.remove(key);
                    return Ok(None);
                }
            }
            return Ok(Some(entry.value.clone()));
        }

        Ok(None)
    }

    async fn put(
        &self,
        key: &str,
        value: Value,
        ttl: Option<Duration>,
    ) -> Result<(), CommandError> {
        let mut guard = self.inner.write().await;
        let expires_at = ttl.map(|ttl| Instant::now() + ttl);
        guard.insert(key.to_string(), CacheEntry { value, expires_at });
        Ok(())
    }

    async fn forget(&self, key: &str) -> Result<(), CommandError> {
        let mut guard = self.inner.write().await;
        guard.remove(key);
        Ok(())
    }

    async fn clear(&self, prefix: Option<&str>) -> Result<(), CommandError> {
        let mut guard = self.inner.write().await;
        if let Some(prefix) = prefix {
            guard.retain(|key, _| !key.starts_with(prefix));
        } else {
            guard.clear();
        }
        Ok(())
    }
}
