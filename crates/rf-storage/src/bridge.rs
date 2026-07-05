//! A reusable, deadlock-safe sync-over-async bridge for storage drivers.
//!
//! ## Why this exists
//!
//! The synchronous storage surface ([`crate::StorageFacade`], `Storage::put`/`get`
//! on a blocking caller) needs to reach *async* drivers — most importantly the
//! [`crate::S3Storage`] backend, whose every operation is `async` and depends on
//! the Tokio reactor to drive network I/O. Bridging sync → async naively is a
//! well-known footgun:
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
//! This mirrors the proven `rf_cache::bridge` implementation; it is kept local to
//! `rf-storage` so the storage crate does not take a dependency on the cache
//! crate for what is a tiny, self-contained utility.
//!
//! ```rust
//! use rf_storage::bridge::AsyncBridge;
//! use std::time::Duration;
//!
//! let bridge = AsyncBridge::new();
//! let n = bridge.block_on(async {
//!     tokio::time::sleep(Duration::from_millis(1)).await;
//!     2 + 2
//! });
//! assert_eq!(n, 4);
//! ```

use std::sync::Arc;

use crate::{Storage, StorageResult};

// The deadlock-safe sync-over-async core now lives in the shared
// `rf-async-bridge` crate (it was previously duplicated verbatim here and in
// `rf-cache` / `rf-mail`). Re-export it so `rf_storage::bridge::AsyncBridge` and
// the `rf_storage::AsyncBridge` re-export keep working unchanged; the
// crate-specific `BridgedStorage` wrapper below is built on top of it.
pub use rf_async_bridge::AsyncBridge;

/// A **synchronous** view over any async [`Storage`] driver, executed through an
/// [`AsyncBridge`].
///
/// This is how an async driver such as [`crate::S3Storage`] is exposed behind a
/// blocking, `.await`-free API without risking the sync-over-async deadlocks
/// described on [`AsyncBridge`]. The wrapped driver is held in an [`Arc`] so each
/// call can hand an owned, `'static` future to the worker thread (the driver
/// itself does not need to be `Clone`).
///
/// ```rust
/// use rf_storage::{bridge::BridgedStorage, MemoryStorage};
///
/// // Works over the async in-memory driver with no external services — the same
/// // code path used for the S3 driver behind [`BridgedStorage::connect_s3`].
/// let storage = BridgedStorage::new(MemoryStorage::new());
/// storage.put("k.txt", b"v".to_vec()).unwrap();
/// assert!(storage.exists("k.txt").unwrap());
/// assert_eq!(storage.get("k.txt").unwrap(), b"v");
/// ```
#[derive(Clone)]
pub struct BridgedStorage {
    inner: Arc<dyn Storage>,
    bridge: AsyncBridge,
}

impl BridgedStorage {
    /// Wrap an already-constructed async driver, spawning a fresh bridge worker.
    pub fn new<S: Storage + 'static>(storage: S) -> Self {
        Self {
            inner: Arc::new(storage),
            bridge: AsyncBridge::new(),
        }
    }

    /// Wrap a driver, reusing an existing [`AsyncBridge`] (share one worker
    /// thread across several bridged storages).
    pub fn with_bridge<S: Storage + 'static>(storage: S, bridge: AsyncBridge) -> Self {
        Self {
            inner: Arc::new(storage),
            bridge,
        }
    }

    /// Wrap an already-`Arc`'d driver (e.g. one shared elsewhere), reusing a bridge.
    pub fn from_arc(storage: Arc<dyn Storage>, bridge: AsyncBridge) -> Self {
        Self {
            inner: storage,
            bridge,
        }
    }

    /// The underlying async driver (for direct `async` use).
    pub fn inner(&self) -> &Arc<dyn Storage> {
        &self.inner
    }

    /// The bridge backing this storage (share it with [`Self::with_bridge`]).
    pub fn bridge(&self) -> &AsyncBridge {
        &self.bridge
    }

    /// Synchronously store `contents` at `path` through the async driver.
    pub fn put(&self, path: &str, contents: Vec<u8>) -> StorageResult<()> {
        let storage = self.inner.clone();
        let path = path.to_string();
        self.bridge
            .block_on(async move { storage.put(&path, contents).await })
    }

    /// Synchronously read the bytes stored at `path`.
    pub fn get(&self, path: &str) -> StorageResult<Vec<u8>> {
        let storage = self.inner.clone();
        let path = path.to_string();
        self.bridge
            .block_on(async move { storage.get(&path).await })
    }

    /// Synchronously delete the object at `path`.
    pub fn delete(&self, path: &str) -> StorageResult<()> {
        let storage = self.inner.clone();
        let path = path.to_string();
        self.bridge
            .block_on(async move { storage.delete(&path).await })
    }

    /// Synchronously check whether an object exists at `path`.
    pub fn exists(&self, path: &str) -> StorageResult<bool> {
        let storage = self.inner.clone();
        let path = path.to_string();
        self.bridge
            .block_on(async move { storage.exists(&path).await })
    }

    /// Synchronously get the size in bytes of the object at `path`.
    pub fn size(&self, path: &str) -> StorageResult<u64> {
        let storage = self.inner.clone();
        let path = path.to_string();
        self.bridge
            .block_on(async move { storage.size(&path).await })
    }

    /// Synchronously list object keys under `prefix`.
    pub fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let storage = self.inner.clone();
        let prefix = prefix.to_string();
        self.bridge
            .block_on(async move { storage.list(&prefix).await })
    }

    /// The public URL for `path` (delegates straight to the driver; not async).
    pub fn url(&self, path: &str) -> String {
        self.inner.url(path)
    }
}

impl BridgedStorage {
    /// Construct an **S3-backed** storage and expose it behind the **sync** API,
    /// driving the async client construction and all operations through the bridge.
    ///
    /// This is the config-gated S3 wiring: it requires real S3/MinIO credentials
    /// and an endpoint/region in [`crate::S3Config`]. Building the client here does
    /// not perform network I/O, but the returned handle's `put`/`get`/`delete`/
    /// `exists` calls talk to a live bucket — so an **end-to-end test needs a
    /// running S3 or MinIO** (see `crates/rf-storage/src/s3.rs` tests, which skip
    /// when `127.0.0.1:9000` is unreachable). The offline deadlock-safety of the
    /// bridge itself is proven against the async `MemoryStorage` driver in this
    /// module's tests and in the sandbox probe.
    ///
    /// ```no_run
    /// use rf_storage::{bridge::BridgedStorage, S3Config};
    ///
    /// # fn demo() -> rf_storage::StorageResult<()> {
    /// let cfg = S3Config {
    ///     bucket: "my-bucket".into(),
    ///     region: "us-east-1".into(),
    ///     endpoint: Some("http://localhost:9000".into()),
    ///     access_key: "minioadmin".into(),
    ///     secret_key: "minioadmin".into(),
    ///     path_style: true,
    /// };
    /// let s3 = BridgedStorage::connect_s3(cfg)?;
    /// s3.put("hello.txt", b"world".to_vec())?; // -> live bucket via the bridge
    /// # Ok(())
    /// # }
    /// ```
    pub fn connect_s3(config: crate::S3Config) -> StorageResult<Self> {
        let bridge = AsyncBridge::new();
        let s3 = bridge.block_on(async move { crate::S3Storage::new(config).await })?;
        Ok(Self {
            inner: Arc::new(s3),
            bridge,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryStorage;
    use std::sync::atomic::{AtomicU32, Ordering};
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

    // --- BridgedStorage over the async MemoryStorage driver (no external services) ---

    #[test]
    fn bridged_memory_storage_roundtrip_from_sync() {
        // Same code path S3 uses, exercised against the async in-memory driver.
        let storage = BridgedStorage::new(MemoryStorage::new());
        assert!(!storage.exists("k.txt").unwrap());
        storage.put("k.txt", b"value".to_vec()).unwrap();
        assert!(storage.exists("k.txt").unwrap());
        assert_eq!(storage.get("k.txt").unwrap(), b"value");
        assert_eq!(storage.size("k.txt").unwrap(), 5);
        storage.delete("k.txt").unwrap();
        assert!(!storage.exists("k.txt").unwrap());
        assert!(storage.get("k.txt").is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridged_memory_storage_roundtrip_from_inside_runtime() {
        // The whole point: a SYNC storage call routed through the async driver,
        // invoked from inside a live Tokio runtime, must not deadlock.
        let out = tokio::task::spawn_blocking(|| {
            let storage = BridgedStorage::new(MemoryStorage::new());
            storage.put("nested/a.bin", vec![1, 2, 3, 4]).unwrap();
            storage.get("nested/a.bin").unwrap()
        })
        .await
        .unwrap();
        assert_eq!(out, vec![1, 2, 3, 4]);
    }

    #[test]
    fn bridged_storage_shares_one_bridge() {
        // Two bridged storages over one worker thread; both round-trip.
        let bridge = AsyncBridge::new();
        let a = BridgedStorage::with_bridge(MemoryStorage::new(), bridge.clone());
        let b = BridgedStorage::with_bridge(MemoryStorage::new(), bridge);
        a.put("x", b"a".to_vec()).unwrap();
        b.put("x", b"b".to_vec()).unwrap();
        assert_eq!(a.get("x").unwrap(), b"a");
        assert_eq!(b.get("x").unwrap(), b"b");
    }
}
