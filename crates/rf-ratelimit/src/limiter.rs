//! Rate limiter trait and result types

use crate::RateLimitError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Rate limiter backend trait
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Check if request is allowed and update counter
    async fn check(&self, key: &str) -> Result<LimitResult, RateLimitError>;

    /// Reset rate limit for key
    async fn reset(&self, key: &str) -> Result<(), RateLimitError>;

    /// Get current limit info without incrementing
    async fn info(&self, key: &str) -> Result<LimitInfo, RateLimitError>;
}

/// Result of rate limit check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitResult {
    /// Whether request is allowed
    pub allowed: bool,

    /// Maximum requests allowed in window
    pub limit: u64,

    /// Remaining requests in current window
    pub remaining: u64,

    /// Time until window resets (seconds)
    pub reset_after: u64,

    /// Timestamp when window resets
    pub reset_at: chrono::DateTime<chrono::Utc>,

    /// Time to wait before retry (if not allowed)
    pub retry_after: Option<u64>,
}

/// Rate limit information
#[derive(Debug, Clone)]
pub struct LimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset_at: chrono::DateTime<chrono::Utc>,
}
