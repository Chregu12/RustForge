# Rate Limiting Guide

## Overview

Rate limiting protects your application from abuse by limiting the number of requests a client can make within a time window.

## Quick Start

```rust
use foundry_application::middleware::rate_limit::{
    RateLimitMiddleware, RateLimitConfig, InMemoryRateLimitStorage
};

// Create rate limiter: 60 requests per minute per IP
let limiter = RateLimitMiddleware::in_memory(
    RateLimitConfig::per_ip(60)
        .exempt("/health")
        .exempt("/metrics")
);

// Add to router
app = app.layer(axum::middleware::from_fn(move |req, next| {
    limiter.handle(req, next)
}));
```

## Strategies

### 1. Per IP Address

Limit requests based on client IP address:

```rust
let config = RateLimitConfig::per_ip(60); // 60 requests per minute
```

**Best for:** Public APIs, anonymous endpoints

### 2. Per User

Limit requests based on authenticated user:

```rust
let config = RateLimitConfig::per_user(100); // 100 requests per minute
```

**Best for:** Authenticated APIs, user dashboards

### 3. Per Route

Limit requests to specific routes:

```rust
let config = RateLimitConfig::per_route(30); // 30 requests per minute per route
```

**Best for:** Resource-intensive endpoints

### 4. Custom Strategy

Define custom key extraction logic:

```rust
use foundry_application::middleware::rate_limit::RateLimitStrategy;

let config = RateLimitConfig {
    strategy: RateLimitStrategy::Custom(Arc::new(|req| {
        // Extract API key from header
        req.headers()
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok())
            .map(|key| format!("api_key:{}", key))
            .unwrap_or_else(|| "anonymous".to_string())
    })),
    window: RateLimitWindow::per_minute(100),
    exempt_routes: Arc::new(vec![]),
    whitelisted_ips: Arc::new(vec![]),
};
```

## Time Windows

### Per Minute (Default)

```rust
RateLimitWindow::per_minute(60)
```

### Per Hour

```rust
RateLimitWindow::per_hour(1000)
```

### Custom Window

```rust
RateLimitWindow::custom(
    500,    // requests
    300     // seconds (5 minutes)
)
```

## Advanced Configuration

### Multiple Rate Limits

Apply different limits to different route groups:

```rust
// Strict limit for auth endpoints
let auth_limiter = RateLimitMiddleware::in_memory(
    RateLimitConfig::per_ip(5)  // Only 5 login attempts per minute
);

// More relaxed for API
let api_limiter = RateLimitMiddleware::in_memory(
    RateLimitConfig::per_user(100)
);

// Apply to specific route groups
let app = Router::new()
    .route("/login", post(login))
    .layer(axum::middleware::from_fn(move |req, next| {
        auth_limiter.handle(req, next)
    }))
    .route("/api/*", get(api_handler))
    .layer(axum::middleware::from_fn(move |req, next| {
        api_limiter.handle(req, next)
    }));
```

### Whitelist IPs

Exempt trusted IPs from rate limiting:

```rust
let config = RateLimitConfig::per_ip(60)
    .whitelist_ip("127.0.0.1".parse().unwrap())
    .whitelist_ip("::1".parse().unwrap())
    .whitelist_ip("10.0.0.1".parse().unwrap());  // Internal service
```

### Exempt Routes

Skip rate limiting for specific routes:

```rust
let config = RateLimitConfig::per_ip(60)
    .exempt("/health")
    .exempt("/metrics")
    .exempt("/webhooks/*");  // Wildcard support
```

## Storage Backends

### In-Memory (Development)

```rust
let storage = InMemoryRateLimitStorage::new();
let limiter = RateLimitMiddleware::new(storage, config);
```

**Pros:** Fast, simple
**Cons:** Not shared across instances, lost on restart

### Redis (Production)

```rust
// TODO: Redis backend implementation
// let storage = RedisRateLimitStorage::new("redis://localhost:6379");
// let limiter = RateLimitMiddleware::new(storage, config);
```

**Pros:** Shared across instances, persistent
**Cons:** Requires Redis server

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
// Limit login attempts
let auth_config = RateLimitConfig::per_ip(5)  // 5 attempts per minute
    .window(RateLimitWindow::custom(5, 300)); // 5 attempts per 5 minutes
```

### 2. API Rate Limiting

```rust
// Different tiers for different users
let free_tier_config = RateLimitConfig::per_user(100);      // 100/min
let pro_tier_config = RateLimitConfig::per_user(1000);     // 1000/min
let enterprise_config = RateLimitConfig::per_user(10000);  // 10000/min
```

### 3. DDoS Protection

```rust
// Aggressive rate limiting for public endpoints
let config = RateLimitConfig::per_ip(30)
    .window(RateLimitWindow::per_minute(30))
    .whitelist_ip("trusted-service-ip".parse().unwrap());
```

### 4. Resource-Intensive Operations

```rust
// Limit expensive operations
let export_config = RateLimitConfig::per_user(5)
    .window(RateLimitWindow::per_hour(5));  // 5 exports per hour
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
    let storage = InMemoryRateLimitStorage::new();
    let config = RateLimitConfig::per_ip(5);
    let limiter = RateLimitMiddleware::new(storage, config);

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

**Solution:** Use Redis backend instead of in-memory:
```rust
// Use shared Redis storage
let storage = RedisRateLimitStorage::new("redis://localhost");
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
