//! Integration tests for Redis cache backend
//!
//! These tests require a running Redis instance.
//! Start test services with: ./scripts/test-env-up.sh
//! Then run: cargo test --features redis-backend

#![cfg(feature = "redis-backend")]

use rf_cache::{Cache, CacheConfig, RedisCache};
use std::sync::Arc;
use std::time::Duration;

fn get_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

async fn redis_available() -> bool {
    use redis::AsyncCommands;
    match redis::Client::open(get_redis_url().as_str()) {
        Ok(client) => match client.get_multiplexed_async_connection().await {
            Ok(mut conn) => conn.ping::<_, String>().await.is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

#[tokio::test]
async fn test_redis_basic_operations() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_basic_operations: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_basic")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    // Set and get
    cache
        .set("key1", &"value1", Duration::from_secs(60))
        .await
        .unwrap();
    let value: Option<String> = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));

    // Exists
    assert!(cache.exists("key1").await.unwrap());
    assert!(!cache.exists("nonexistent").await.unwrap());

    // Delete
    cache.delete("key1").await.unwrap();
    let value: Option<String> = cache.get("key1").await.unwrap();
    assert_eq!(value, None);

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_distributed_cache() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_distributed_cache: Redis not available");
        return;
    }
    let cache1 = RedisCache::new(&get_redis_url(), "test_distributed")
        .await
        .unwrap();
    let cache2 = RedisCache::new(&get_redis_url(), "test_distributed")
        .await
        .unwrap();

    cache1.flush().await.unwrap();

    // Set in cache1
    cache1
        .set("shared_key", &"shared_value", Duration::from_secs(60))
        .await
        .unwrap();

    // Get from cache2 (different instance)
    let value: Option<String> = cache2.get("shared_key").await.unwrap();
    assert_eq!(value, Some("shared_value".to_string()));

    // Update in cache2
    cache2
        .set("shared_key", &"updated_value", Duration::from_secs(60))
        .await
        .unwrap();

    // Get from cache1
    let value: Option<String> = cache1.get("shared_key").await.unwrap();
    assert_eq!(value, Some("updated_value".to_string()));

    // Cleanup
    cache1.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_ttl_expiration() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_ttl_expiration: Redis not available");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_ttl").await.unwrap();
    cache.flush().await.unwrap();

    // Set with short TTL
    cache
        .set("expiring_key", &"value", Duration::from_secs(2))
        .await
        .unwrap();

    // Should exist initially
    let value: Option<String> = cache.get("expiring_key").await.unwrap();
    assert_eq!(value, Some("value".to_string()));

    // Wait for expiration
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Should be expired
    let value: Option<String> = cache.get("expiring_key").await.unwrap();
    assert_eq!(value, None);

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_cache_tags() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_cache_tags: Redis not available");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_tags")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    // Set multiple entries with tags
    cache
        .tags(&["users", "user:1"])
        .set("user:1:profile", &"profile data 1", Duration::from_secs(60))
        .await
        .unwrap();

    cache
        .tags(&["users", "user:2"])
        .set("user:2:profile", &"profile data 2", Duration::from_secs(60))
        .await
        .unwrap();

    cache
        .tags(&["posts", "user:1"])
        .set("user:1:posts", &"posts data", Duration::from_secs(60))
        .await
        .unwrap();

    // Verify data exists
    let value: Option<String> = cache.get("user:1:profile").await.unwrap();
    assert_eq!(value, Some("profile data 1".to_string()));

    let value: Option<String> = cache.get("user:2:profile").await.unwrap();
    assert_eq!(value, Some("profile data 2".to_string()));

    // Flush by tag "users"
    cache.tags(&["users"]).flush().await.unwrap();

    // User profiles should be gone
    let value: Option<String> = cache.get("user:1:profile").await.unwrap();
    assert_eq!(value, None);

    let value: Option<String> = cache.get("user:2:profile").await.unwrap();
    assert_eq!(value, None);

    // Posts should still exist (different tag)
    let value: Option<String> = cache.get("user:1:posts").await.unwrap();
    assert_eq!(value, Some("posts data".to_string()));

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_tag_based_invalidation() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_tag_based_invalidation: Redis not available");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_invalidation")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    // Cache user data with multiple tags
    cache
        .tags(&["users", "premium", "verified"])
        .set("user:123", &"premium user data", Duration::from_secs(60))
        .await
        .unwrap();

    // Invalidate all premium users
    cache.tags(&["premium"]).flush().await.unwrap();

    // Data should be gone
    let value: Option<String> = cache.get("user:123").await.unwrap();
    assert_eq!(value, None);

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_remember_with_lock() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_remember_with_lock: Redis not available");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_lock")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    let mut computation_count = 0;

    // First call should compute
    let value: String = cache
        .remember_with_lock("expensive_key", Duration::from_secs(60), || async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            computation_count += 1;
            Ok("computed value".to_string())
        })
        .await
        .unwrap();

    assert_eq!(value, "computed value");
    assert_eq!(computation_count, 1);

    // Second call should use cached value (fast, no computation)
    let start = std::time::Instant::now();
    let value: String = cache
        .remember_with_lock("expensive_key", Duration::from_secs(60), || async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            computation_count += 1;
            Ok("new value".to_string())
        })
        .await
        .unwrap();

    let elapsed = start.elapsed();
    assert_eq!(value, "computed value");
    assert_eq!(computation_count, 1); // Should not have computed again
    assert!(elapsed < Duration::from_millis(50)); // Should be fast

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_stampede_prevention() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_stampede_prevention: Redis not available");
        return;
    }
    let cache = Arc::new(
        RedisCache::new(&get_redis_url(), "test_stampede")
            .await
            .unwrap(),
    );
    cache.flush().await.unwrap();

    let computation_count = Arc::new(tokio::sync::Mutex::new(0));

    // Spawn multiple concurrent requests for same key
    let mut handles = vec![];

    for _ in 0..10 {
        let cache = cache.clone();
        let computation_count = computation_count.clone();

        let handle = tokio::spawn(async move {
            cache
                .remember_with_lock("stampede_key", Duration::from_secs(60), || async {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    let mut count = computation_count.lock().await;
                    *count += 1;
                    Ok::<_, rf_cache::CacheError>("computed".to_string())
                })
                .await
        });
        handles.push(handle);
    }

    // Wait for all requests
    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        assert_eq!(result, "computed");
    }

    // Should only have computed once (stampede prevented)
    let final_count = *computation_count.lock().await;
    assert_eq!(
        final_count, 1,
        "Expected 1 computation, got {}",
        final_count
    );

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_remember_pattern() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_remember_pattern: Redis not available");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_remember")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    // First call computes value
    let value: String = cache
        .remember("computed_key", Duration::from_secs(60), || async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok("computed value".to_string())
        })
        .await
        .unwrap();

    assert_eq!(value, "computed value");

    // Second call uses cached value (fast)
    let start = std::time::Instant::now();
    let value: String = cache
        .remember("computed_key", Duration::from_secs(60), || async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok("new value".to_string())
        })
        .await
        .unwrap();

    let elapsed = start.elapsed();
    assert_eq!(value, "computed value");
    assert!(elapsed < Duration::from_millis(25));

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_cache_config() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_cache_config: Redis not available");
        return;
    }
    let config = CacheConfig::redis(get_redis_url(), "test_config");
    let cache = config.build().await.unwrap();

    cache.flush().await.unwrap();

    // Basic operations through config-created cache
    cache
        .set("config_key", &"config_value", Duration::from_secs(60))
        .await
        .unwrap();

    let value: Option<String> = cache.get("config_key").await.unwrap();
    assert_eq!(value, Some("config_value".to_string()));

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_complex_data_types() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_complex_data_types: Redis not available");
        return;
    }
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
        email: String,
        tags: Vec<String>,
    }

    let cache = RedisCache::new(&get_redis_url(), "test_complex")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    let user = User {
        id: 123,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        tags: vec!["admin".to_string(), "verified".to_string()],
    };

    // Cache complex object
    cache
        .set("user:123", &user, Duration::from_secs(60))
        .await
        .unwrap();

    // Retrieve complex object
    let cached_user: Option<User> = cache.get("user:123").await.unwrap();
    assert_eq!(cached_user, Some(user));

    // Cleanup
    cache.flush().await.unwrap();
}

#[tokio::test]
async fn test_redis_cache_performance() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_cache_performance: Redis not available");
        return;
    }
    let cache = RedisCache::new(&get_redis_url(), "test_performance")
        .await
        .unwrap();
    cache.flush().await.unwrap();

    let start = std::time::Instant::now();

    // Write 1000 entries
    for i in 0..1000 {
        let key = format!("key:{}", i);
        let value = format!("value:{}", i);
        cache
            .set(&key, &value, Duration::from_secs(60))
            .await
            .unwrap();
    }

    let write_elapsed = start.elapsed();
    let write_throughput = 1000.0 / write_elapsed.as_secs_f64();

    println!("Write throughput: {:.0} ops/sec", write_throughput);

    // Read 1000 entries
    let start = std::time::Instant::now();

    for i in 0..1000 {
        let key = format!("key:{}", i);
        let _: Option<String> = cache.get(&key).await.unwrap();
    }

    let read_elapsed = start.elapsed();
    let read_throughput = 1000.0 / read_elapsed.as_secs_f64();

    println!("Read throughput: {:.0} ops/sec", read_throughput);

    // Performance assertions
    assert!(write_throughput > 100.0); // At least 100 ops/sec
    assert!(read_throughput > 100.0); // At least 100 ops/sec

    // Cleanup
    cache.flush().await.unwrap();
}
