//! Offline background jobs — the batteries-included, **no-Redis** path.
//!
//! RustForge ships two crates that export a `Job` trait:
//!
//! - `rf-jobs` — the Redis-backed queue (`QueueManager`/`WorkerPool`). The
//!   `jobs-demo` example uses it and early-returns if Redis is unreachable.
//! - `rf-queue` — the **batteries-included offline path**: an in-memory
//!   [`MemoryQueue`], the process-global [`Jobs`] facade, and a [`Worker`] you
//!   can drain in-process. **No Redis, no external services.**
//!
//! This example is the missing counterpart to `jobs-demo`: it dispatches and
//! runs a real job to completion on the in-memory queue. Run it with:
//!
//! ```text
//! cargo run -p jobs-offline
//! ```
//!
//! and it prints the processed jobs and exits — nothing to install.

use async_trait::async_trait;
use rf_queue::{Job, Jobs, MemoryQueue, Queue, QueueError, Worker};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A trivial job that "processes" an uploaded file. `handle` is where the real
/// work would go (resize an image, transcode a video, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessUpload {
    file: String,
}

#[async_trait]
impl Job for ProcessUpload {
    async fn handle(&self) -> Result<(), QueueError> {
        println!("  processing upload: {}", self.file);
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "process_upload"
    }
}

/// Dispatch a couple of jobs onto the in-memory queue and drain them with a
/// worker. Shared by `main` and the integration test so the offline path is
/// actually exercised, not just compiled.
async fn run_offline_jobs() -> Result<usize, QueueError> {
    // OFFLINE: a process-local, in-memory queue. No Redis, no config.
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    // Install it as the process-global default so the handle-free `Jobs` facade
    // (and `Job::dispatch_now`) enqueue onto it from anywhere.
    Jobs::set_queue(Arc::clone(&queue));

    // Dispatch from anywhere with no queue handle threaded through.
    Jobs::dispatch(ProcessUpload { file: "avatar.png".into() })?;
    ProcessUpload { file: "invoice.pdf".into() }.dispatch_now()?;

    // Drain the queue in-process with a worker that runs each job's `handle()`
    // for real. `work_once` returns Ok(false) once the queue is empty.
    let worker = Worker::new(queue).register::<ProcessUpload>();
    let mut processed = 0usize;
    while worker.work_once().await? {
        processed += 1;
    }
    Ok(processed)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("rf-queue offline jobs demo (no Redis required)");
    println!("dispatching jobs onto an in-memory queue...");

    let processed = run_offline_jobs().await?;

    println!("done — {processed} job(s) processed offline.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Process-wide side-effect sink: `Job::handle(&self)` has no external state
    // handle, so a global counter is how we prove the body actually ran.
    static PROCESSED: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct CountingJob {
        n: u64,
    }

    #[async_trait]
    impl Job for CountingJob {
        async fn handle(&self) -> Result<(), QueueError> {
            PROCESSED.fetch_add(self.n as usize, Ordering::SeqCst);
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "counting_job"
        }
    }

    /// The offline path must run dispatched jobs to completion with NO Redis.
    ///
    /// Both scenarios run in a single sequential test: `Jobs::set_queue`/
    /// `Jobs::dispatch` operate on ONE process-global default queue, so parallel
    /// `#[test]`s would race on it (each other's jobs land on whichever queue was
    /// installed last). Running sequentially keeps every dispatch deterministic.
    #[tokio::test]
    async fn offline_worker_runs_dispatched_jobs_to_completion() {
        // (1) Handle-free facade dispatch + worker drain, proving `handle()` ran.
        PROCESSED.store(0, Ordering::SeqCst);
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        Jobs::set_queue(Arc::clone(&queue));

        Jobs::dispatch(CountingJob { n: 2 }).unwrap();
        Jobs::dispatch(CountingJob { n: 5 }).unwrap();

        let worker = Worker::new(queue).register::<CountingJob>();
        // Drain: both jobs run, then the empty queue is a safe no-op.
        assert!(worker.work_once().await.unwrap());
        assert!(worker.work_once().await.unwrap());
        assert!(!worker.work_once().await.unwrap());
        // Both bodies executed to completion (2 + 5).
        assert_eq!(PROCESSED.load(Ordering::SeqCst), 7);

        // (2) The example's own `run_offline_jobs` helper processes both uploads.
        let processed = run_offline_jobs().await.unwrap();
        assert_eq!(processed, 2, "both dispatched uploads should be processed");
    }
}
