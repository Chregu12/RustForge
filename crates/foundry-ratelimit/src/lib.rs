//! # Foundry Rate Limiting
//!
//! Request and user-based rate limiting with Redis backend support.

pub mod limiter;
pub mod middleware;
pub mod storage;

pub use limiter::{RateLimit, RateLimiter};
pub use middleware::RateLimitMiddleware;
pub use storage::{MemoryStorage, RateLimitStorage};

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    LimitExceeded,

    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type Result<T> = std::result::Result<T, RateLimitError>;
