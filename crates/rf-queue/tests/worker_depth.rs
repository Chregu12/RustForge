//! Deep integration tests for rf-queue Worker, MemoryQueue, and Job lifecycle.
//!
//! Covers the following critical paths not exercised by inline unit tests:
//!   - dispatch -> Worker::work_once drains the queue (job body really executes)
//!   - Priority ordering: higher-priority jobs are dequeued before lower-priority
//!     ones regardless of insertion order; FIFO is the tiebreaker within a tier.
//!   - Retry → dead-letter: a persistently-failing job is retried up to
//!     `max_retries`, then moved to `Queue::failed()`; assert the dead-letter
//!     contents (error text, attempt count).
//!   - Panic isolation: a panicking job body does NOT kill the worker; the
//!     subsequent good job still executes; the panicking job appears in
//!     `Queue::failed()` rather than being silently lost.
//!   - Payload round-trip: a complex, unicode-rich payload survives the full
//!     `dispatch → push → reserve → deserialize` cycle with all fields intact.

use async_trait::async_trait;
use rf_queue::{Job, JobMetadata, MemoryQueue, Queue, QueueError, Worker};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Job types used across tests ──────────────────────────────────────────────

/// A trivially-successful job. Tests use queue-state assertions (not global
/// counters) to confirm execution so that tests are safe to run in parallel.
#[derive(Serialize, Deserialize, Clone, Debug)]
struct OkJob {
    label: String,
    /// Priority stored in the payload so `Job::priority` can vary per-instance.
    #[serde(default)]
    prio: i32,
}

#[async_trait]
impl Job for OkJob {
    async fn handle(&self) -> Result<(), QueueError> {
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "ok_job"
    }

    fn priority(&self) -> i32 {
        self.prio
    }
}

/// A job that always returns an error, used to drive retry → dead-letter paths.
#[derive(Serialize, Deserialize, Clone)]
struct AlwaysFailJob {
    msg: String,
}

#[async_trait]
impl Job for AlwaysFailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        Err(QueueError::JobFailed(self.msg.clone()))
    }

    fn job_type(&self) -> &'static str {
        "always_fail_job"
    }

    /// Two attempts total before dead-lettering (attempt 1 → retry, attempt 2
    /// → dead-letter).
    fn max_retries(&self) -> u32 {
        2
    }
}

/// A job whose `handle` body panics.  `max_retries = 0` so the caught panic
/// lands in the dead-letter immediately (one attempt, no retry loop).
#[derive(Serialize, Deserialize, Clone)]
struct PanicJob {
    panic_msg: String,
}

#[async_trait]
impl Job for PanicJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // This panic is caught by Worker::process_job via `catch_unwind`.
        panic!("{}", self.panic_msg);
    }

    fn job_type(&self) -> &'static str {
        "panic_job"
    }

    fn max_retries(&self) -> u32 {
        0 // caught panic → no retry → immediate dead-letter
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Deserialize the job payload stored inside `metadata`.
fn decode<J: Job>(meta: &JobMetadata) -> J {
    meta.deserialize::<J>().expect("payload round-trip failed")
}

// ── Tests ────────────────────────────────────────────────────────────────────

/// `Worker::work_once` on an empty queue is a non-destructive no-op: it returns
/// `Ok(false)` and the worker survives to process the next job.
#[tokio::test]
async fn work_once_on_empty_queue_returns_false() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    let worker = Worker::new(Arc::clone(&queue));

    let result = worker.work_once().await;
    assert!(result.is_ok(), "work_once on empty queue must not error");
    assert!(!result.unwrap(), "empty queue -> work_once returns false");
}

/// `Worker::work_once` dispatches to a job's `handle` implementation and marks
/// the job as completed (not left in the dead-letter or in-flight stores).
///
/// We verify execution via queue-state changes (not a global counter) so this
/// test is safe to run concurrently with other tests that register OkJob.
#[tokio::test]
async fn work_once_executes_job_body_and_completes() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    let worker = Worker::new(Arc::clone(&queue)).register::<OkJob>();

    OkJob { label: "test".into(), prio: 0 }
        .dispatch(&queue)
        .await
        .unwrap();
    assert_eq!(queue.size("default").await.unwrap(), 1, "pre-condition");

    let processed = worker.work_once().await.unwrap();
    assert!(processed, "work_once must return true when a job was processed");

    // Queue is drained (job was completed, not stuck in-flight or re-queued).
    assert_eq!(queue.size("default").await.unwrap(), 0, "queue empty after successful execution");

    // Not in dead-letter (job completed successfully).
    assert!(
        queue.failed().await.unwrap().is_empty(),
        "completed job must not appear in dead-letter"
    );

    // Follow-up call: no jobs → returns false (worker stays alive).
    assert!(!worker.work_once().await.unwrap(), "empty after drain");
}

/// Priority ordering: `Queue::reserve` returns the highest-priority job first,
/// regardless of insertion order.
#[tokio::test]
async fn priority_ordering_highest_wins() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    // Insert in order: priority 0 ("low"), then 10 ("high"), then 5 ("mid").
    // Expected dequeue order by priority: high(10), mid(5), low(0).
    for (label, prio) in [("low", 0i32), ("high", 10), ("mid", 5)] {
        let job = OkJob { label: label.into(), prio };
        let meta = JobMetadata::new(&job).unwrap();
        queue.push(meta).await.unwrap();
    }

    let first = queue.reserve("default").await.unwrap().expect("first job");
    assert_eq!(first.priority, 10, "highest priority dequeued first");
    queue.complete(&first.id).await.unwrap();

    let second = queue.reserve("default").await.unwrap().expect("second job");
    assert_eq!(second.priority, 5, "second highest priority next");
    queue.complete(&second.id).await.unwrap();

    let third = queue.reserve("default").await.unwrap().expect("third job");
    assert_eq!(third.priority, 0, "lowest priority last");
    queue.complete(&third.id).await.unwrap();

    assert!(queue.reserve("default").await.unwrap().is_none(), "queue now empty");
}

/// FIFO tiebreaker: among jobs with equal priority, insertion order is preserved.
#[tokio::test]
async fn fifo_tiebreak_within_same_priority() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    for label in ["alpha", "beta", "gamma"] {
        let job = OkJob { label: label.into(), prio: 0 };
        let meta = JobMetadata::new(&job).unwrap();
        queue.push(meta).await.unwrap();
    }

    let first = queue.reserve("default").await.unwrap().unwrap();
    assert_eq!(decode::<OkJob>(&first).label, "alpha", "FIFO: first inserted, first out");
    queue.complete(&first.id).await.unwrap();

    let second = queue.reserve("default").await.unwrap().unwrap();
    assert_eq!(decode::<OkJob>(&second).label, "beta");
    queue.complete(&second.id).await.unwrap();

    let third = queue.reserve("default").await.unwrap().unwrap();
    assert_eq!(decode::<OkJob>(&third).label, "gamma");
    queue.complete(&third.id).await.unwrap();
}

/// A mixed-priority queue with a multi-tier scenario: priority 10, 10, 5.
/// The two priority-10 jobs must come out in insertion order before the
/// priority-5 job — i.e. priority ordering + FIFO tiebreak together.
#[tokio::test]
async fn priority_and_fifo_mixed_tiers() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    for (label, prio) in [("first-10", 10i32), ("second-10", 10), ("only-5", 5)] {
        let job = OkJob { label: label.into(), prio };
        queue.push(JobMetadata::new(&job).unwrap()).await.unwrap();
    }

    let a = queue.reserve("default").await.unwrap().unwrap();
    assert_eq!(decode::<OkJob>(&a).label, "first-10", "first high-priority inserted");
    queue.complete(&a.id).await.unwrap();

    let b = queue.reserve("default").await.unwrap().unwrap();
    assert_eq!(decode::<OkJob>(&b).label, "second-10", "second high-priority inserted");
    queue.complete(&b.id).await.unwrap();

    let c = queue.reserve("default").await.unwrap().unwrap();
    assert_eq!(decode::<OkJob>(&c).label, "only-5", "lower-priority last");
    queue.complete(&c.id).await.unwrap();
}

/// A persistently-failing job is retried up to `max_retries` times, then
/// permanently dead-lettered. After exhaustion:
///   - `Queue::size("default")` is 0 (job is not re-queued)
///   - `Queue::failed()` has exactly one entry
///   - The dead-letter record carries the last error message and the correct
///     attempt count (== max_retries)
#[tokio::test]
async fn retry_exhausts_then_dead_letters() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    let worker = Worker::new(Arc::clone(&queue)).register::<AlwaysFailJob>();

    let job = AlwaysFailJob { msg: "intentional failure".into() };
    job.dispatch(&queue).await.unwrap();
    assert_eq!(queue.size("default").await.unwrap(), 1, "pre-condition: one job enqueued");

    // Attempt 1 (attempts becomes 1; 1 < 2 → retry → re-enqueued).
    let did_process = worker.work_once().await.unwrap();
    assert!(did_process, "first work_once processed a job");
    assert_eq!(
        queue.size("default").await.unwrap(), 1,
        "job must be re-enqueued after first failure"
    );
    assert!(
        queue.failed().await.unwrap().is_empty(),
        "no dead-letter entries after retriable failure"
    );

    // Attempt 2 (attempts becomes 2; 2 < 2 is false → dead-letter).
    let did_process = worker.work_once().await.unwrap();
    assert!(did_process, "second work_once processed a job");
    assert_eq!(
        queue.size("default").await.unwrap(), 0,
        "queue empty after dead-lettering"
    );

    // Queue is now truly drained.
    assert!(!worker.work_once().await.unwrap(), "no more jobs");

    // Dead-letter assertions.
    let failed = queue.failed().await.unwrap();
    assert_eq!(failed.len(), 1, "exactly one permanently-failed job");

    let dead = &failed[0];
    assert!(
        dead.last_error.is_some(),
        "dead-lettered job must record the last error"
    );
    assert!(
        dead.last_error
            .as_ref()
            .unwrap()
            .contains("intentional failure"),
        "error text must survive into the dead-letter store: {:?}",
        dead.last_error
    );
    assert_eq!(
        dead.attempts, 2,
        "job must have been attempted exactly max_retries (2) times; got {}",
        dead.attempts
    );
}

/// A job with `max_retries = 0` is dead-lettered after the very first failure,
/// without any retry.
#[tokio::test]
async fn zero_retries_dead_letters_immediately() {
    #[derive(Serialize, Deserialize, Clone)]
    struct ZeroRetryJob;

    #[async_trait]
    impl Job for ZeroRetryJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Err(QueueError::JobFailed("zero-retry fail".into()))
        }
        fn job_type(&self) -> &'static str { "zero_retry_job" }
        fn max_retries(&self) -> u32 { 0 }
    }

    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    let worker = Worker::new(Arc::clone(&queue)).register::<ZeroRetryJob>();

    ZeroRetryJob.dispatch(&queue).await.unwrap();

    // Single attempt: attempts=1, can_retry = (1 < 0) = false → dead-letter.
    worker.work_once().await.unwrap();

    assert_eq!(queue.size("default").await.unwrap(), 0, "not re-queued");

    let failed = queue.failed().await.unwrap();
    assert_eq!(failed.len(), 1, "dead-lettered immediately");
    assert_eq!(failed[0].attempts, 1, "only one attempt was made");
}

/// A panicking job body is caught by `Worker::work_once` via `catch_unwind`.
/// The worker must:
///   1. Return `Ok(true)` (not propagate the panic as an Err/unwind)
///   2. Dead-letter the panicking job with a message that references "panic"
///   3. Continue processing subsequent good jobs normally
///
/// Note: Rust's panic machinery prints a note to stderr even when the panic is
/// caught by `catch_unwind`; the "panicked at …" line in test output is expected
/// and does not indicate test failure.
///
/// This test intentionally avoids process-global side-effect counters (which
/// would race against other tests using the same job type in parallel). Instead
/// it observes only queue / dead-letter state, which is fully isolated per-test
/// because each test creates its own `MemoryQueue` instance.
#[tokio::test]
async fn panic_in_job_is_isolated_worker_survives() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    let worker = Worker::new(Arc::clone(&queue))
        .register::<PanicJob>()
        .register::<OkJob>();

    // Enqueue panic job first, good job second (same priority → FIFO).
    PanicJob { panic_msg: "deliberate test panic".into() }
        .dispatch(&queue)
        .await
        .unwrap();
    OkJob { label: "after-panic".into(), prio: 0 }
        .dispatch(&queue)
        .await
        .unwrap();

    assert_eq!(queue.size("default").await.unwrap(), 2, "pre-condition: 2 jobs");

    // --- First work_once: PanicJob runs and panics -------------------------
    let result = worker.work_once().await;
    assert!(
        result.is_ok(),
        "work_once must not propagate the job panic; got: {:?}",
        result.err()
    );
    assert!(result.unwrap(), "work_once returns true (a job was processed)");

    // Panic job should be in dead-letter; OkJob still waiting.
    let failed_after_panic = queue.failed().await.unwrap();
    assert_eq!(
        failed_after_panic.len(), 1,
        "panicking job must be dead-lettered, not silently dropped"
    );
    let dead = &failed_after_panic[0];
    assert!(
        dead.last_error
            .as_ref()
            .map(|e| e.to_lowercase().contains("panic"))
            .unwrap_or(false),
        "dead-letter error must mention 'panic': {:?}",
        dead.last_error
    );

    // Good job is still in the queue (panic recovery did not consume it).
    assert_eq!(
        queue.size("default").await.unwrap(), 1,
        "OkJob must still be in the queue after recovering from the panic"
    );

    // --- Second work_once: OkJob runs fine --------------------------------
    let ok = worker.work_once().await;
    assert!(
        ok.is_ok() && ok.unwrap(),
        "worker processed OkJob after recovering from the panic"
    );

    // Queue fully drained; dead-letter still has exactly one entry (PanicJob).
    assert_eq!(queue.size("default").await.unwrap(), 0, "queue drained");
    assert_eq!(
        queue.failed().await.unwrap().len(), 1,
        "only the panic job in dead-letter; OkJob completed successfully"
    );
}

/// The full payload of a dispatched job survives the
/// `JobMetadata::new → Queue::push → Queue::reserve → JobMetadata::deserialize`
/// cycle with every field bit-for-bit intact, including unicode, control
/// characters, and boundary integer values.
#[tokio::test]
async fn payload_round_trips_through_queue() {
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    struct RichPayload {
        text: String,
        count: u64,
        signed: i64,
        flag: bool,
        nested: Vec<String>,
    }

    #[async_trait]
    impl Job for RichPayload {
        async fn handle(&self) -> Result<(), QueueError> { Ok(()) }
        fn job_type(&self) -> &'static str { "rich_payload_job" }
    }

    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    let original = RichPayload {
        text: "héllo wörld \u{1F30D} \x00 \n \t".to_string(),
        count: u64::MAX,
        signed: i64::MIN,
        flag: true,
        nested: vec!["a".into(), "b with spaces".into(), "c\nwith\nnewlines".into()],
    };

    let meta = JobMetadata::new(&original).unwrap();
    let dispatched_id = meta.id.clone();

    queue.push(meta).await.unwrap();
    assert_eq!(queue.size("default").await.unwrap(), 1);

    let reserved = queue
        .reserve("default")
        .await
        .unwrap()
        .expect("must have the job we just pushed");

    // Job identity round-tripped.
    assert_eq!(reserved.id, dispatched_id, "job ID survived queue round-trip");
    assert_eq!(reserved.job_type, "rich_payload_job");

    // Full payload equality after deserialization.
    let restored: RichPayload = reserved.deserialize().unwrap();
    assert_eq!(restored, original, "payload must be bit-for-bit identical after round-trip");
    assert_eq!(restored.count, u64::MAX, "u64::MAX survives JSON round-trip");
    assert_eq!(restored.signed, i64::MIN, "i64::MIN survives JSON round-trip");
    assert!(restored.text.contains('\u{1F30D}'), "unicode (🌍) survives round-trip");
    assert!(restored.text.contains('\x00'), "null byte survives round-trip");
    assert_eq!(restored.nested.len(), 3, "vec length preserved");
}

/// `JobMetadata` itself can be serialized to bytes and deserialized back with
/// all metadata fields identical (covers `to_bytes`/`from_bytes` on the envelope).
#[tokio::test]
async fn job_metadata_envelope_round_trips() {
    let job = OkJob { label: "meta-rt".into(), prio: 7 };
    let meta = JobMetadata::new(&job).unwrap();
    let id = meta.id.clone();

    let bytes = meta.to_bytes().unwrap();
    assert!(!bytes.is_empty(), "serialized envelope must not be empty");

    let restored = JobMetadata::from_bytes(&bytes).unwrap();
    assert_eq!(restored.id, id);
    assert_eq!(restored.job_type, "ok_job");
    assert_eq!(restored.priority, 7, "priority survives envelope round-trip");
    assert_eq!(restored.max_retries, 3, "default max_retries in envelope");
    assert_eq!(restored.attempts, 0, "attempts start at 0");
    assert!(restored.execute_at.is_none(), "no delay -> execute_at is None");
}

/// `Queue::reserve` on an unknown / non-existent queue name returns `Ok(None)`
/// instead of an error.
#[tokio::test]
async fn reserve_from_nonexistent_queue_returns_none() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    let result = queue.reserve("definitely-does-not-exist").await;
    assert!(result.is_ok(), "reserving from a nonexistent queue must not error");
    assert!(result.unwrap().is_none(), "must return None, not an error");
}

/// A job dispatched to a named queue (not "default") is only visible in that
/// queue; `Queue::size("default")` stays zero.
#[tokio::test]
async fn named_queue_dispatch_is_isolated() {
    #[derive(Serialize, Deserialize, Clone)]
    struct EmailJob { to: String }

    #[async_trait]
    impl Job for EmailJob {
        async fn handle(&self) -> Result<(), QueueError> { Ok(()) }
        fn job_type(&self) -> &'static str { "email_job" }
        fn queue(&self) -> &str { "emails" }
    }

    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    let meta = JobMetadata::new(&EmailJob { to: "user@example.com".into() }).unwrap();
    // Note: meta.queue will be "emails" (from Job::queue())
    assert_eq!(meta.queue, "emails", "metadata picks up the custom queue name");
    queue.push(meta).await.unwrap();

    assert_eq!(queue.size("emails").await.unwrap(), 1, "job in emails queue");
    assert_eq!(queue.size("default").await.unwrap(), 0, "default queue not affected");

    // reserve from default returns nothing; emails queue has the job.
    assert!(queue.reserve("default").await.unwrap().is_none());
    assert!(queue.reserve("emails").await.unwrap().is_some());
}

/// `Queue::clear` removes all jobs from the named queue (including multiple)
/// without touching other queues.
#[tokio::test]
async fn clear_removes_all_jobs_in_named_queue() {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    for i in 0..5u32 {
        let job = OkJob { label: format!("job-{i}"), prio: 0 };
        queue.push(JobMetadata::new(&job).unwrap()).await.unwrap();
    }

    // Also add a job to a different queue to confirm isolation.
    let other_job = OkJob { label: "other".into(), prio: 0 };
    let mut other_meta = JobMetadata::new(&other_job).unwrap();
    other_meta.queue = "other".into();
    queue.push(other_meta).await.unwrap();

    assert_eq!(queue.size("default").await.unwrap(), 5);
    assert_eq!(queue.size("other").await.unwrap(), 1);

    queue.clear("default").await.unwrap();

    assert_eq!(queue.size("default").await.unwrap(), 0, "default queue cleared");
    assert_eq!(queue.size("other").await.unwrap(), 1, "other queue untouched");
}

/// `Queue::failed()` initially returns an empty list; entries only appear after
/// a job is explicitly failed (via `Queue::fail`) or permanently dead-lettered by
/// the Worker.
#[tokio::test]
async fn failed_list_is_empty_initially_and_grows_on_fail() {
    let queue = Arc::new(MemoryQueue::new());

    // Initially empty.
    assert!(queue.failed().await.unwrap().is_empty());

    // Directly call fail() (simulating the dead-letter path).
    let job = OkJob { label: "doomed".into(), prio: 0 };
    let meta = JobMetadata::new(&job).unwrap();
    let id = meta.id.clone();
    // Push and immediately reserve (so in-flight is populated) then fail.
    queue.push(meta).await.unwrap();
    let reserved = queue.reserve("default").await.unwrap().unwrap();
    queue.fail(&reserved.id, "simulated failure").await.unwrap();

    let failed = queue.failed().await.unwrap();
    assert_eq!(failed.len(), 1, "one failed job after explicit fail()");
    assert_eq!(failed[0].id, id, "failed record preserves the original job id");
    assert_eq!(
        failed[0].last_error.as_deref(),
        Some("simulated failure"),
        "error text preserved in dead-letter"
    );
}
