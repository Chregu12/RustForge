//! Rate limiter implementation

use std::time::Duration;
use crate::{RateLimitStorage, Result};

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub max_requests: u32,
    pub window: Duration,
}

impl RateLimit {
    pub fn per_minute(max_requests: u32) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(60),
        }
    }

    pub fn per_hour(max_requests: u32) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(3600),
        }
    }
}

pub struct RateLimiter<S: RateLimitStorage> {
    storage: S,
    limit: RateLimit,
}

impl<S: RateLimitStorage> RateLimiter<S> {
    pub fn new(storage: S, limit: RateLimit) -> Self {
        Self { storage, limit }
    }

    pub async fn check(&self, key: &str) -> Result<bool> {
        let count = self.storage.increment(key, self.limit.window).await?;
        Ok(count <= self.limit.max_requests)
    }

    pub async fn remaining(&self, key: &str) -> Result<u32> {
        let count = self.storage.get(key).await?;
        Ok(self.limit.max_requests.saturating_sub(count))
    }
}
