//! Redis-backed rate limiter for distributed deployments

use crate::{LimitInfo, LimitResult, RateLimitConfig, RateLimitError, RateLimiter};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;

/// Redis-backed rate limiter
///
/// Uses Redis sorted sets to implement sliding window rate limiting
/// across multiple servers.
///
/// # Example
///
/// ```no_run
/// use rf_ratelimit::{RedisRateLimiter, RateLimitConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = RateLimitConfig::per_minute(60);
/// let limiter = RedisRateLimiter::new("redis://localhost", config).await?;
///
/// let result = limiter.check("user:123").await?;
/// if result.allowed {
///     println!("Request allowed");
/// }
/// # Ok(())
/// # }
/// ```
pub struct RedisRateLimiter {
    pool: Pool,
    config: RateLimitConfig,
}

impl RedisRateLimiter {
    /// Create new Redis rate limiter
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    /// * `config` - Rate limit configuration
    pub async fn new(redis_url: &str, config: RateLimitConfig) -> Result<Self, RateLimitError> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        // Test connection
        let mut conn = pool
            .get()
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        Ok(Self { pool, config })
    }

    /// Get Redis key for rate limit
    fn get_key(&self, key: &str) -> String {
        format!("{}:{}", self.config.key_prefix, key)
    }

    /// Get current timestamp in milliseconds
    fn now_millis() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, key: &str) -> Result<LimitResult, RateLimitError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        let redis_key = self.get_key(key);
        let now = Self::now_millis();
        let window_start = now - (self.config.window.as_millis() as i64);

        // Remove old entries outside the window
        let _: () = conn
            .zrembyscore(&redis_key, "-inf", window_start)
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        // Count entries in current window
        let count: i64 = conn
            .zcount(&redis_key, window_start, "+inf")
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        let allowed = count < self.config.max_requests as i64;

        if allowed {
            // Add current request with timestamp as score
            let _: () = conn
                .zadd(&redis_key, now, now)
                .await
                .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

            // Set expiration to window duration
            let ttl_seconds = self.config.window.as_secs() as i64;
            let _: () = conn
                .expire(&redis_key, ttl_seconds)
                .await
                .map_err(|e| RateLimitError::BackendError(e.to_string()))?;
        }

        let remaining = if allowed {
            (self.config.max_requests as i64 - count - 1).max(0) as u64
        } else {
            0
        };

        let reset_at = chrono::Utc::now() + self.config.window;
        let reset_after = self.config.window.as_secs();

        let retry_after = if !allowed {
            Some(self.config.window.as_secs())
        } else {
            None
        };

        tracing::debug!(
            key = %key,
            count = count,
            limit = self.config.max_requests,
            allowed = allowed,
            "Rate limit check (Redis)"
        );

        Ok(LimitResult {
            allowed,
            limit: self.config.max_requests,
            remaining,
            reset_after,
            reset_at,
            retry_after,
        })
    }

    async fn reset(&self, key: &str) -> Result<(), RateLimitError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        let redis_key = self.get_key(key);

        let _: () = conn
            .del(&redis_key)
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        tracing::debug!(key = %key, "Rate limit reset (Redis)");

        Ok(())
    }

    async fn info(&self, key: &str) -> Result<LimitInfo, RateLimitError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        let redis_key = self.get_key(key);
        let now = Self::now_millis();
        let window_start = now - (self.config.window.as_millis() as i64);

        // Count entries in current window
        let count: i64 = conn
            .zcount(&redis_key, window_start, "+inf")
            .await
            .map_err(|e| RateLimitError::BackendError(e.to_string()))?;

        let remaining = (self.config.max_requests as i64 - count).max(0) as u64;
        let reset_at = chrono::Utc::now() + self.config.window;

        Ok(LimitInfo {
            limit: self.config.max_requests,
            remaining,
            reset_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Note: These tests require a running Redis instance
    // Run with: docker run -d -p 6379:6379 redis

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            key_prefix: "test".to_string(),
        };

        let limiter = RedisRateLimiter::new("redis://localhost", config)
            .await
            .unwrap();

        // Reset to ensure clean state
        limiter.reset("test:user").await.unwrap();

        // Should allow first 5 requests
        for i in 0..5 {
            let result = limiter.check("test:user").await.unwrap();
            assert!(result.allowed, "Request {} should be allowed", i + 1);
            assert_eq!(result.limit, 5);
            assert_eq!(result.remaining, 4 - i);
        }

        // 6th request should be blocked
        let result = limiter.check("test:user").await.unwrap();
        assert!(!result.allowed, "6th request should be blocked");
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_reset() {
        let config = RateLimitConfig::per_minute(3);
        let limiter = RedisRateLimiter::new("redis://localhost", config)
            .await
            .unwrap();

        limiter.reset("test:reset").await.unwrap();

        // Use up limit
        for _ in 0..3 {
            limiter.check("test:reset").await.unwrap();
        }

        // Should be blocked
        let result = limiter.check("test:reset").await.unwrap();
        assert!(!result.allowed);

        // Reset
        limiter.reset("test:reset").await.unwrap();

        // Should be allowed again
        let result = limiter.check("test:reset").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_info() {
        let config = RateLimitConfig::per_minute(10);
        let limiter = RedisRateLimiter::new("redis://localhost", config)
            .await
            .unwrap();

        limiter.reset("test:info").await.unwrap();

        // Make 3 requests
        for _ in 0..3 {
            limiter.check("test:info").await.unwrap();
        }

        // Check info without incrementing
        let info = limiter.info("test:info").await.unwrap();
        assert_eq!(info.limit, 10);
        assert_eq!(info.remaining, 7);

        // Info should not have incremented
        let info2 = limiter.info("test:info").await.unwrap();
        assert_eq!(info2.remaining, 7);
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_separate_keys() {
        let config = RateLimitConfig::per_minute(2);
        let limiter = RedisRateLimiter::new("redis://localhost", config)
            .await
            .unwrap();

        limiter.reset("user:1").await.unwrap();
        limiter.reset("user:2").await.unwrap();

        // Use up limit for user:1
        limiter.check("user:1").await.unwrap();
        limiter.check("user:1").await.unwrap();

        // user:1 should be blocked
        let result = limiter.check("user:1").await.unwrap();
        assert!(!result.allowed);

        // user:2 should still be allowed
        let result = limiter.check("user:2").await.unwrap();
        assert!(result.allowed);
    }
}
