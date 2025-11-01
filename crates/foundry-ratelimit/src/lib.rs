//! # Foundry Rate Limiting
//!
//! Request and user-based rate limiting with Redis backend support.

pub mod middleware;
pub mod limiter;
pub mod storage;

pub use middleware::RateLimitMiddleware;
pub use limiter::{RateLimiter, RateLimit};
pub use storage::{RateLimitStorage, MemoryStorage};

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    LimitExceeded,

    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type Result<T> = std::result::Result<T, RateLimitError>;
