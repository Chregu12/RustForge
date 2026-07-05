//! A deadlock-safe sync-over-async bridge for the mail layer.
//!
//! ## Why this exists
//!
//! RustForge's `Mail` facade is a **synchronous** API (`Mail::to(..).send(..)`),
//! but the real SMTP driver is `async` (lettre's [`AsyncSmtpTransport`]). Bridging
//! sync → async naively is a well-known footgun:
//!
//! - [`tokio::runtime::Runtime::block_on`] from a thread already inside a Tokio
//!   runtime panics with *"Cannot start a runtime from within a runtime"*.
//! - `futures::executor::block_on` on a Tokio future can deadlock, because the
//!   future depends on Tokio's reactor/timer which isn't being driven.
//!
//! [`AsyncBridge`] sidesteps both problems by owning **one dedicated, long-lived
//! OS thread** running its own *current-thread* Tokio runtime. Sync callers submit
//! an async job over a channel and block on a reply channel. Because the future
//! runs on a *separate* thread with a *separate* runtime, blocking the caller
//! thread never blocks the executor making progress — so [`AsyncBridge::block_on`]
//! is safe from **plain sync code** *and* from **inside an existing Tokio runtime**
//! (a request handler, a spawned task, `#[tokio::main]`).
//!
//! ## Provenance
//!
//! This is a faithful copy of the proven pattern in `rf-cache`'s
//! `crate::bridge::AsyncBridge`. A shared `rf-bridge` crate to host this single
//! implementation is a natural **future consolidation**; it is duplicated here for
//! now to avoid an inter-crate dependency for one small type.

use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc as std_mpsc;
use std::thread::JoinHandle;

use tokio::sync::mpsc as tokio_mpsc;

use crate::backends::{SmtpConfig, SmtpMailer};
use crate::{Mail, MailResult, Mailer};

/// A type-erased, `Send`, `'static` unit-future executed on the bridge worker.
type BridgeJob = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// A deadlock-safe sync-over-async bridge backed by a dedicated worker thread.
///
/// Clone cheaply: clones share the same worker thread and job queue.
///
/// ```rust
/// use rf_mail::bridge::AsyncBridge;
/// use std::time::Duration;
///
/// let bridge = AsyncBridge::new();
/// let n = bridge.block_on(async {
///     tokio::time::sleep(Duration::from_millis(1)).await;
///     2 + 2
/// });
/// assert_eq!(n, 4);
/// ```
#[derive(Clone)]
pub struct AsyncBridge {
    sender: tokio_mpsc::UnboundedSender<BridgeJob>,
    // Held purely for its `Drop`: shared so cloning the bridge doesn't join the
    // thread twice, and so the worker is only torn down once the last handle drops.
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
    /// Spawn the dedicated worker thread and its current-thread Tokio runtime.
    ///
    /// # Panics
    ///
    /// Panics only if the OS refuses to spawn the worker thread or the worker
    /// cannot build its Tokio runtime — both unrecoverable setup failures.
    pub fn new() -> Self {
        let (sender, mut receiver) = tokio_mpsc::unbounded_channel::<BridgeJob>();

        let handle = std::thread::Builder::new()
            .name("rf-mail-async-bridge".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("rf-mail async bridge failed to build its runtime");

                rt.block_on(async move {
                    // Drive each submitted job as a task so multiple in-flight jobs
                    // make progress concurrently on this single thread.
                    while let Some(job) = receiver.recv().await {
                        tokio::spawn(job);
                    }
                });
            })
            .expect("rf-mail async bridge failed to spawn its worker thread");

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
    /// runtime: unlike `Runtime::block_on`, it never touches the caller's ambient
    /// runtime, so it cannot panic with "runtime within a runtime" nor deadlock
    /// the executor.
    ///
    /// # Panics
    ///
    /// Panics if the worker thread has died (e.g. the submitted future panicked
    /// before replying), surfacing the failure rather than blocking forever.
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
            .expect("rf-mail async bridge worker is not running");

        reply_rx
            .recv()
            .expect("rf-mail async bridge worker dropped the reply (job panicked?)")
    }
}

impl Default for AsyncBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// A **synchronous** view over any async [`Mailer`] driver, executed through an
/// [`AsyncBridge`].
///
/// This is how an async driver such as the SMTP [`SmtpMailer`] is exposed behind a
/// blocking API without risking the sync-over-async deadlocks described on
/// [`AsyncBridge`]. The wrapped driver must be `Clone` (lettre's transport is a
/// cheap `Arc`-backed clone) so each call can hand an owned, `'static` future to
/// the worker thread.
///
/// ```rust
/// use rf_mail::bridge::BridgedMailer;
/// use rf_mail::{FileMailer, MailBuilder, Address};
///
/// # fn demo() -> rf_mail::MailResult<()> {
/// // Works over the async FileMailer with no external services — the same code
/// // path the SMTP driver uses, just to the filesystem transport.
/// let dir = std::env::temp_dir().join("rf-mail-bridge-doctest");
/// let mailer = BridgedMailer::new(FileMailer::new(&dir));
/// let mail = MailBuilder::new()
///     .from(Address::new("noreply@example.com"))
///     .to(Address::new("user@example.com"))
///     .subject("Hi")
///     .text("Hello from the bridge")
///     .build()?;
/// mailer.deliver(mail)?; // sync call, routed through the async driver
/// # let _ = std::fs::remove_dir_all(&dir);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct BridgedMailer<M: Mailer + Clone + 'static> {
    mailer: M,
    bridge: AsyncBridge,
}

impl<M: Mailer + Clone + 'static> BridgedMailer<M> {
    /// Wrap an already-constructed async driver, spawning a fresh bridge worker.
    pub fn new(mailer: M) -> Self {
        Self {
            mailer,
            bridge: AsyncBridge::new(),
        }
    }

    /// Wrap a driver, reusing an existing [`AsyncBridge`] (share one worker thread
    /// across several bridged mailers).
    pub fn with_bridge(mailer: M, bridge: AsyncBridge) -> Self {
        Self { mailer, bridge }
    }

    /// The underlying async driver (for direct `async` use).
    pub fn inner(&self) -> &M {
        &self.mailer
    }

    /// The bridge backing this mailer (share it with [`Self::with_bridge`]).
    pub fn bridge(&self) -> &AsyncBridge {
        &self.bridge
    }

    /// Synchronously deliver a message through the async driver.
    ///
    /// Blocks the calling thread until the async `send` completes. Safe from plain
    /// sync code and from inside a running Tokio runtime.
    pub fn deliver(&self, mail: Mail) -> MailResult<()> {
        let mailer = self.mailer.clone();
        self.bridge.block_on(async move { mailer.send(mail).await })
    }
}

/// A synchronous SMTP mailer: real lettre [`AsyncSmtpTransport`] delivery exposed
/// behind a blocking API via the [`AsyncBridge`].
///
/// [`AsyncSmtpTransport`]: lettre::AsyncSmtpTransport
pub type BridgedSmtpMailer = BridgedMailer<SmtpMailer>;

impl BridgedMailer<SmtpMailer> {
    /// Build a bridged SMTP mailer from an [`SmtpConfig`], spawning a dedicated
    /// bridge worker and constructing the async transport on it.
    ///
    /// Building the transport is cheap and does **not** open a network
    /// connection: lettre connects lazily on the first [`Self::deliver`] call. A
    /// live SMTP server is therefore only required to actually *send* — see the
    /// crate tests, which prove the bridge + message construction offline against
    /// the filesystem transport instead.
    pub fn connect_smtp(config: SmtpConfig) -> MailResult<Self> {
        let bridge = AsyncBridge::new();
        let mailer = bridge.block_on(async move { SmtpMailer::new(config).await })?;
        Ok(Self { mailer, bridge })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::FileMailer;
    use crate::{Address, MailBuilder};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // --- Core bridge deadlock-safety (mirrors rf-cache's proven pattern) ---

    #[test]
    fn block_on_from_plain_sync_runs_real_async_op() {
        let bridge = AsyncBridge::new();
        let result = bridge.block_on(async {
            // A REAL async op: the Tokio timer must be driven by the worker runtime.
            tokio::time::sleep(Duration::from_millis(20)).await;
            21 * 2
        });
        assert_eq!(result, 42);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn block_on_from_inside_tokio_runtime_does_not_deadlock() {
        let bridge = AsyncBridge::new();
        // Call the BLOCKING bridge from inside a running Tokio runtime. With a
        // naive Runtime::block_on this panics; here it must return the value.
        let out = tokio::task::spawn_blocking(move || {
            bridge.block_on(async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                "from-inside-runtime"
            })
        })
        .await
        .unwrap();
        assert_eq!(out, "from-inside-runtime");
    }

    #[test]
    fn concurrent_jobs_make_progress_together() {
        // Prove the worker drives jobs concurrently: 8 jobs each sleeping 100ms
        // should finish well under 800ms if they overlap on the runtime.
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

    // --- BridgedMailer over the async FileMailer driver (no external services) ---
    // This is the offline proof for the SMTP path: identical bridge + Mailer::send
    // code, delivering to the filesystem transport instead of a live SMTP server.

    fn sample_mail(dir_marker: &str) -> Mail {
        MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new("user@example.com"))
            .subject(format!("Bridged {dir_marker}"))
            .text("Hello through the deadlock-safe bridge")
            .build()
            .unwrap()
    }

    #[test]
    fn bridged_file_delivery_from_plain_sync_writes_eml() {
        let dir = std::env::temp_dir().join(format!("rf-mail-bridge-sync-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let mailer = BridgedMailer::new(FileMailer::new(&dir));
        mailer.deliver(sample_mail("sync")).unwrap();

        let entries: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert_eq!(entries.len(), 1, "bridge must have delivered exactly one .eml");
        let eml = std::fs::read_to_string(entries[0].as_ref().unwrap().path()).unwrap();
        assert!(eml.contains("Subject: Bridged sync"), "eml:\n{eml}");
        assert!(eml.contains("Hello through the deadlock-safe bridge"), "eml:\n{eml}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridged_file_delivery_from_inside_runtime_does_not_deadlock() {
        // The whole point: a SYNC deliver routed through the async driver, invoked
        // from inside a live Tokio runtime, must not deadlock.
        let dir = std::env::temp_dir().join(format!("rf-mail-bridge-rt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let dir2 = dir.clone();

        tokio::task::spawn_blocking(move || {
            let mailer = BridgedMailer::new(FileMailer::new(&dir2));
            mailer.deliver(sample_mail("rt")).unwrap();
        })
        .await
        .unwrap();

        let count = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn connect_smtp_builds_transport_without_connecting() {
        // Building the SMTP transport must succeed offline (lettre connects lazily
        // on send). This constructs a REAL AsyncSmtpTransport through the bridge;
        // no network I/O happens until deliver() is called.
        let mailer = BridgedSmtpMailer::connect_smtp(SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            username: "user".into(),
            password: "secret".into(),
            from_address: "noreply@example.com".into(),
            from_name: Some("RustForge".into()),
        });
        assert!(mailer.is_ok(), "SMTP transport must build offline");
    }
}
