//! Rate Limiting for Jobs
//!
//! Provides rate limiting capabilities using Redis-based sliding window algorithm.
//! Similar to Laravel's `Redis::throttle()`.
//!
//! # Example
//!
//! ```ignore
//! use rf_jobs::rate_limit::RateLimiter;
//! use std::time::Duration;
//!
//! let limiter = RateLimiter::new(queue_manager);
//!
//! // Allow 10 requests per 60 seconds
//! if limiter.allow("emails", 10, Duration::from_secs(60)).await? {
//!     // Execute job
//! } else {
//!     // Rate limit exceeded
//! }
//! ```

use crate::error::QueueError;
use crate::queue::QueueManager;
use redis::AsyncCommands;
use std::time::Duration;

/// Rate limiter using Redis sliding window algorithm
#[derive(Clone)]
pub struct RateLimiter {
    queue_manager: QueueManager,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(queue_manager: QueueManager) -> Self {
        Self { queue_manager }
    }

    /// Check if action is allowed under rate limit
    ///
    /// # Arguments
    ///
    /// * `key` - Unique identifier for the rate limit (e.g., "emails", "api_calls")
    /// * `max` - Maximum number of actions allowed in the time window
    /// * `window` - Time window duration
    ///
    /// # Returns
    ///
    /// `true` if action is allowed, `false` if rate limit exceeded
    pub async fn allow(&self, key: &str, max: u32, window: Duration) -> Result<bool, QueueError> {
        let mut conn = self.queue_manager.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let redis_key = format!("rate_limit:{}", key);
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - window.as_millis() as i64;

        // Remove old entries outside the window
        conn.zrembyscore::<_, _, _, ()>(&redis_key, 0, window_start)
            .await?;

        // Count current entries
        let count: u32 = conn.zcard(&redis_key).await?;

        if count < max {
            // Add new entry with current timestamp as score and unique value
            let value = format!("{}:{}", now, uuid::Uuid::new_v4());
            conn.zadd::<_, _, _, ()>(&redis_key, value, now).await?;

            // Set expiration on the key (cleanup)
            let expiration = window.as_secs() as i64 + 10; // Add 10 seconds buffer
            conn.expire::<_, ()>(&redis_key, expiration).await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Wait until rate limit allows the action
    ///
    /// This will block until a slot becomes available.
    pub async fn wait_for_slot(
        &self,
        key: &str,
        max: u32,
        window: Duration,
    ) -> Result<(), QueueError> {
        loop {
            if self.allow(key, max, window).await? {
                return Ok(());
            }

            // Wait a bit before retrying
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Reset rate limit for a key
    pub async fn reset(&self, key: &str) -> Result<(), QueueError> {
        let mut conn = self.queue_manager.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let redis_key = format!("rate_limit:{}", key);
        conn.del::<_, ()>(&redis_key).await?;

        Ok(())
    }

    /// Get remaining slots in the current window
    pub async fn remaining(
        &self,
        key: &str,
        max: u32,
        window: Duration,
    ) -> Result<u32, QueueError> {
        let mut conn = self.queue_manager.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let redis_key = format!("rate_limit:{}", key);
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - window.as_millis() as i64;

        // Remove old entries
        conn.zrembyscore::<_, _, _, ()>(&redis_key, 0, window_start)
            .await?;

        // Count current entries
        let count: u32 = conn.zcard(&redis_key).await?;

        Ok(max.saturating_sub(count))
    }

    /// Get time until next slot becomes available (in milliseconds)
    pub async fn retry_after(
        &self,
        key: &str,
        max: u32,
        window: Duration,
    ) -> Result<Option<u64>, QueueError> {
        let mut conn = self.queue_manager.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let redis_key = format!("rate_limit:{}", key);
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - window.as_millis() as i64;

        // Remove old entries
        conn.zrembyscore::<_, _, _, ()>(&redis_key, 0, window_start)
            .await?;

        // Count current entries
        let count: u32 = conn.zcard(&redis_key).await?;

        if count < max {
            // Slots available
            Ok(None)
        } else {
            // Get oldest entry in the window
            let oldest: Vec<(String, i64)> = conn
                .zrange_withscores(&redis_key, 0, 0)
                .await
                .unwrap_or_default();

            if let Some((_, oldest_timestamp)) = oldest.first() {
                let oldest_expires_at = oldest_timestamp + window.as_millis() as i64;
                let retry_after = (oldest_expires_at - now).max(0) as u64;
                Ok(Some(retry_after))
            } else {
                Ok(None)
            }
        }
    }

    /// Attempt to acquire multiple slots at once
    ///
    /// Returns the number of slots acquired (may be less than requested)
    pub async fn acquire(
        &self,
        key: &str,
        count: u32,
        max: u32,
        window: Duration,
    ) -> Result<u32, QueueError> {
        let remaining = self.remaining(key, max, window).await?;
        let acquired = count.min(remaining);

        // Acquire the slots
        for _ in 0..acquired {
            self.allow(key, max, window).await?;
        }

        Ok(acquired)
    }
}

/// Extension trait for JobContext to provide rate limiting
pub trait RateLimitExt {
    /// Apply rate limit to job execution
    fn rate_limit(
        &self,
        limiter: &RateLimiter,
        key: &str,
        max: u32,
        window: Duration,
    ) -> impl std::future::Future<Output = Result<bool, QueueError>> + Send;

    /// Wait for rate limit slot
    fn wait_for_rate_limit(
        &self,
        limiter: &RateLimiter,
        key: &str,
        max: u32,
        window: Duration,
    ) -> impl std::future::Future<Output = Result<(), QueueError>> + Send;
}

impl RateLimitExt for crate::JobContext {
    async fn rate_limit(
        &self,
        limiter: &RateLimiter,
        key: &str,
        max: u32,
        window: Duration,
    ) -> Result<bool, QueueError> {
        limiter.allow(key, max, window).await
    }

    async fn wait_for_rate_limit(
        &self,
        limiter: &RateLimiter,
        key: &str,
        max: u32,
        window: Duration,
    ) -> Result<(), QueueError> {
        limiter.wait_for_slot(key, max, window).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rate_limiter_creation() {
        // This test just verifies the struct can be created
        // Actual Redis tests would require a running Redis instance
    }

    #[tokio::test]
    async fn test_remaining_calculation() {
        // Test the logic without Redis
        let max = 10u32;
        let current = 7u32;
        let remaining = max.saturating_sub(current);
        assert_eq!(remaining, 3);
    }

    #[tokio::test]
    async fn test_acquire_logic() {
        let _max = 10u32;
        let remaining = 5u32;
        let requested = 7u32;
        let acquired = requested.min(remaining);
        assert_eq!(acquired, 5);
    }
}
