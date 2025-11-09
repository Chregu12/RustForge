//! API Rate Limiting for RustForge
//!
//! Provides production-ready rate limiting with multiple backends and Axum integration.
//!
//! # Features
//!
//! - Sliding window algorithm
//! - Memory backend for development/testing
//! - Redis backend for production (optional feature)
//! - Axum middleware integration
//! - Rate limit headers (X-RateLimit-*)
//!
//! # Quick Start
//!
//! ```
//! use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimiter};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create rate limiter (60 requests per minute)
//! let config = RateLimitConfig::per_minute(60);
//! let limiter = MemoryRateLimiter::new(config);
//!
//! // Check rate limit
//! let result = limiter.check("user:123").await?;
//!
//! if result.allowed {
//!     println!("Request allowed! {} remaining", result.remaining);
//! } else {
//!     println!("Rate limit exceeded!");
//! }
//! # Ok(())
//! # }
//! ```

mod config;
mod error;
mod limiter;
mod memory;
pub mod middleware;

#[cfg(feature = "redis-backend")]
mod redis;

pub use config::RateLimitConfig;
pub use error::{RateLimitError, RateLimitResult};
pub use limiter::{LimitInfo, LimitResult, RateLimiter};
pub use memory::MemoryRateLimiter;
pub use middleware::RateLimitLayer;

#[cfg(feature = "redis-backend")]
pub use redis::RedisRateLimiter;
