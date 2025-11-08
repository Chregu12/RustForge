//! Rate Limiting Middleware
//!
//! Provides flexible rate limiting with multiple strategies and storage backends.
//!
//! # Features
//! - Multiple strategies: Per IP, Per User, Per Route, Custom
//! - Configurable limits (requests per minute/hour)
//! - Pluggable storage backends (in-memory, Redis)
//! - Proper HTTP 429 responses with Retry-After header
//! - Exemption/whitelist support
//! - Rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset)
//!
//! # Example
//! ```rust,ignore
//! use foundry_application::middleware::rate_limit::{RateLimitMiddleware, RateLimitConfig};
//!
//! let limiter = RateLimitMiddleware::new(
//!     RateLimitConfig::per_ip(60) // 60 requests per minute
//!         .exempt("/health")
//!         .exempt("/metrics")
//! );
//! ```

use axum::{
    body::Body,
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Rate limit strategy
#[derive(Debug, Clone, PartialEq)]
pub enum RateLimitStrategy {
    /// Limit by IP address
    PerIp,
    /// Limit by user ID (from auth)
    PerUser,
    /// Limit by route path
    PerRoute,
    /// Custom key extraction function
    Custom(Arc<dyn Fn(&Request<Body>) -> String + Send + Sync>),
}

/// Rate limit window configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitWindow {
    /// Number of requests allowed
    pub limit: u32,
    /// Time window in seconds
    pub window_secs: u64,
}

impl RateLimitWindow {
    /// Create a per-minute limit
    pub fn per_minute(limit: u32) -> Self {
        Self {
            limit,
            window_secs: 60,
        }
    }

    /// Create a per-hour limit
    pub fn per_hour(limit: u32) -> Self {
        Self {
            limit,
            window_secs: 3600,
        }
    }

    /// Create a custom window
    pub fn custom(limit: u32, window_secs: u64) -> Self {
        Self { limit, window_secs }
    }
}

/// Rate limit record for a key
#[derive(Debug, Clone)]
struct RateLimitRecord {
    /// Number of requests made in current window
    count: u32,
    /// Window start time (Unix timestamp)
    window_start: u64,
}

impl RateLimitRecord {
    fn new(now: u64) -> Self {
        Self {
            count: 1,
            window_start: now,
        }
    }

    fn is_expired(&self, now: u64, window_secs: u64) -> bool {
        now - self.window_start >= window_secs
    }

    fn increment(&mut self, now: u64, window_secs: u64) -> bool {
        if self.is_expired(now, window_secs) {
            // Reset window
            self.count = 1;
            self.window_start = now;
            true
        } else {
            self.count += 1;
            true
        }
    }
}

/// Storage backend for rate limit data
#[async_trait::async_trait]
pub trait RateLimitStorage: Send + Sync {
    /// Check if a request should be allowed
    async fn check_limit(
        &self,
        key: &str,
        window: &RateLimitWindow,
    ) -> Result<RateLimitInfo, RateLimitError>;

    /// Get remaining requests for a key
    async fn get_remaining(&self, key: &str, window: &RateLimitWindow) -> Result<u32, RateLimitError>;
}

/// Rate limit information
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Number of requests remaining
    pub remaining: u32,
    /// Time when the limit resets (Unix timestamp)
    pub reset_at: u64,
    /// Retry after seconds (if rate limited)
    pub retry_after: Option<u64>,
}

/// In-memory rate limit storage
#[derive(Clone)]
pub struct InMemoryRateLimitStorage {
    records: Arc<RwLock<HashMap<String, RateLimitRecord>>>,
}

impl InMemoryRateLimitStorage {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

impl Default for InMemoryRateLimitStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl RateLimitStorage for InMemoryRateLimitStorage {
    async fn check_limit(
        &self,
        key: &str,
        window: &RateLimitWindow,
    ) -> Result<RateLimitInfo, RateLimitError> {
        let now = Self::current_timestamp();
        let mut records = self.records.write().await;

        let record = records
            .entry(key.to_string())
            .or_insert_with(|| RateLimitRecord::new(now));

        // Check if window expired
        if record.is_expired(now, window.window_secs) {
            record.count = 1;
            record.window_start = now;
        } else {
            record.count += 1;
        }

        let allowed = record.count <= window.limit;
        let remaining = if record.count <= window.limit {
            window.limit - record.count
        } else {
            0
        };
        let reset_at = record.window_start + window.window_secs;
        let retry_after = if !allowed {
            Some(reset_at.saturating_sub(now))
        } else {
            None
        };

        Ok(RateLimitInfo {
            allowed,
            remaining,
            reset_at,
            retry_after,
        })
    }

    async fn get_remaining(&self, key: &str, window: &RateLimitWindow) -> Result<u32, RateLimitError> {
        let now = Self::current_timestamp();
        let records = self.records.read().await;

        match records.get(key) {
            Some(record) if !record.is_expired(now, window.window_secs) => {
                Ok(window.limit.saturating_sub(record.count))
            }
            _ => Ok(window.limit),
        }
    }
}

/// Rate limit error
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Rate limit configuration
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Rate limiting strategy
    pub strategy: RateLimitStrategy,
    /// Rate limit window
    pub window: RateLimitWindow,
    /// Exempt routes (patterns)
    pub exempt_routes: Arc<Vec<String>>,
    /// Whitelisted IPs
    pub whitelisted_ips: Arc<Vec<IpAddr>>,
}

impl RateLimitConfig {
    /// Create a per-IP rate limiter
    pub fn per_ip(requests_per_minute: u32) -> Self {
        Self {
            strategy: RateLimitStrategy::PerIp,
            window: RateLimitWindow::per_minute(requests_per_minute),
            exempt_routes: Arc::new(Vec::new()),
            whitelisted_ips: Arc::new(Vec::new()),
        }
    }

    /// Create a per-user rate limiter
    pub fn per_user(requests_per_minute: u32) -> Self {
        Self {
            strategy: RateLimitStrategy::PerUser,
            window: RateLimitWindow::per_minute(requests_per_minute),
            exempt_routes: Arc::new(Vec::new()),
            whitelisted_ips: Arc::new(Vec::new()),
        }
    }

    /// Create a per-route rate limiter
    pub fn per_route(requests_per_minute: u32) -> Self {
        Self {
            strategy: RateLimitStrategy::PerRoute,
            window: RateLimitWindow::per_minute(requests_per_minute),
            exempt_routes: Arc::new(Vec::new()),
            whitelisted_ips: Arc::new(Vec::new()),
        }
    }

    /// Set the rate limit window
    pub fn window(mut self, window: RateLimitWindow) -> Self {
        self.window = window;
        self
    }

    /// Add an exempt route pattern
    pub fn exempt(mut self, pattern: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.exempt_routes).push(pattern.into());
        self
    }

    /// Add a whitelisted IP
    pub fn whitelist_ip(mut self, ip: IpAddr) -> Self {
        Arc::make_mut(&mut self.whitelisted_ips).push(ip);
        self
    }

    /// Check if a path is exempt
    fn is_exempt(&self, path: &str) -> bool {
        self.exempt_routes.iter().any(|pattern| {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                path.starts_with(prefix)
            } else {
                path == pattern
            }
        })
    }

    /// Check if an IP is whitelisted
    fn is_whitelisted(&self, ip: &IpAddr) -> bool {
        self.whitelisted_ips.contains(ip)
    }
}

/// Rate limit middleware
pub struct RateLimitMiddleware<S: RateLimitStorage> {
    storage: Arc<S>,
    config: RateLimitConfig,
}

impl<S: RateLimitStorage> RateLimitMiddleware<S> {
    /// Create a new rate limit middleware
    pub fn new(storage: S, config: RateLimitConfig) -> Self {
        Self {
            storage: Arc::new(storage),
            config,
        }
    }

    /// Handle rate limiting for a request
    pub async fn handle(&self, request: Request, next: Next) -> Response {
        let path = request.uri().path();

        // Check if route is exempt
        if self.config.is_exempt(path) {
            return next.run(request).await;
        }

        // Extract IP and check whitelist
        if let Some(ip) = self.extract_ip(&request) {
            if self.config.is_whitelisted(&ip) {
                return next.run(request).await;
            }
        }

        // Extract rate limit key
        let key = self.extract_key(&request);

        // Check rate limit
        match self.storage.check_limit(&key, &self.config.window).await {
            Ok(info) => {
                if info.allowed {
                    // Request allowed
                    let mut response = next.run(request).await;
                    self.add_rate_limit_headers(&mut response, &info);
                    response
                } else {
                    // Rate limit exceeded
                    self.rate_limited_response(&info)
                }
            }
            Err(_) => {
                // On storage error, allow the request
                next.run(request).await
            }
        }
    }

    /// Extract rate limit key based on strategy
    fn extract_key(&self, request: &Request<Body>) -> String {
        match &self.config.strategy {
            RateLimitStrategy::PerIp => {
                self.extract_ip(request)
                    .map(|ip| ip.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            }
            RateLimitStrategy::PerUser => {
                // Try to extract user ID from auth extensions
                request
                    .extensions()
                    .get::<crate::auth::middleware::AuthUser>()
                    .map(|user| format!("user:{}", user.user_id))
                    .unwrap_or_else(|| "anonymous".to_string())
            }
            RateLimitStrategy::PerRoute => {
                format!("route:{}", request.uri().path())
            }
            RateLimitStrategy::Custom(extractor) => extractor(request),
        }
    }

    /// Extract IP address from request
    fn extract_ip(&self, request: &Request<Body>) -> Option<IpAddr> {
        // Try X-Forwarded-For header first
        if let Some(forwarded) = request.headers().get("X-Forwarded-For") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse() {
                        return Some(ip);
                    }
                }
            }
        }

        // Try X-Real-IP header
        if let Some(real_ip) = request.headers().get("X-Real-IP") {
            if let Ok(ip_str) = real_ip.to_str() {
                if let Ok(ip) = ip_str.parse() {
                    return Some(ip);
                }
            }
        }

        None
    }

    /// Add rate limit headers to response
    fn add_rate_limit_headers(&self, response: &mut Response, info: &RateLimitInfo) {
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            self.config.window.limit.to_string().parse().unwrap(),
        );
        response.headers_mut().insert(
            "X-RateLimit-Remaining",
            info.remaining.to_string().parse().unwrap(),
        );
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            info.reset_at.to_string().parse().unwrap(),
        );
    }

    /// Generate rate limited response
    fn rate_limited_response(&self, info: &RateLimitInfo) -> Response {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please try again later.",
        )
            .into_response();

        // Add rate limit headers
        self.add_rate_limit_headers(&mut response, info);

        // Add Retry-After header
        if let Some(retry_after) = info.retry_after {
            response.headers_mut().insert(
                header::RETRY_AFTER,
                retry_after.to_string().parse().unwrap(),
            );
        }

        response
    }
}

impl RateLimitMiddleware<InMemoryRateLimitStorage> {
    /// Create a rate limiter with in-memory storage
    pub fn in_memory(config: RateLimitConfig) -> Self {
        Self::new(InMemoryRateLimitStorage::new(), config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limit_record() {
        let now = 1000;
        let mut record = RateLimitRecord::new(now);

        assert_eq!(record.count, 1);
        assert_eq!(record.window_start, now);
        assert!(!record.is_expired(now + 30, 60));
        assert!(record.is_expired(now + 70, 60));
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryRateLimitStorage::new();
        let window = RateLimitWindow::per_minute(5);

        // First request should be allowed
        let info = storage.check_limit("test-key", &window).await.unwrap();
        assert!(info.allowed);
        assert_eq!(info.remaining, 4);

        // Subsequent requests
        for i in 1..5 {
            let info = storage.check_limit("test-key", &window).await.unwrap();
            assert!(info.allowed);
            assert_eq!(info.remaining, 4 - i);
        }

        // 6th request should be denied
        let info = storage.check_limit("test-key", &window).await.unwrap();
        assert!(!info.allowed);
        assert_eq!(info.remaining, 0);
        assert!(info.retry_after.is_some());
    }

    #[test]
    fn test_exempt_routes() {
        let config = RateLimitConfig::per_ip(60)
            .exempt("/health")
            .exempt("/api/webhooks/*");

        assert!(config.is_exempt("/health"));
        assert!(config.is_exempt("/api/webhooks/stripe"));
        assert!(config.is_exempt("/api/webhooks/github"));
        assert!(!config.is_exempt("/api/users"));
    }

    #[test]
    fn test_whitelist() {
        let config = RateLimitConfig::per_ip(60)
            .whitelist_ip("127.0.0.1".parse().unwrap())
            .whitelist_ip("::1".parse().unwrap());

        assert!(config.is_whitelisted(&"127.0.0.1".parse().unwrap()));
        assert!(config.is_whitelisted(&"::1".parse().unwrap()));
        assert!(!config.is_whitelisted(&"192.168.1.1".parse().unwrap()));
    }
}
