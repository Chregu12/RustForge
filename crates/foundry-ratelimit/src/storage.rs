//! Rate limit storage backends

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use crate::Result;

#[async_trait]
pub trait RateLimitStorage: Send + Sync {
    async fn increment(&self, key: &str, window: Duration) -> Result<u32>;
    async fn get(&self, key: &str) -> Result<u32>;
    async fn reset(&self, key: &str) -> Result<()>;
}

#[derive(Clone)]
struct RateLimitEntry {
    count: u32,
    expires_at: Instant,
}

pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn cleanup(&self) {
        let now = Instant::now();
        self.data.write().unwrap().retain(|_, entry| entry.expires_at > now);
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RateLimitStorage for MemoryStorage {
    async fn increment(&self, key: &str, window: Duration) -> Result<u32> {
        self.cleanup();
        let mut data = self.data.write().unwrap();
        let now = Instant::now();

        let entry = data.entry(key.to_string()).or_insert_with(|| RateLimitEntry {
            count: 0,
            expires_at: now + window,
        });

        if entry.expires_at <= now {
            entry.count = 0;
            entry.expires_at = now + window;
        }

        entry.count += 1;
        Ok(entry.count)
    }

    async fn get(&self, key: &str) -> Result<u32> {
        let data = self.data.read().unwrap();
        Ok(data.get(key).map(|e| e.count).unwrap_or(0))
    }

    async fn reset(&self, key: &str) -> Result<()> {
        self.data.write().unwrap().remove(key);
        Ok(())
    }
}
