//! In-memory rate limiter for development and testing

use crate::{LimitInfo, LimitResult, RateLimitConfig, RateLimitError, RateLimiter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory rate limiter
///
/// Uses sliding window algorithm with timestamps stored in memory.
/// Not suitable for distributed systems - use RedisRateLimiter for production.
#[derive(Clone)]
pub struct MemoryRateLimiter {
    state: Arc<Mutex<HashMap<String, Vec<i64>>>>,
    config: RateLimitConfig,
}

impl MemoryRateLimiter {
    /// Create new memory rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Clear all state (for testing)
    pub fn clear(&self) {
        self.state.lock().unwrap().clear();
    }

    /// Get number of tracked keys
    pub fn key_count(&self) -> usize {
        self.state.lock().unwrap().len()
    }
}

#[async_trait]
impl RateLimiter for MemoryRateLimiter {
    async fn check(&self, key: &str) -> Result<LimitResult, RateLimitError> {
        let full_key = format!("{}:{}", self.config.key_prefix, key);
        let now = chrono::Utc::now();
        let window_start = now
            - chrono::Duration::from_std(self.config.window)
                .map_err(|_| RateLimitError::InvalidConfig("Invalid window duration".into()))?;

        let mut state = self.state.lock().unwrap();
        let timestamps = state.entry(full_key.clone()).or_insert_with(Vec::new);

        // Remove old timestamps outside window
        timestamps.retain(|&ts| ts > window_start.timestamp_millis());

        let count = timestamps.len() as u64;
        let allowed = count < self.config.max_requests;

        if allowed {
            // Add current timestamp
            timestamps.push(now.timestamp_millis());
        }

        let remaining = if count >= self.config.max_requests {
            0
        } else {
            self.config.max_requests - count - (if allowed { 1 } else { 0 })
        };

        let reset_at = now
            + chrono::Duration::from_std(self.config.window)
                .map_err(|_| RateLimitError::InvalidConfig("Invalid window duration".into()))?;

        let retry_after = if !allowed {
            Some(self.config.window.as_secs())
        } else {
            None
        };

        tracing::debug!(
            key = %key,
            allowed = %allowed,
            remaining = %remaining,
            count = %count,
            "Rate limit check"
        );

        Ok(LimitResult {
            allowed,
            limit: self.config.max_requests,
            remaining,
            reset_after: self.config.window.as_secs(),
            reset_at,
            retry_after,
        })
    }

    async fn reset(&self, key: &str) -> Result<(), RateLimitError> {
        let full_key = format!("{}:{}", self.config.key_prefix, key);
        self.state.lock().unwrap().remove(&full_key);

        tracing::debug!(key = %key, "Rate limit reset");

        Ok(())
    }

    async fn info(&self, key: &str) -> Result<LimitInfo, RateLimitError> {
        let result = self.check(key).await?;
        Ok(LimitInfo {
            limit: result.limit,
            remaining: result.remaining,
            reset_at: result.reset_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_rate_limiter_allows_within_limit() {
        let config = RateLimitConfig::per_minute(5);
        let limiter = MemoryRateLimiter::new(config);

        // First 5 requests should be allowed
        for i in 0..5 {
            let result = limiter.check("test").await.unwrap();
            assert!(result.allowed, "Request {} should be allowed", i + 1);
            assert_eq!(result.limit, 5);
            assert_eq!(result.remaining, 4 - i);
        }
    }

    #[tokio::test]
    async fn test_memory_rate_limiter_blocks_over_limit() {
        let config = RateLimitConfig::per_minute(3);
        let limiter = MemoryRateLimiter::new(config);

        // Use up limit
        for _ in 0..3 {
            limiter.check("test").await.unwrap();
        }

        // 4th request should be denied
        let result = limiter.check("test").await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    async fn test_memory_rate_limiter_reset() {
        let config = RateLimitConfig::per_minute(2);
        let limiter = MemoryRateLimiter::new(config);

        // Use up limit
        for _ in 0..2 {
            limiter.check("test").await.unwrap();
        }

        // Should be blocked
        let result = limiter.check("test").await.unwrap();
        assert!(!result.allowed);

        // Reset
        limiter.reset("test").await.unwrap();

        // Should be allowed again
        let result = limiter.check("test").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_memory_rate_limiter_separate_keys() {
        let config = RateLimitConfig::per_minute(2);
        let limiter = MemoryRateLimiter::new(config);

        // Use up limit for key1
        for _ in 0..2 {
            limiter.check("key1").await.unwrap();
        }

        // key1 should be blocked
        let result = limiter.check("key1").await.unwrap();
        assert!(!result.allowed);

        // key2 should still be allowed
        let result = limiter.check("key2").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_memory_rate_limiter_clear() {
        let config = RateLimitConfig::per_minute(2);
        let limiter = MemoryRateLimiter::new(config);

        limiter.check("key1").await.unwrap();
        limiter.check("key2").await.unwrap();

        assert_eq!(limiter.key_count(), 2);

        limiter.clear();

        assert_eq!(limiter.key_count(), 0);
    }

    #[tokio::test]
    async fn test_memory_rate_limiter_info() {
        let config = RateLimitConfig::per_minute(10);
        let limiter = MemoryRateLimiter::new(config);

        // Make some requests
        for _ in 0..3 {
            limiter.check("test").await.unwrap();
        }

        // Get info (doesn't increment)
        let info = limiter.info("test").await.unwrap();
        assert_eq!(info.limit, 10);
        assert_eq!(info.remaining, 6); // 10 - 3 - 1 (from info check)
    }
}
