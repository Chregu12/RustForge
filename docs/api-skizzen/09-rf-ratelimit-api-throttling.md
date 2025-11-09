# API-Skizze: rf-ratelimit - API Rate Limiting

**Phase**: Phase 3 - Advanced Features
**PR-Slice**: #11
**Status**: Planning
**Date**: 2025-11-09

## 1. Overview

The `rf-ratelimit` crate provides production-ready API rate limiting with multiple algorithms, Redis backend support, and seamless Axum integration.

**Key Features:**
- Multiple algorithms (Sliding Window, Token Bucket, Fixed Window)
- Redis-backed distributed rate limiting
- In-memory rate limiting for single-server deployments
- Middleware integration with Axum
- Custom rate limit headers (X-RateLimit-*)
- Per-route and global rate limiting
- IP-based and user-based limiting

**Comparison with Laravel:**
- ✅ Throttle middleware
- ✅ Rate limit headers
- ✅ Per-route limits
- ✅ Redis backend
- ✅ Custom rate limit keys
- ⏳ Named rate limiters (future)

## 2. Core Types

### 2.1 Rate Limiter Trait

```rust
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Check if request is allowed and update counter
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError>;

    /// Reset rate limit for key
    async fn reset(&self, key: &str) -> Result<(), RateLimitError>;

    /// Get current limit info
    async fn info(&self, key: &str) -> Result<RateLimitInfo, RateLimitError>;
}

#[derive(Debug, Clone)]
pub struct RateLimitResult {
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

#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub reset_at: chrono::DateTime<chrono::Utc>,
}
```

### 2.2 Rate Limit Configuration

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum number of requests
    pub max_requests: u64,

    /// Time window
    pub window: Duration,

    /// Rate limit algorithm
    pub algorithm: RateLimitAlgorithm,

    /// Key prefix for storage
    pub key_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RateLimitAlgorithm {
    /// Sliding window (most accurate)
    SlidingWindow,

    /// Fixed window (simpler, less accurate)
    FixedWindow,

    /// Token bucket (smooths bursts)
    TokenBucket,
}

impl RateLimitConfig {
    /// Create config for requests per minute
    pub fn per_minute(max_requests: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(60),
            algorithm: RateLimitAlgorithm::SlidingWindow,
            key_prefix: "ratelimit".into(),
        }
    }

    /// Create config for requests per hour
    pub fn per_hour(max_requests: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(3600),
            algorithm: RateLimitAlgorithm::SlidingWindow,
            key_prefix: "ratelimit".into(),
        }
    }

    /// Custom time window
    pub fn custom(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            algorithm: RateLimitAlgorithm::SlidingWindow,
            key_prefix: "ratelimit".into(),
        }
    }
}
```

## 3. Backend Implementations

### 3.1 Redis Backend (Production)

```rust
use redis::AsyncCommands;

pub struct RedisRateLimiter {
    pool: deadpool_redis::Pool,
    config: RateLimitConfig,
}

impl RedisRateLimiter {
    pub async fn new(redis_url: &str, config: RateLimitConfig) -> Result<Self, RateLimitError> {
        let cfg = deadpool_redis::Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .map_err(|e| RateLimitError::ConnectionError(e.to_string()))?;

        Ok(Self { pool, config })
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let full_key = format!("{}:{}", self.config.key_prefix, key);
        let now = chrono::Utc::now();

        match self.config.algorithm {
            RateLimitAlgorithm::SlidingWindow => {
                self.sliding_window_check(&full_key, now).await
            }
            RateLimitAlgorithm::FixedWindow => {
                self.fixed_window_check(&full_key, now).await
            }
            RateLimitAlgorithm::TokenBucket => {
                self.token_bucket_check(&full_key, now).await
            }
        }
    }

    async fn reset(&self, key: &str) -> Result<(), RateLimitError> {
        let full_key = format!("{}:{}", self.config.key_prefix, key);
        let mut conn = self.pool.get().await?;
        conn.del(&full_key).await?;
        Ok(())
    }

    async fn info(&self, key: &str) -> Result<RateLimitInfo, RateLimitError> {
        let result = self.check(key).await?;
        Ok(RateLimitInfo {
            limit: result.limit,
            remaining: result.remaining,
            reset_at: result.reset_at,
        })
    }
}

impl RedisRateLimiter {
    /// Sliding window algorithm using sorted sets
    async fn sliding_window_check(
        &self,
        key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut conn = self.pool.get().await?;

        let window_start = now - chrono::Duration::from_std(self.config.window)
            .map_err(|_| RateLimitError::InvalidConfig)?;

        let now_ts = now.timestamp_millis();
        let window_start_ts = window_start.timestamp_millis();

        // Remove old entries
        let _: () = conn.zrembyscore(key, 0, window_start_ts).await?;

        // Count current requests in window
        let count: u64 = conn.zcard(key).await?;

        let allowed = count < self.config.max_requests;

        if allowed {
            // Add current request
            let _: () = conn.zadd(key, now_ts, now_ts).await?;

            // Set expiry
            let _: () = conn.expire(key, self.config.window.as_secs() as i64).await?;
        }

        let remaining = if count >= self.config.max_requests {
            0
        } else {
            self.config.max_requests - count - 1
        };

        let reset_at = now + chrono::Duration::from_std(self.config.window)
            .map_err(|_| RateLimitError::InvalidConfig)?;

        let retry_after = if !allowed {
            Some(self.config.window.as_secs())
        } else {
            None
        };

        Ok(RateLimitResult {
            allowed,
            limit: self.config.max_requests,
            remaining,
            reset_after: self.config.window.as_secs(),
            reset_at,
            retry_after,
        })
    }

    /// Fixed window algorithm
    async fn fixed_window_check(
        &self,
        key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<RateLimitResult, RateLimitError> {
        let mut conn = self.pool.get().await?;

        // Calculate window key
        let window_key = format!("{}:{}", key, now.timestamp() / self.config.window.as_secs() as i64);

        // Increment counter
        let count: u64 = conn.incr(&window_key, 1).await?;

        // Set expiry on first request
        if count == 1 {
            let _: () = conn.expire(&window_key, self.config.window.as_secs() as i64).await?;
        }

        let allowed = count <= self.config.max_requests;

        let remaining = if count >= self.config.max_requests {
            0
        } else {
            self.config.max_requests - count
        };

        let window_end = now.timestamp() / self.config.window.as_secs() as i64 * self.config.window.as_secs() as i64
            + self.config.window.as_secs() as i64;

        let reset_at = chrono::DateTime::from_timestamp(window_end, 0)
            .unwrap_or(now + chrono::Duration::from_std(self.config.window)
                .map_err(|_| RateLimitError::InvalidConfig)?);

        Ok(RateLimitResult {
            allowed,
            limit: self.config.max_requests,
            remaining,
            reset_after: (reset_at - now).num_seconds() as u64,
            reset_at,
            retry_after: if !allowed { Some((reset_at - now).num_seconds() as u64) } else { None },
        })
    }

    /// Token bucket algorithm (simplified)
    async fn token_bucket_check(
        &self,
        key: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<RateLimitResult, RateLimitError> {
        // For now, use sliding window (token bucket requires more complex state)
        self.sliding_window_check(key, now).await
    }
}
```

### 3.2 Memory Backend (Development/Testing)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MemoryRateLimiter {
    state: Arc<Mutex<HashMap<String, Vec<i64>>>>,
    config: RateLimitConfig,
}

impl MemoryRateLimiter {
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
}

#[async_trait]
impl RateLimiter for MemoryRateLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let full_key = format!("{}:{}", self.config.key_prefix, key);
        let now = chrono::Utc::now();
        let window_start = now - chrono::Duration::from_std(self.config.window)
            .map_err(|_| RateLimitError::InvalidConfig)?;

        let mut state = self.state.lock().unwrap();
        let timestamps = state.entry(full_key.clone()).or_insert_with(Vec::new);

        // Remove old timestamps
        timestamps.retain(|&ts| ts > window_start.timestamp_millis());

        let count = timestamps.len() as u64;
        let allowed = count < self.config.max_requests;

        if allowed {
            timestamps.push(now.timestamp_millis());
        }

        let remaining = if count >= self.config.max_requests {
            0
        } else {
            self.config.max_requests - count - (if allowed { 1 } else { 0 })
        };

        let reset_at = now + chrono::Duration::from_std(self.config.window)
            .map_err(|_| RateLimitError::InvalidConfig)?;

        Ok(RateLimitResult {
            allowed,
            limit: self.config.max_requests,
            remaining,
            reset_after: self.config.window.as_secs(),
            reset_at,
            retry_after: if !allowed { Some(self.config.window.as_secs()) } else { None },
        })
    }

    async fn reset(&self, key: &str) -> Result<(), RateLimitError> {
        let full_key = format!("{}:{}", self.config.key_prefix, key);
        self.state.lock().unwrap().remove(&full_key);
        Ok(())
    }

    async fn info(&self, key: &str) -> Result<RateLimitInfo, RateLimitError> {
        let result = self.check(key).await?;
        Ok(RateLimitInfo {
            limit: result.limit,
            remaining: result.remaining,
            reset_at: result.reset_at,
        })
    }
}
```

## 4. Axum Middleware Integration

### 4.1 Rate Limit Layer

```rust
use axum::{
    extract::{Request, ConnectInfo},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

pub struct RateLimitLayer {
    limiter: Arc<dyn RateLimiter>,
    key_extractor: Box<dyn Fn(&Request) -> String + Send + Sync>,
}

impl RateLimitLayer {
    /// Create layer with IP-based limiting
    pub fn new(limiter: Arc<dyn RateLimiter>) -> Self {
        Self {
            limiter,
            key_extractor: Box::new(|req| {
                req.extensions()
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ci| ci.0.ip().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            }),
        }
    }

    /// Custom key extraction function
    pub fn with_key_extractor<F>(mut self, extractor: F) -> Self
    where
        F: Fn(&Request) -> String + Send + Sync + 'static,
    {
        self.key_extractor = Box::new(extractor);
        self
    }

    /// Middleware handler
    pub async fn handle(
        &self,
        req: Request,
        next: Next,
    ) -> Response {
        let key = (self.key_extractor)(&req);

        match self.limiter.check(&key).await {
            Ok(result) => {
                if result.allowed {
                    // Add rate limit headers
                    let mut response = next.run(req).await;
                    add_rate_limit_headers(response.headers_mut(), &result);
                    response
                } else {
                    // Rate limit exceeded
                    rate_limit_exceeded_response(&result)
                }
            }
            Err(e) => {
                tracing::error!("Rate limit check failed: {}", e);
                // On error, allow request but log
                next.run(req).await
            }
        }
    }
}

fn add_rate_limit_headers(headers: &mut HeaderMap, result: &RateLimitResult) {
    headers.insert("X-RateLimit-Limit", result.limit.to_string().parse().unwrap());
    headers.insert("X-RateLimit-Remaining", result.remaining.to_string().parse().unwrap());
    headers.insert("X-RateLimit-Reset", result.reset_at.timestamp().to_string().parse().unwrap());
}

fn rate_limit_exceeded_response(result: &RateLimitResult) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        serde_json::json!({
            "error": "Rate limit exceeded",
            "message": "Too many requests. Please try again later.",
            "retry_after": result.retry_after,
        }).to_string(),
    ).into_response();

    add_rate_limit_headers(response.headers_mut(), result);

    if let Some(retry_after) = result.retry_after {
        response.headers_mut().insert(
            "Retry-After",
            retry_after.to_string().parse().unwrap(),
        );
    }

    response
}
```

## 5. Usage Examples

### 5.1 Basic Usage

```rust
use rf_ratelimit::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create rate limiter (100 requests per minute)
    let config = RateLimitConfig::per_minute(100);
    let limiter = MemoryRateLimiter::new(config);

    // Check rate limit
    let result = limiter.check("user:123").await?;

    if result.allowed {
        println!("Request allowed! {} remaining", result.remaining);
    } else {
        println!("Rate limit exceeded! Retry after {} seconds", result.retry_after.unwrap());
    }

    Ok(())
}
```

### 5.2 Axum Integration

```rust
use axum::{Router, routing::get};
use rf_ratelimit::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create rate limiter
    let config = RateLimitConfig::per_minute(60);
    let limiter = Arc::new(RedisRateLimiter::new("redis://localhost:6379", config).await?);

    // Create router with rate limiting
    let app = Router::new()
        .route("/api/users", get(get_users))
        .layer(axum::middleware::from_fn(move |req, next| {
            let limiter = Arc::clone(&limiter);
            async move {
                RateLimitLayer::new(limiter).handle(req, next).await
            }
        }));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### 5.3 User-based Rate Limiting

```rust
// Extract user ID from JWT token
let layer = RateLimitLayer::new(limiter)
    .with_key_extractor(|req| {
        req.extensions()
            .get::<User>()
            .map(|user| format!("user:{}", user.id))
            .unwrap_or_else(|| "anonymous".to_string())
    });
```

### 5.4 Per-Route Rate Limiting

```rust
// Different limits for different routes
let router = Router::new()
    .route("/api/search", get(search)
        .layer(rate_limit_layer(10, Duration::from_secs(60))))
    .route("/api/users", get(get_users)
        .layer(rate_limit_layer(100, Duration::from_secs(60))));

fn rate_limit_layer(max: u64, window: Duration) -> impl Layer {
    let config = RateLimitConfig::custom(max, window);
    let limiter = Arc::new(MemoryRateLimiter::new(config));
    axum::middleware::from_fn(move |req, next| {
        let limiter = Arc::clone(&limiter);
        async move {
            RateLimitLayer::new(limiter).handle(req, next).await
        }
    })
}
```

## 6. Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Pool error: {0}")]
    PoolError(#[from] deadpool_redis::PoolError),

    #[error("Invalid configuration")]
    InvalidConfig,

    #[error("Rate limit error: {0}")]
    Other(String),
}
```

## 7. Testing

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_rate_limiter() {
        let config = RateLimitConfig::per_minute(5);
        let limiter = MemoryRateLimiter::new(config);

        // First 5 requests should be allowed
        for _ in 0..5 {
            let result = limiter.check("test").await.unwrap();
            assert!(result.allowed);
        }

        // 6th request should be denied
        let result = limiter.check("test").await.unwrap();
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limit_reset() {
        let config = RateLimitConfig::per_minute(3);
        let limiter = MemoryRateLimiter::new(config);

        // Use up limit
        for _ in 0..3 {
            limiter.check("test").await.unwrap();
        }

        // Reset
        limiter.reset("test").await.unwrap();

        // Should be allowed again
        let result = limiter.check("test").await.unwrap();
        assert!(result.allowed);
    }
}
```

## 8. Implementation Plan

### Phase 1: Core (2 hours)
- [ ] Create rf-ratelimit crate
- [ ] Implement RateLimiter trait
- [ ] Implement RateLimitConfig
- [ ] Add error types

### Phase 2: Backends (1.5 hours)
- [ ] Implement MemoryRateLimiter
- [ ] Implement RedisRateLimiter (sliding window)
- [ ] Add fixed window algorithm
- [ ] Test both backends

### Phase 3: Middleware (1 hour)
- [ ] Implement RateLimitLayer
- [ ] Add header support
- [ ] Add key extraction
- [ ] Test middleware

### Phase 4: Examples & Tests (0.5 hours)
- [ ] Create ratelimit-demo example
- [ ] Write 15+ unit tests
- [ ] Integration tests
- [ ] Documentation

**Total: 3-4 hours**

## 9. Dependencies

```toml
[dependencies]
async-trait.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
axum.workspace = true

# Redis support
redis = { workspace = true, optional = true }
deadpool-redis = { workspace = true, optional = true }

[features]
default = ["redis-backend"]
redis-backend = ["redis", "deadpool-redis"]
```

## 10. Laravel Comparison

| Feature | Laravel | rf-ratelimit | Status |
|---------|---------|--------------|--------|
| Throttle middleware | ✅ | ✅ | ✅ Complete |
| Per-route limits | ✅ | ✅ | ✅ Complete |
| Redis backend | ✅ | ✅ | ✅ Complete |
| Custom keys | ✅ | ✅ | ✅ Complete |
| Rate limit headers | ✅ | ✅ | ✅ Complete |
| Multiple algorithms | ⏳ | ✅ | ✅ Better than Laravel |
| Named limiters | ✅ | ⏳ | ⏳ Future |

**Feature Parity**: ~85% (6/7 features, plus extras)

---

**Estimated Lines**: ~800 production + ~200 tests + ~150 examples = **~1,150 total**
