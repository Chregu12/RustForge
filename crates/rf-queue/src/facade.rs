//! Process-global queue facade for ergonomic background dispatch.
//!
//! This is the sync-friendly, handle-free counterpart to
//! [`Job::dispatch`](crate::Job::dispatch). It mirrors the other RustForge
//! facades (`Mail`, `Cache`, `Storage`): a single process-global default
//! [`Queue`] is configured once (usually at boot) and jobs are then dispatched
//! from anywhere with no queue argument threaded through call sites:
//!
//! ```no_run
//! use rf_queue::{Jobs, MemoryQueue, Queue, QueueError, Job};
//! use async_trait::async_trait;
//! use serde::{Serialize, Deserialize};
//! use std::sync::Arc;
//!
//! #[derive(Serialize, Deserialize)]
//! struct SendInvoice { order_id: u64 }
//!
//! #[async_trait]
//! impl Job for SendInvoice {
//!     async fn handle(&self) -> Result<(), QueueError> { Ok(()) }
//!     fn job_type(&self) -> &'static str { "send_invoice" }
//! }
//!
//! // Configure the global default queue once at boot.
//! Jobs::set_queue(Arc::new(MemoryQueue::new()) as Arc<dyn Queue>);
//!
//! // Dispatch from anywhere — no queue handle, no raw block_on.
//! SendInvoice { order_id: 42 }.dispatch_now().unwrap();
//! // or, equivalently:
//! Jobs::dispatch(SendInvoice { order_id: 43 }).unwrap();
//! ```
//!
//! ## Why a bridge and not `block_on`
//!
//! [`Queue::push`](crate::Queue) is `async`, but the facade is **synchronous**
//! so it is callable from ordinary code. Naively wrapping it in
//! `Runtime::block_on` panics ("Cannot start a runtime from within a runtime")
//! when a caller is already inside a Tokio runtime (an Axum handler, a spawned
//! task, `#[tokio::main]`). The facade therefore drives the push on the
//! deadlock-safe [`AsyncBridge`], which runs the future on its own dedicated
//! runtime thread — safe to call **with or without** an ambient Tokio runtime.

use crate::error::QueueError;
use crate::job::{Job, JobMetadata};
use crate::memory::MemoryQueue;
use crate::queue::Queue;
use once_cell::sync::Lazy;
use rf_async_bridge::AsyncBridge;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// The process-global default queue backing the [`Jobs`] facade and
/// [`Job::dispatch_now`]. Defaults to an in-memory queue so the facade is usable
/// out of the box (e.g. in tests); replace it with a real backend at boot via
/// [`set_default_queue`] / [`Jobs::set_queue`].
static DEFAULT_QUEUE: Lazy<RwLock<Arc<dyn Queue>>> =
    Lazy::new(|| RwLock::new(Arc::new(MemoryQueue::new()) as Arc<dyn Queue>));

/// One shared, deadlock-safe bridge for the whole process so the synchronous
/// facade can drive async `Queue::push` from inside or outside a Tokio runtime.
static BRIDGE: Lazy<AsyncBridge> = Lazy::new(AsyncBridge::new);

/// Borrow the single process-global [`AsyncBridge`]. Shared with the
/// [`QueueFacade`](crate::QueueFacade) free-fn API so every synchronous queue
/// entry point drives its async operation on the *same* deadlock-safe worker
/// thread instead of spinning up a fresh runtime (or panicking on a raw
/// `block_on` from inside an ambient Tokio runtime).
pub(crate) fn shared_bridge() -> &'static AsyncBridge {
    &BRIDGE
}

/// Install the process-global default queue used by the handle-free dispatch
/// API ([`Jobs::dispatch`], [`Job::dispatch_now`]).
///
/// Call this once at application boot. Subsequent calls replace the default.
pub fn set_default_queue(queue: Arc<dyn Queue>) {
    *DEFAULT_QUEUE
        .write()
        .expect("rf-queue default queue lock poisoned") = queue;
}

/// Get a handle to the current process-global default queue.
///
/// Returns an in-memory queue until [`set_default_queue`] has been called.
pub fn default_queue() -> Arc<dyn Queue> {
    Arc::clone(
        &DEFAULT_QUEUE
            .read()
            .expect("rf-queue default queue lock poisoned"),
    )
}

/// Push already-built [`JobMetadata`] onto the global default queue, driving the
/// async push on the deadlock-safe bridge. Shared by the facade helpers and
/// [`Job::dispatch_now`].
pub(crate) fn push_to_default(metadata: JobMetadata) -> Result<String, QueueError> {
    let queue = default_queue();
    BRIDGE.block_on(async move { queue.push(metadata).await })
}

/// Laravel-style global facade for background job dispatch.
///
/// All methods operate on the process-global default queue configured with
/// [`Jobs::set_queue`]; no queue handle is threaded through call sites.
pub struct Jobs;

impl Jobs {
    /// Install the process-global default queue (alias for [`set_default_queue`]).
    pub fn set_queue(queue: Arc<dyn Queue>) {
        set_default_queue(queue);
    }

    /// Get a handle to the process-global default queue (alias for
    /// [`default_queue`]) — useful to build a [`Worker`](crate::Worker) against
    /// the same queue jobs are dispatched to.
    pub fn queue() -> Arc<dyn Queue> {
        default_queue()
    }

    /// Dispatch a job onto the global default queue with no queue handle.
    ///
    /// Synchronous and safe to call from inside or outside a Tokio runtime.
    pub fn dispatch<J: Job>(job: J) -> Result<String, QueueError> {
        let metadata = JobMetadata::new(&job)?;
        push_to_default(metadata)
    }

    /// Dispatch a delayed job onto the global default queue with no queue handle.
    pub fn dispatch_later<J: Job>(job: J, delay: Duration) -> Result<String, QueueError> {
        let metadata = JobMetadata::new_delayed(&job, delay)?;
        push_to_default(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static RUNS: AtomicUsize = AtomicUsize::new(0);
    static LAST: Mutex<String> = Mutex::new(String::new());

    #[derive(Serialize, Deserialize, Clone)]
    struct FacadeJob {
        msg: String,
    }

    #[async_trait]
    impl Job for FacadeJob {
        async fn handle(&self) -> Result<(), QueueError> {
            RUNS.fetch_add(1, Ordering::SeqCst);
            *LAST.lock().unwrap() = self.msg.clone();
            Ok(())
        }
        fn job_type(&self) -> &'static str {
            "facade_job"
        }
    }

    #[test]
    fn dispatch_now_uses_global_queue_no_handle() {
        RUNS.store(0, Ordering::SeqCst);
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        Jobs::set_queue(Arc::clone(&queue));

        // No queue handle threaded here — the facade uses the global default.
        FacadeJob {
            msg: "hello".into(),
        }
        .dispatch_now()
        .unwrap();
        Jobs::dispatch(FacadeJob { msg: "world".into() }).unwrap();

        // Both landed on the SAME global queue.
        let bridge = AsyncBridge::new();
        let size = bridge.block_on({
            let q = Arc::clone(&queue);
            async move { q.size("default").await.unwrap() }
        });
        assert_eq!(size, 2, "both jobs enqueued on the global default queue");

        // Drain via a worker and prove handle() actually ran.
        let worker = crate::Worker::new(Arc::clone(&queue)).register::<FacadeJob>();
        bridge.block_on(async move {
            assert!(worker.work_once().await.unwrap());
            assert!(worker.work_once().await.unwrap());
            assert!(!worker.work_once().await.unwrap());
        });
        assert_eq!(RUNS.load(Ordering::SeqCst), 2, "both job bodies executed");
        assert_eq!(&*LAST.lock().unwrap(), "world", "last payload round-tripped");
    }
}
