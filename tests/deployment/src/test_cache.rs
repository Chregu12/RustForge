//! Deployment tests for rf-cache

#[cfg(test)]
mod tests {
    use rf_cache::{Cache, MemoryCache, CacheManager};
    use rf_cache::prelude::CacheWarmer;
    use std::time::Duration;

    // ── MemoryCache ──────────────────────────────────────────────

    #[tokio::test]
    async fn memory_cache_set_get() {
        let cache = MemoryCache::new();
        cache.set("key1", &"value1", Duration::from_secs(60)).await.expect("set");
        let val: Option<String> = cache.get("key1").await.expect("get");
        assert_eq!(val, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn memory_cache_missing_key() {
        let cache = MemoryCache::new();
        let val: Option<String> = cache.get("nonexistent").await.expect("get");
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn memory_cache_delete() {
        let cache = MemoryCache::new();
        cache.set("key", &42, Duration::from_secs(60)).await.expect("set");
        cache.delete("key").await.expect("delete");
        let val: Option<i32> = cache.get("key").await.expect("get");
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn memory_cache_exists() {
        let cache = MemoryCache::new();
        cache.set("exists", &true, Duration::from_secs(60)).await.expect("set");
        assert!(cache.exists("exists").await.expect("exists"));
        assert!(!cache.exists("not-exists").await.expect("exists"));
    }

    #[tokio::test]
    async fn memory_cache_flush() {
        let cache = MemoryCache::new();
        cache.set("a", &1, Duration::from_secs(60)).await.expect("set");
        cache.set("b", &2, Duration::from_secs(60)).await.expect("set");
        cache.flush().await.expect("flush");
        let a: Option<i32> = cache.get("a").await.expect("get");
        assert!(a.is_none());
    }

    #[tokio::test]
    async fn memory_cache_increment_decrement() {
        let cache = MemoryCache::new();
        cache.set("counter", &0i64, Duration::from_secs(60)).await.expect("set");
        let val = cache.increment("counter", 5).await.expect("inc");
        assert_eq!(val, 5);
        let val = cache.decrement("counter", 2).await.expect("dec");
        assert_eq!(val, 3);
    }

    #[tokio::test]
    async fn memory_cache_get_many_set_many() {
        let cache = MemoryCache::new();
        let a = "hello".to_string();
        let b = "world".to_string();
        let items = vec![("k1", &a), ("k2", &b)];
        cache.set_many(&items, Duration::from_secs(60)).await.expect("set_many");

        let result: std::collections::HashMap<String, String> =
            cache.get_many(&["k1", "k2"]).await.expect("get_many");
        assert_eq!(result.get("k1"), Some(&"hello".to_string()));
        assert_eq!(result.get("k2"), Some(&"world".to_string()));
    }

    #[tokio::test]
    async fn memory_cache_remember() {
        let cache = MemoryCache::new();
        let val: String = cache.remember("computed", Duration::from_secs(60), || async {
            Ok("computed_value".to_string())
        }).await.expect("remember");
        assert_eq!(val, "computed_value");

        // Second call should return cached value
        let val2: String = cache.remember("computed", Duration::from_secs(60), || async {
            Ok("should_not_be_this".to_string())
        }).await.expect("remember");
        assert_eq!(val2, "computed_value");
    }

    #[tokio::test]
    async fn memory_cache_stats() {
        let cache = MemoryCache::new();
        cache.set("x", &1, Duration::from_secs(60)).await.expect("set");
        let _: Option<i32> = cache.get("x").await.expect("get");
        let _: Option<i32> = cache.get("miss").await.expect("get");

        let stats = cache.stats().await;
        assert!(stats.hits >= 1);
        assert!(stats.misses >= 1);
        assert!(stats.sets >= 1);
    }

    // ── Tagged Cache ─────────────────────────────────────────────

    #[tokio::test]
    async fn tagged_cache_operations() {
        let cache = MemoryCache::new();
        let tagged = cache.tags(&["users", "posts"]);

        tagged.set("user:1", &"John", Duration::from_secs(60)).await.expect("set");
        let val: Option<String> = tagged.get("user:1").await.expect("get");
        assert_eq!(val, Some("John".to_string()));

        tagged.flush().await.expect("flush");
        let val: Option<String> = tagged.get("user:1").await.expect("get");
        assert!(val.is_none());
    }

    // ── CacheManager (Facade) ────────────────────────────────────

    #[test]
    fn cache_manager_basic_operations() {
        let manager = CacheManager::new();
        manager.put("test_key", &"test_value", Duration::from_secs(60)).expect("put");
        let val: Option<String> = manager.get("test_key").expect("get");
        assert_eq!(val, Some("test_value".to_string()));
        assert!(manager.has("test_key").expect("has"));
        manager.forget("test_key").expect("forget");
        assert!(!manager.has("test_key").expect("has"));
    }

    #[test]
    fn cache_manager_forever() {
        let manager = CacheManager::new();
        manager.forever("permanent", &42i32).expect("forever");
        let val: Option<i32> = manager.get("permanent").expect("get");
        assert_eq!(val, Some(42));
    }

    #[test]
    fn cache_manager_pull() {
        let manager = CacheManager::new();
        manager.put("pull_key", &"pull_value", Duration::from_secs(60)).expect("put");
        let val: Option<String> = manager.pull("pull_key").expect("pull");
        assert_eq!(val, Some("pull_value".to_string()));
        // Should be gone after pull
        let val2: Option<String> = manager.get("pull_key").expect("get");
        assert!(val2.is_none());
    }

    #[test]
    fn cache_manager_add() {
        let manager = CacheManager::new();
        manager.flush().expect("flush"); // clean state
        let added = manager.add("add_key", &"first", Duration::from_secs(60)).expect("add");
        assert!(added);
        let added2 = manager.add("add_key", &"second", Duration::from_secs(60)).expect("add");
        assert!(!added2);
        let val: Option<String> = manager.get("add_key").expect("get");
        assert_eq!(val, Some("first".to_string()));
    }

    // ── CacheWarmer ──────────────────────────────────────────────

    #[tokio::test]
    async fn cache_warmer() {
        let cache = MemoryCache::new();
        CacheWarmer::new(cache.clone())
            .warm("preloaded", Duration::from_secs(300), || async {
                Ok("warm_value".to_string())
            })
            .start()
            .await
            .expect("warm");

        let val: Option<String> = cache.get("preloaded").await.expect("get");
        assert_eq!(val, Some("warm_value".to_string()));
    }
}
