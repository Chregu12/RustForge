//! A reusable, deadlock-safe sync-over-async bridge.
//!
//! ## Why this exists
//!
//! The cache facade is a *synchronous* API (`Cache::get`, `Cache::put`, …), but
//! the real drivers ([`crate::MemoryCache`], [`crate::RedisCache`]) are `async`.
//! Bridging sync → async naively is a well-known footgun:
//!
//! - Calling [`tokio::runtime::Runtime::block_on`] from a thread that is *already*
//!   running inside a Tokio runtime panics with
//!   *"Cannot start a runtime from within a runtime"*.
//! - `futures::executor::block_on` on a Tokio future can deadlock, because the
//!   future may depend on Tokio's reactor/timer which isn't being driven.
//!
//! [`AsyncBridge`] sidesteps both problems. It owns **one dedicated, long-lived
//! OS thread** running its own *current-thread* Tokio runtime. Sync callers submit
//! an async job over a channel and block on a reply channel. Because the future
//! runs on a *separate* thread with a *separate* runtime, blocking the caller
//! thread never blocks the executor that is making progress — so it is safe to
//! call [`AsyncBridge::block_on`] from **plain sync code** *and* from **inside an
//! existing Tokio runtime** (a request handler, a spawned task, `#[tokio::main]`).
//!
//! ```rust
//! use rf_cache::bridge::AsyncBridge;
//! use std::time::Duration;
//!
//! let bridge = AsyncBridge::new();
//!
//! // From plain sync context:
//! let n = bridge.block_on(async {
//!     tokio::time::sleep(Duration::from_millis(1)).await;
//!     2 + 2
//! });
//! assert_eq!(n, 4);
//! ```

// The deadlock-safe sync-over-async core now lives in the shared
// `rf-async-bridge` crate (it was previously duplicated verbatim here and in
// `rf-mail` / `rf-storage`). Re-export it so `rf_cache::bridge::AsyncBridge` and
// the `rf_cache::AsyncBridge` re-export keep working unchanged; the
// crate-specific `BridgedCache` wrapper below is built on top of it.
pub use rf_async_bridge::AsyncBridge;

use serde::{de::DeserializeOwned, Serialize};

use crate::{Cache, CacheResult};

/// A **synchronous** view over any async [`Cache`] driver, executed through an
/// [`AsyncBridge`].
///
/// This is how an async driver such as [`crate::RedisCache`] is exposed behind a
/// blocking API without risking the sync-over-async deadlocks described on
/// [`AsyncBridge`]. The wrapped driver must be `Clone` (all RustForge drivers are
/// cheap `Arc`-backed clones) so each call can hand an owned, `'static` future to
/// the worker thread.
///
/// ```rust
/// use rf_cache::{bridge::BridgedCache, MemoryCache};
///
/// // Works over the in-memory driver with no external services — same code path
/// // used for the Redis driver behind the `redis-backend` feature.
/// let cache = BridgedCache::new(MemoryCache::new());
/// cache.put("k", &"v".to_string(), 60).unwrap();
/// let v: Option<String> = cache.get("k").unwrap();
/// assert_eq!(v, Some("v".to_string()));
/// ```
#[derive(Clone)]
pub struct BridgedCache<C: Cache + Clone + Send + Sync + 'static> {
    cache: C,
    bridge: AsyncBridge,
}

impl<C: Cache + Clone + Send + Sync + 'static> BridgedCache<C> {
    /// Wrap an already-constructed async driver, spawning a fresh bridge worker.
    pub fn new(cache: C) -> Self {
        Self {
            cache,
            bridge: AsyncBridge::new(),
        }
    }

    /// Wrap a driver, reusing an existing [`AsyncBridge`] (share one worker
    /// thread across several bridged caches).
    pub fn with_bridge(cache: C, bridge: AsyncBridge) -> Self {
        Self { cache, bridge }
    }

    /// The underlying async driver (for direct `async` use).
    pub fn inner(&self) -> &C {
        &self.cache
    }

    /// The bridge backing this cache (share it with `with_bridge`).
    pub fn bridge(&self) -> &AsyncBridge {
        &self.bridge
    }

    /// Synchronously get a value from the async driver.
    pub fn get<T: DeserializeOwned + Send + 'static>(&self, key: &str) -> CacheResult<Option<T>> {
        let cache = self.cache.clone();
        let key = key.to_string();
        self.bridge.block_on(async move { cache.get::<T>(&key).await })
    }

    /// Synchronously set a value with a TTL in seconds (or a `Duration`).
    ///
    /// The value is serialized to an owned [`serde_json::Value`] on the calling
    /// thread (so the future handed to the worker is `'static`), then stored
    /// through the driver — its own key-prefixing / TTL semantics still apply.
    pub fn put<T: Serialize + Send + Sync + 'static>(
        &self,
        key: &str,
        value: &T,
        ttl: impl crate::IntoTtl,
    ) -> CacheResult<()> {
        let cache = self.cache.clone();
        let key = key.to_string();
        let ttl = ttl.into_duration();
        let json = serde_json::to_value(value)
            .map_err(|e| crate::CacheError::Serialization(e.to_string()))?;
        self.bridge
            .block_on(async move { cache.set(&key, &json, ttl).await })
    }

    /// Synchronously delete a key.
    pub fn forget(&self, key: &str) -> CacheResult<()> {
        let cache = self.cache.clone();
        let key = key.to_string();
        self.bridge.block_on(async move { cache.delete(&key).await })
    }

    /// Synchronously check whether a key exists.
    pub fn has(&self, key: &str) -> CacheResult<bool> {
        let cache = self.cache.clone();
        let key = key.to_string();
        self.bridge.block_on(async move { cache.exists(&key).await })
    }

    /// Synchronously extend a key's expiry to `now + ttl` without rewriting it.
    pub fn touch(&self, key: &str, ttl: impl crate::IntoTtl) -> CacheResult<bool> {
        let cache = self.cache.clone();
        let key = key.to_string();
        let ttl = ttl.into_duration();
        self.bridge.block_on(async move { cache.touch(&key, ttl).await })
    }

    /// Synchronously clear the whole backend.
    pub fn flush(&self) -> CacheResult<()> {
        let cache = self.cache.clone();
        self.bridge.block_on(async move { cache.flush().await })
    }
}

#[cfg(feature = "redis-backend")]
impl BridgedCache<crate::RedisCache> {
    /// Construct a Redis-backed cache and expose it behind the **sync** API,
    /// driving the async connect + all operations through the bridge.
    ///
    /// This performs a real connection (Redis `PING`) via the bridge from a sync
    /// context, so it requires a live `redis-server`. Gated behind the
    /// `redis-backend` feature.
    ///
    /// ```no_run
    /// # #[cfg(feature = "redis-backend")]
    /// # fn demo() -> Result<(), rf_cache::CacheError> {
    /// use rf_cache::bridge::BridgedCache;
    /// let cache = BridgedCache::connect_redis("redis://localhost:6379", "myapp")?;
    /// cache.put("k", &"v".to_string(), 60)?;
    /// let v: Option<String> = cache.get("k")?;
    /// # let _ = v; Ok(())
    /// # }
    /// ```
    pub fn connect_redis(url: &str, prefix: &str) -> CacheResult<Self> {
        let bridge = AsyncBridge::new();
        let url = url.to_string();
        let prefix = prefix.to_string();
        let cache = bridge.block_on(async move { crate::RedisCache::new(&url, &prefix).await })?;
        Ok(Self { cache, bridge })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // --- Core bridge correctness / deadlock-safety ---

    #[test]
    fn block_on_from_plain_sync_runs_real_async_op() {
        let bridge = AsyncBridge::new();
        let result = bridge.block_on(async {
            // A REAL async op: Tokio timer must be driven by the worker runtime.
            tokio::time::sleep(Duration::from_millis(20)).await;
            21 * 2
        });
        assert_eq!(result, 42);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn block_on_from_inside_tokio_runtime_does_not_deadlock() {
        let bridge = AsyncBridge::new();
        // Call the BLOCKING bridge from inside a spawned Tokio task. With a naive
        // Runtime::block_on this panics; here it must return the value.
        let handle = tokio::task::spawn_blocking(move || {
            bridge.block_on(async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                "from-inside-runtime"
            })
        });
        let out = handle.await.unwrap();
        assert_eq!(out, "from-inside-runtime");
    }

    #[test]
    fn concurrent_jobs_make_progress_together() {
        // Prove the worker drives jobs concurrently (each sleeps 100ms; 8 of them
        // should finish well under 800ms if they overlap on the runtime).
        let bridge = AsyncBridge::new();
        let counter = Arc::new(AtomicU32::new(0));
        let start = std::time::Instant::now();
        let mut threads = Vec::new();
        for _ in 0..8 {
            let b = bridge.clone();
            let c = counter.clone();
            threads.push(std::thread::spawn(move || {
                b.block_on(async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    c.fetch_add(1, Ordering::SeqCst);
                });
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 8);
        assert!(
            start.elapsed() < Duration::from_millis(600),
            "jobs did not overlap: {:?}",
            start.elapsed()
        );
    }

    // --- BridgedCache over the async MemoryCache driver (no external services) ---

    #[test]
    fn bridged_memory_cache_roundtrip_from_sync() {
        let cache = BridgedCache::new(crate::MemoryCache::new());
        assert!(!cache.has("k").unwrap());
        cache.put("k", &"value".to_string(), 60u64).unwrap();
        let v: Option<String> = cache.get("k").unwrap();
        assert_eq!(v, Some("value".to_string()));
        assert!(cache.has("k").unwrap());
        cache.forget("k").unwrap();
        assert_eq!(cache.get::<String>("k").unwrap(), None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridged_memory_cache_roundtrip_from_inside_runtime() {
        // The whole point: a SYNC facade call routed through the async driver,
        // invoked from inside a live Tokio runtime, must not deadlock.
        let out = tokio::task::spawn_blocking(|| {
            let cache = BridgedCache::new(crate::MemoryCache::new());
            cache.put("k", &7i64, 60u64).unwrap();
            cache.get::<i64>("k").unwrap()
        })
        .await
        .unwrap();
        assert_eq!(out, Some(7));
    }

    #[test]
    fn bridged_memory_cache_touch_extends_ttl() {
        let cache = BridgedCache::new(crate::MemoryCache::new());
        cache.put("k", &"v".to_string(), Duration::from_millis(60)).unwrap();
        assert!(cache.touch("k", 10u64).unwrap());
        std::thread::sleep(Duration::from_millis(150));
        assert!(cache.has("k").unwrap(), "touch through bridge must extend lifetime");
    }
}
