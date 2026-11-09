//! A reusable, deadlock-safe sync-over-async bridge.
//!
//! ## Why this exists
//!
//! Several RustForge facades expose a **synchronous** API (`Cache::get`,
//! `Mail::to(..).send(..)`, `Storage::put`, …) while their real drivers are
//! `async` (Redis, lettre SMTP, the AWS S3 client, …). Bridging sync → async
//! naively is a well-known footgun:
//!
//! - Calling [`tokio::runtime::Runtime::block_on`] from a thread that is *already*
//!   running inside a Tokio runtime panics with
//!   *"Cannot start a runtime from within a runtime"*.
//! - `futures::executor::block_on` on a Tokio future can deadlock, because the
//!   future may depend on Tokio's reactor/timer which isn't being driven.
//!
//! [`AsyncBridge::run`] awaits a reply without blocking the caller. HTTP and
//! async services should use it (or native async drivers). [`AsyncBridge::block_on`]
//! is an explicitly blocking compatibility API for synchronous code. Calling it
//! directly in a Tokio task blocks that worker, even though the dedicated runtime
//! continues making progress. Use `run` or `spawn_blocking` at async boundaries.
//!
//! This is the single, shared implementation extracted from what used to be
//! duplicated verbatim in `rf-cache`, `rf-mail` and `rf-storage`; each of those
//! crates now re-exports [`AsyncBridge`] from here and builds its own
//! sync-over-async wrapper (`BridgedCache`, `BridgedMailer`, `BridgedStorage`) on
//! top of it.
//!
//! ```rust
//! use rf_async_bridge::AsyncBridge;
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

use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc as std_mpsc;
use std::thread::JoinHandle;

use tokio::sync::mpsc as tokio_mpsc;

/// A type-erased, `Send`, `'static` unit-future executed on the bridge worker.
type BridgeJob = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// A deadlock-safe sync-over-async bridge backed by a dedicated worker thread.
///
/// Clone cheaply: clones share the same worker thread and job queue.
#[derive(Clone)]
pub struct AsyncBridge {
    sender: tokio_mpsc::UnboundedSender<BridgeJob>,
    // Held purely for its `Drop`: shared so cloning the bridge doesn't try to
    // join the thread twice, and so the worker is only torn down once the last
    // handle is dropped.
    #[allow(dead_code)]
    worker: std::sync::Arc<WorkerGuard>,
}

/// Joins the worker thread when the last [`AsyncBridge`] handle is dropped.
struct WorkerGuard {
    handle: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        // The `sender` inside `AsyncBridge` has already been dropped by the time
        // the last `Arc<WorkerGuard>` drops, so the worker's `recv()` returns
        // `None`, its loop exits and the runtime shuts down. Join to be tidy.
        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

impl AsyncBridge {
    /// Run I/O on the bridge runtime without blocking the caller's Tokio worker.
    /// Cancelling this future cancels the submitted job when it next yields.
    pub async fn run<F>(&self, fut: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (mut reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.sender
            .send(Box::pin(async move {
                tokio::select! {
                    _ = reply_tx.closed() => {},
                    output = fut => { let _ = reply_tx.send(output); }
                }
            }))
            .expect("rf-async-bridge worker is not running");
        reply_rx
            .await
            .expect("rf-async-bridge job dropped the reply")
    }

    /// Spawn the dedicated worker thread and its current-thread Tokio runtime.
    ///
    /// # Panics
    ///
    /// Panics only if the OS refuses to spawn the worker thread or the worker
    /// cannot build its Tokio runtime — both are unrecoverable setup failures.
    pub fn new() -> Self {
        let (sender, mut receiver) = tokio_mpsc::unbounded_channel::<BridgeJob>();

        let handle = std::thread::Builder::new()
            .name("rf-async-bridge".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("rf-async-bridge failed to build its runtime");

                rt.block_on(async move {
                    // Drive each submitted job as a task so multiple in-flight
                    // jobs make progress concurrently on this single thread.
                    while let Some(job) = receiver.recv().await {
                        tokio::spawn(job);
                    }
                });
            })
            .expect("rf-async-bridge failed to spawn its worker thread");

        Self {
            sender,
            worker: std::sync::Arc::new(WorkerGuard {
                handle: std::sync::Mutex::new(Some(handle)),
            }),
        }
    }

    /// Run `fut` to completion on the bridge worker and block the current thread
    /// until it produces a value.
    ///
    /// Safe to call from plain sync code *and* from inside a running Tokio
    /// runtime: unlike `Runtime::block_on`, it never touches the caller's
    /// ambient runtime, so it cannot panic with "runtime within a runtime" but it DOES block the calling worker. Use `run` from async code.
    ///
    /// # Panics
    ///
    /// Panics if the worker thread has died (e.g. the submitted future panicked
    /// before replying), surfacing the failure to the caller rather than
    /// blocking forever.
    pub fn block_on<F>(&self, fut: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (reply_tx, reply_rx) = std_mpsc::sync_channel::<F::Output>(1);

        let job: BridgeJob = Box::pin(async move {
            let output = fut.await;
            // Receiver may have gone away only if the caller was cancelled; ignore.
            let _ = reply_tx.send(output);
        });

        self.sender
            .send(job)
            .expect("rf-async-bridge worker is not running");

        reply_rx
            .recv()
            .expect("rf-async-bridge worker dropped the reply (job panicked?)")
    }
}

impl Default for AsyncBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test(flavor = "current_thread")]
    async fn async_bridge_keeps_the_callers_runtime_responsive() {
        let bridge = AsyncBridge::new();
        let progressed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let observed = progressed.clone();
        let (ran, ()) = tokio::join!(
            bridge.run(async move {
                tokio::time::sleep(Duration::from_millis(30)).await;
                observed.load(Ordering::SeqCst)
            }),
            async {
                progressed.store(true, Ordering::SeqCst);
            }
        );
        assert!(ran, "bridge I/O blocked the caller's only runtime worker");
    }

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
}
