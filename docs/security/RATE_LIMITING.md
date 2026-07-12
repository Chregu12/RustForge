# Rate Limiting Guide

## Overview

Rate limiting protects your application from abuse by limiting the number of requests a client can make within a time window.

## Quick Start

```rust
use rf_ratelimit::{RateLimitLayer, RateLimitConfig, MemoryRateLimiter};
use std::sync::Arc;

// Create rate limiter: 60 requests per minute
let layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::per_minute(60))
));

// Add to router
app = app.layer(layer);
```

## Strategies

### 1. Per Minute (global)

Limit total requests within a time window:

```rust
use rf_ratelimit::RateLimitConfig;

let config = RateLimitConfig::per_minute(60); // 60 requests per minute
```

**Best for:** Public APIs, anonymous endpoints

### 2. Per Hour

```rust
let config = RateLimitConfig::per_hour(1000); // 1000 requests per hour
```

**Best for:** Resource-intensive or low-frequency operations

### 3. Custom Key Extraction

Differentiate rate limits by client IP, user ID, or API key using `RateLimitLayer::with_key_extractor`:

```rust
use rf_ratelimit::{RateLimitLayer, RateLimitConfig, MemoryRateLimiter};
use std::sync::Arc;

let layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::per_minute(100))
)).with_key_extractor(|req| {
    // Use API key as the rate-limit bucket key
    req.headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|key| format!("api_key:{}", key))
        .unwrap_or_else(|| "anonymous".to_string())
});
```

## Time Windows

### Per Minute (Default)

```rust
RateLimitConfig::per_minute(60)
```

### Per Hour

```rust
RateLimitConfig::per_hour(1000)
```

### Custom Window

```rust
use std::time::Duration;

RateLimitConfig::custom(500, Duration::from_secs(300)) // 500 requests per 5 minutes
```

## Advanced Configuration

### Multiple Rate Limits

Apply different limits to different route groups:

```rust
use rf_ratelimit::{RateLimitLayer, RateLimitConfig, MemoryRateLimiter};
use std::sync::Arc;

// Strict limit for auth endpoints
let auth_layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::per_minute(5))
));

// More relaxed for API
let api_layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::per_minute(100))
));

// Apply to specific route groups
let app = Router::new()
    .route("/login", post(login))
    .layer(auth_layer)
    .route("/api/*path", get(api_handler))
    .layer(api_layer);
```

### Exempt Health / Metrics Routes

Apply rate limiting only to specific nested routers and keep health / metrics routes outside that router:

```rust
// Wrap only the routes that should be rate-limited
let protected = Router::new()
    .route("/api/*path", get(api_handler))
    .layer(api_layer);

let app = Router::new()
    .route("/health", get(health))
    .route("/metrics", get(metrics))
    .merge(protected);
```

## Storage Backends

### In-Memory (Development)

```rust
use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimitLayer};
use std::sync::Arc;

let layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::per_minute(60))
));
```

**Pros:** Fast, simple, zero external dependencies
**Cons:** Not shared across instances, counters reset on restart

### Redis (Production)

```rust
use rf_ratelimit::{RedisRateLimiter, RateLimitConfig, RateLimitLayer};
use std::sync::Arc;

let layer = RateLimitLayer::new(Arc::new(
    RedisRateLimiter::new("redis://localhost:6379", RateLimitConfig::per_minute(60))
));
```

**Pros:** Shared across instances, persistent counters
**Cons:** Requires Redis; `rf-ratelimit` must be compiled with the `redis` feature flag

## Response Headers

Rate limit information is included in response headers:

```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 42
X-RateLimit-Reset: 1699564800
```

### When Rate Limited (429)

```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1699564800
Retry-After: 23

Rate limit exceeded. Please try again later.
```

## Client-Side Handling

### JavaScript Example

```javascript
async function makeRequest(url) {
    const response = await fetch(url);

    // Check rate limit headers
    const limit = response.headers.get('X-RateLimit-Limit');
    const remaining = response.headers.get('X-RateLimit-Remaining');
    const reset = response.headers.get('X-RateLimit-Reset');

    console.log(`Rate limit: ${remaining}/${limit}, resets at ${new Date(reset * 1000)}`);

    if (response.status === 429) {
        const retryAfter = response.headers.get('Retry-After');
        console.log(`Rate limited! Retry after ${retryAfter} seconds`);

        // Wait and retry
        await new Promise(resolve => setTimeout(resolve, retryAfter * 1000));
        return makeRequest(url);
    }

    return response.json();
}
```

### Exponential Backoff

```javascript
async function fetchWithBackoff(url, maxRetries = 3) {
    for (let i = 0; i < maxRetries; i++) {
        const response = await fetch(url);

        if (response.status !== 429) {
            return response;
        }

        // Exponential backoff: 1s, 2s, 4s, 8s, ...
        const delay = Math.pow(2, i) * 1000;
        await new Promise(resolve => setTimeout(resolve, delay));
    }

    throw new Error('Max retries exceeded');
}
```

## Use Cases

### 1. Prevent Brute Force Attacks

```rust
use rf_ratelimit::{RateLimitLayer, RateLimitConfig, MemoryRateLimiter};
use std::{sync::Arc, time::Duration};

// 5 attempts per 5 minutes
let auth_layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::custom(5, Duration::from_secs(300)))
));
```

### 2. API Rate Limiting

```rust
// Different tiers — differentiate by user via key_extractor
let free_layer  = RateLimitLayer::new(Arc::new(MemoryRateLimiter::new(RateLimitConfig::per_minute(100))));
let pro_layer   = RateLimitLayer::new(Arc::new(MemoryRateLimiter::new(RateLimitConfig::per_minute(1000))));
```

### 3. Resource-Intensive Operations

```rust
// 5 exports per hour
let export_layer = RateLimitLayer::new(Arc::new(
    MemoryRateLimiter::new(RateLimitConfig::per_hour(5))
));
```

## Best Practices

### ✅ DO
- Use different limits for different endpoint types
- Whitelist internal services and health checks
- Return clear error messages with Retry-After
- Monitor rate limit violations
- Adjust limits based on actual usage patterns
- Use Redis for production (shared state)

### ❌ DON'T
- Set limits too low (frustrates legitimate users)
- Apply same limit to all endpoints
- Rate limit health check endpoints
- Forget to exempt webhooks
- Use in-memory storage in production clusters

## Monitoring

### Track Rate Limit Events

```rust
// Log rate limit violations
app.layer(axum::middleware::from_fn(|req: Request, next: Next| async move {
    let response = next.run(req).await;

    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        tracing::warn!(
            "Rate limit exceeded for {}",
            req.headers().get("X-Forwarded-For").unwrap_or(&"unknown")
        );
    }

    response
}));
```

### Metrics

Track:
- Rate limit hit rate (429 responses / total requests)
- Top rate-limited IPs
- Rate limit exhaustion (users hitting limit frequently)
- Average requests per user/IP

## Testing

```rust
#[tokio::test]
async fn test_rate_limiting() {
    use rf_ratelimit::{RateLimitLayer, RateLimitConfig, MemoryRateLimiter};
    use std::sync::Arc;

    let limiter = Arc::new(MemoryRateLimiter::new(RateLimitConfig::per_minute(5)));
    let layer = RateLimitLayer::new(limiter);

    // Make 5 requests (should succeed)
    for _ in 0..5 {
        let response = make_request(&limiter).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    // 6th request should be rate limited
    let response = make_request(&limiter).await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
```

## Troubleshooting

### All Requests Get Rate Limited

**Problem:** Even first request returns 429

**Solutions:**
1. Check if limit is too low
2. Verify key extraction is working correctly
3. Check if IP is being extracted properly
4. Clear rate limit storage

### Rate Limits Not Shared Across Instances

**Problem:** Each instance has its own limits

**Solution:** Use the Redis-backed limiter instead of in-memory:
```rust
use rf_ratelimit::{RedisRateLimiter, RateLimitConfig, RateLimitLayer};
use std::sync::Arc;

let layer = RateLimitLayer::new(Arc::new(
    RedisRateLimiter::new("redis://localhost", RateLimitConfig::per_minute(60))
));
```

### Wrong IP Being Used

**Problem:** All requests seem to come from same IP

**Solutions:**
1. Configure reverse proxy to set X-Forwarded-For
2. Trust proxy headers: `app.layer(tower_http::request_id::PropagateRequestIdLayer::new())`

## Related Documentation

- [CSRF Protection](./CSRF_PROTECTION.md)
- [Authentication](./AUTHENTICATION.md)
- [Security Best Practices](./SECURITY_BEST_PRACTICES.md)
