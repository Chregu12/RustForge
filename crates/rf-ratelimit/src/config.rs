//! Rate limit configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum number of requests
    pub max_requests: u64,

    /// Time window
    #[serde(with = "humantime_serde")]
    pub window: Duration,

    /// Key prefix for storage
    pub key_prefix: String,
}

impl RateLimitConfig {
    /// Create config for requests per minute
    pub fn per_minute(max_requests: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(60),
            key_prefix: "ratelimit".into(),
        }
    }

    /// Create config for requests per hour
    pub fn per_hour(max_requests: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(3600),
            key_prefix: "ratelimit".into(),
        }
    }

    /// Create config for requests per second
    pub fn per_second(max_requests: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(1),
            key_prefix: "ratelimit".into(),
        }
    }

    /// Custom time window
    pub fn custom(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            key_prefix: "ratelimit".into(),
        }
    }

    /// Set key prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::per_minute(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_minute() {
        let config = RateLimitConfig::per_minute(100);
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window.as_secs(), 60);
    }

    #[test]
    fn test_per_hour() {
        let config = RateLimitConfig::per_hour(1000);
        assert_eq!(config.max_requests, 1000);
        assert_eq!(config.window.as_secs(), 3600);
    }

    #[test]
    fn test_custom() {
        let config = RateLimitConfig::custom(50, Duration::from_secs(30));
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window.as_secs(), 30);
    }
}
