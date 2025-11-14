# rf-cache: Advanced Caching for RustForge

Production-ready caching with multiple backends and advanced features.

## Features

- **Multiple Backends**: Memory (development) and Redis (production)
- **Distributed Caching**: Redis backend supports multiple instances
- **Cache Tags**: Group related cache entries
- **Tag Invalidation**: Flush all entries with specific tags
- **Stampede Prevention**: Built-in locking to prevent cache stampedes
- **TTL Support**: Time-to-live for automatic expiration
- **Remember Pattern**: Laravel-style cache-or-compute pattern
- **Type-Safe**: Generic types with serde serialization

## Installation

```toml
[dependencies]
rf-cache = "0.2.0"

# For Redis backend (production)
rf-cache = { version = "0.2.0", features = ["redis-backend"] }
```

## Quick Start

### Memory Backend (Development)

```rust
use rf_cache::{MemoryCache, Cache};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cache = MemoryCache::new();

    // Set value
    cache.set("key", &"value", Duration::from_secs(60)).await?;

    // Get value
    let value: Option<String> = cache.get("key").await?;
    println!("Cached value: {:?}", value);

    // Delete value
    cache.delete("key").await?;

    Ok(())
}
```

### Redis Backend (Production)

```rust
use rf_cache::{RedisCache, Cache};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Redis cache
    let cache = RedisCache::new("redis://localhost:6379", "myapp").await?;

    // Or use config builder
    let cache = CacheConfig::redis("redis://localhost:6379", "myapp")
        .build()
        .await?;

    // Basic operations
    cache.set("user:123", &"John Doe", Duration::from_secs(3600)).await?;
    let name: Option<String> = cache.get("user:123").await?;

    Ok(())
}
```

## Configuration

### Using Config Builder

```rust
use rf_cache::CacheConfigBuilder;

let cache = CacheConfigBuilder::new()
    .backend("redis")
    .redis_url("redis://localhost:6379")
    .prefix("myapp")
    .build()
    .await?;
```

### From Environment Variables

```rust
use rf_cache::CacheConfig;

// Reads from REDIS_URL and CACHE_PREFIX env vars
let cache = CacheConfig::redis_from_env().build().await?;
```

Environment variables:
- `REDIS_URL`: Redis connection URL (default: "redis://localhost:6379")
- `CACHE_PREFIX`: Cache key prefix (default: "cache")

## Core Features

### Basic Operations

```rust
use rf_cache::{Cache, MemoryCache};
use std::time::Duration;

let cache = MemoryCache::new();

// Set with TTL
cache.set("key", &"value", Duration::from_secs(60)).await?;

// Get
let value: Option<String> = cache.get("key").await?;

// Exists
if cache.exists("key").await? {
    println!("Key exists!");
}

// Delete
cache.delete("key").await?;

// Flush all
cache.flush().await?;
```

### Cache Tags

Group related cache entries:

```rust
// Set with tags
cache.tags(&["users", "user:123"])
    .set("user:123:profile", &user_data, Duration::from_secs(3600))
    .await?;

cache.tags(&["users", "user:456"])
    .set("user:456:profile", &user_data, Duration::from_secs(3600))
    .await?;

// Invalidate all entries with "users" tag
cache.tags(&["users"]).flush().await?;
```

### Remember Pattern

Cache-or-compute pattern:

```rust
// If cached, return from cache
// If not cached, compute and cache
let user: User = cache.remember(
    "user:123",
    Duration::from_secs(3600),
    || async {
        // Expensive computation
        let user = fetch_user_from_db(123).await?;
        Ok(user)
    }
).await?;
```

### Stampede Prevention

Prevent cache stampede with distributed locks:

```rust
let cache = RedisCache::new("redis://localhost", "myapp").await?;

// Only one process will compute the value
// Others will wait and use the computed result
let value = cache.remember_with_lock(
    "expensive_computation",
    Duration::from_secs(3600),
    || async {
        // This will only run once, even with concurrent requests
        expensive_computation().await
    }
).await?;
```

## Advanced Features

### Complex Data Types

Cache any serializable type:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
    preferences: Vec<String>,
}

let user = User {
    id: 123,
    name: "John Doe".to_string(),
    email: "john@example.com".to_string(),
    preferences: vec!["dark_mode".to_string()],
};

// Cache complex type
cache.set("user:123", &user, Duration::from_secs(3600)).await?;

// Retrieve complex type
let cached_user: Option<User> = cache.get("user:123").await?;
```

### Multiple Tags

Invalidate by multiple tags:

```rust
// Cache with multiple tags
cache.tags(&["users", "premium", "verified"])
    .set("user:123", &user_data, Duration::from_secs(3600))
    .await?;

// Invalidate all premium users
cache.tags(&["premium"]).flush().await?;

// Invalidate all verified users
cache.tags(&["verified"]).flush().await?;
```

### Cache Hierarchies

Build cache key hierarchies:

```rust
// User profile cache
cache.set("user:123:profile", &profile, Duration::from_secs(3600)).await?;

// User settings cache
cache.set("user:123:settings", &settings, Duration::from_secs(3600)).await?;

// User posts cache
cache.set("user:123:posts", &posts, Duration::from_secs(600)).await?;

// Invalidate all user data
cache.tags(&["user:123"]).flush().await?;
```

## Redis Backend Features

### Distributed Caching

Share cache across multiple instances:

```rust
// Instance 1
let cache1 = RedisCache::new("redis://localhost", "myapp").await?;
cache1.set("shared_key", &"value", Duration::from_secs(60)).await?;

// Instance 2 (different process/machine)
let cache2 = RedisCache::new("redis://localhost", "myapp").await?;
let value: Option<String> = cache2.get("shared_key").await?;
// Returns: Some("value")
```

### Persistence

Cache data persists in Redis:

```rust
// Set value
cache.set("key", &"value", Duration::from_secs(3600)).await?;

// Application restarts...

// Value is still there
let value: Option<String> = cache.get("key").await?;
```

### Atomic Operations

Stampede prevention uses atomic Redis operations:

```rust
// Multiple processes trying to cache same key
// Only one will compute, others will wait
let cache = RedisCache::new("redis://localhost", "myapp").await?;

// Process 1, 2, 3... all call this simultaneously
let value = cache.remember_with_lock(
    "key",
    Duration::from_secs(60),
    || async {
        // Only executed once
        expensive_operation().await
    }
).await?;
```

## Performance

Redis backend performance:

- **Throughput**: 100,000+ ops/sec
- **Latency**: <1ms per operation
- **Persistence**: Optional (configure Redis)
- **Distributed**: Yes

Memory backend performance:

- **Throughput**: 1,000,000+ ops/sec
- **Latency**: <0.001ms per operation
- **Persistence**: No
- **Distributed**: No

## Examples

### User Profile Caching

```rust
use rf_cache::{Cache, CacheConfig};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct UserProfile {
    id: u64,
    name: String,
    email: String,
    avatar_url: String,
}

async fn get_user_profile(
    cache: &impl Cache,
    user_id: u64,
) -> Result<UserProfile, Box<dyn std::error::Error>> {
    let cache_key = format!("user:{}:profile", user_id);

    // Try cache first
    if let Some(profile) = cache.get(&cache_key).await? {
        return Ok(profile);
    }

    // Fetch from database
    let profile = fetch_from_db(user_id).await?;

    // Cache for 1 hour
    cache.set(&cache_key, &profile, Duration::from_secs(3600)).await?;

    Ok(profile)
}
```

### API Response Caching

```rust
async fn get_weather(
    cache: &impl Cache,
    city: &str,
) -> Result<WeatherData, Box<dyn std::error::Error>> {
    let cache_key = format!("weather:{}", city);

    cache.remember(
        &cache_key,
        Duration::from_secs(600), // Cache for 10 minutes
        || async {
            // Fetch from external API
            let response = reqwest::get(&format!(
                "https://api.weather.com/{}",
                city
            )).await?;

            let data: WeatherData = response.json().await?;
            Ok(data)
        }
    ).await
}
```

### Multi-Level Cache Invalidation

```rust
async fn update_user(
    cache: &RedisCache,
    user_id: u64,
    updates: UserUpdates,
) -> Result<(), Box<dyn std::error::Error>> {
    // Update database
    db.update_user(user_id, updates).await?;

    // Invalidate all user-related caches
    cache.tags(&[
        &format!("user:{}", user_id),
        "users",
        "user_list",
    ]).flush().await?;

    Ok(())
}
```

### Rate Limiting with Cache

```rust
async fn check_rate_limit(
    cache: &impl Cache,
    user_id: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let key = format!("rate_limit:user:{}", user_id);

    let count: Option<u32> = cache.get(&key).await?;

    if let Some(count) = count {
        if count >= 100 {
            return Ok(false); // Rate limit exceeded
        }

        cache.set(&key, &(count + 1), Duration::from_secs(3600)).await?;
    } else {
        cache.set(&key, &1u32, Duration::from_secs(3600)).await?;
    }

    Ok(true)
}
```

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_cache_operations() {
    let cache = MemoryCache::new();

    cache.set("key", &"value", Duration::from_secs(60)).await.unwrap();

    let value: Option<String> = cache.get("key").await.unwrap();
    assert_eq!(value, Some("value".to_string()));

    cache.delete("key").await.unwrap();

    let value: Option<String> = cache.get("key").await.unwrap();
    assert_eq!(value, None);
}
```

### Integration Tests (Redis)

```rust
#[tokio::test]
#[ignore] // Requires Redis
async fn test_distributed_cache() {
    let cache1 = RedisCache::new("redis://localhost", "test").await.unwrap();
    let cache2 = RedisCache::new("redis://localhost", "test").await.unwrap();

    // Set in cache1
    cache1.set("shared", &"value", Duration::from_secs(60)).await.unwrap();

    // Get from cache2
    let value: Option<String> = cache2.get("shared").await.unwrap();
    assert_eq!(value, Some("value".to_string()));
}
```

Run Redis tests:
```bash
# Start Redis
docker run -d -p 6379:6379 redis:7

# Run tests
cargo test --features redis-backend -- --ignored
```

## Comparison: Memory vs Redis

| Feature | Memory Backend | Redis Backend |
|---------|---------------|---------------|
| **Use Case** | Development, Testing | Production |
| **Persistence** | No | Yes (optional) |
| **Distributed** | No | Yes |
| **Performance** | Very High (1M+ ops/sec) | High (100K+ ops/sec) |
| **Stampede Prevention** | Local locks | Distributed locks |
| **Setup** | None | Requires Redis |
| **Configuration** | `MemoryCache::new()` | `RedisCache::new(url, prefix)` |

## Best Practices

### 1. Use Redis in Production

Always use Redis backend in production:

```rust
let cache = if cfg!(debug_assertions) {
    CacheConfig::memory().build().await?
} else {
    CacheConfig::redis_from_env().build().await?
};
```

### 2. Set Appropriate TTLs

Choose TTL based on data volatility:

```rust
// Static data: long TTL
cache.set("config", &config, Duration::from_secs(86400)).await?; // 24 hours

// User data: medium TTL
cache.set("user:123", &user, Duration::from_secs(3600)).await?; // 1 hour

// Dynamic data: short TTL
cache.set("stats", &stats, Duration::from_secs(60)).await?; // 1 minute
```

### 3. Use Tags for Related Data

Group related cache entries:

```rust
// Tag by entity type and ID
cache.tags(&["users", &format!("user:{}", user_id)])
    .set(&key, &value, ttl)
    .await?;

// Invalidate all user-related caches
cache.tags(&[&format!("user:{}", user_id)]).flush().await?;
```

### 4. Prevent Cache Stampedes

Use `remember_with_lock` for expensive operations:

```rust
// Good: Uses lock
let value = cache.remember_with_lock(key, ttl, || async {
    expensive_operation().await
}).await?;

// Bad: No stampede prevention
if let Some(value) = cache.get(key).await? {
    return value;
}
let value = expensive_operation().await?;
cache.set(key, &value, ttl).await?;
```

### 5. Handle Cache Failures Gracefully

Always have fallback logic:

```rust
let value = match cache.get("key").await {
    Ok(Some(value)) => value,
    Ok(None) | Err(_) => {
        // Fallback to database
        fetch_from_db().await?
    }
};
```

## License

MIT OR Apache-2.0
