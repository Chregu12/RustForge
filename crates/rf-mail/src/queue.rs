//! Queue integration for background mail sending

#[cfg(feature = "queue")]
use crate::{Mail, MailError, Mailer};
#[cfg(feature = "queue")]
use async_trait::async_trait;
#[cfg(feature = "queue")]
use rf_queue::{dispatch, Job, Jobs, Queue, QueueError, Worker};
#[cfg(feature = "queue")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "queue")]
use std::sync::Arc;

/// Outcome summary returned by [`MailQueue::process_report`].
///
/// Counts how many mail jobs in a single drain pass either delivered
/// successfully or were permanently dead-lettered after exhausting retries.
/// `delivered + dead_lettered` equals the number of jobs that were pending
/// in the queue at the start of the drain (ignoring any delayed jobs not yet
/// due).
///
/// # Backend note
///
/// `dead_lettered` is derived from the queue's
/// [`failed()`](rf_queue::Queue::failed) list. Backends that do not track
/// failed jobs (those that return an empty default) will always report
/// `dead_lettered = 0`; use the in-memory backend (or a backend that
/// overrides `failed()`) for accurate dead-letter counts.
#[cfg(feature = "queue")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryReport {
    /// Number of mail jobs that delivered successfully in this drain pass.
    pub delivered: usize,
    /// Number of mail jobs that exhausted retries and were dead-lettered.
    pub dead_lettered: usize,
}

#[cfg(feature = "queue")]
/// Mail queue for background sending.
///
/// Fully self-contained: both [`push`](MailQueue::push) and
/// [`process`](MailQueue::process) operate on the `Arc<dyn Queue>` injected at
/// construction. No process-global [`rf_queue::DEFAULT_QUEUE`] is mutated, so
/// **multiple `MailQueue` instances can coexist concurrently** without racing
/// each other (e.g. in parallel tests or multi-tenant setups).
pub struct MailQueue {
    queue: Arc<dyn Queue>,
    mailer: Arc<dyn Mailer>,
}

#[cfg(feature = "queue")]
impl MailQueue {
    /// Create a new, self-contained mail queue.
    ///
    /// No global state is touched. The injected `queue` is used exclusively
    /// by this instance's [`push`](Self::push) and [`process`](Self::process)
    /// methods — safe to call from multiple concurrent instances without races.
    pub fn new(queue: Arc<dyn Queue>, mailer: Arc<dyn Mailer>) -> Self {
        Self { queue, mailer }
    }

    /// Push a mail onto **this instance's queue** for background delivery.
    ///
    /// Enqueues a [`SendMailJob`] directly onto `self.queue` via
    /// [`rf_queue::dispatch`] — no process-global state is read or written.
    /// This means push and [`process`](Self::process) always target the
    /// **same** backend (the queue passed to [`new`](Self::new)), making the
    /// pair race-free under concurrent instantiation.
    ///
    /// For convenience dispatch onto the process-global default queue (e.g.
    /// from [`Mailable::queue`](crate::Mailable::queue)), see
    /// [`push_global`](Self::push_global).
    ///
    /// # Async signature, synchronous bridge
    ///
    /// This method is declared `async` for consistency with the surrounding
    /// async interface, but the actual enqueue step is **synchronous** inside:
    /// [`rf_queue::dispatch`] drives the queue's `push` future on a dedicated
    /// [`AsyncBridge`](rf_async_bridge::AsyncBridge) thread rather than the
    /// Tokio thread pool. This design means:
    ///
    /// - **Safe from inside any Tokio runtime** — calling `push().await` from
    ///   an Axum handler, a spawned task, or `#[tokio::main]` will never panic
    ///   with "cannot start a runtime from within a runtime".
    /// - The bridge incurs a brief cross-thread round-trip. For very
    ///   high-throughput enqueue paths, wrapping in
    ///   `tokio::task::spawn_blocking` avoids holding a Tokio worker while the
    ///   bridge completes, though in practice the overhead is negligible for
    ///   single-message dispatches.
    pub async fn push(&self, mail: Mail) -> Result<(), MailError> {
        dispatch(Arc::clone(&self.queue), SendMailJob { mail })
            .map_err(|e| MailError::SendFailed(format!("failed to enqueue mail job: {e}")))?;
        Ok(())
    }

    /// Dispatch a mail onto the **process-global** default queue.
    ///
    /// This is the shared, handle-free path used by
    /// [`Mailable::queue`](crate::Mailable::queue) and
    /// [`MailableAsync::queue`](crate::MailableAsync::queue), which have no
    /// `MailQueue` instance to call the instance method on. The caller is
    /// responsible for configuring the global queue via
    /// [`rf_queue::set_default_queue`] / [`rf_queue::Jobs::set_queue`] before
    /// using this method, otherwise messages land on the default in-memory queue.
    ///
    /// When you *do* have a `MailQueue` instance, prefer the instance
    /// [`push`](Self::push) method — it is self-contained and race-free.
    pub async fn push_global(mail: Mail) -> Result<(), MailError> {
        Jobs::dispatch(SendMailJob { mail })
            .map_err(|e| MailError::SendFailed(format!("failed to enqueue mail job: {e}")))?;
        Ok(())
    }

    /// Process queued emails by draining this queue with a worker.
    ///
    /// Reserves and runs every currently-pending [`SendMailJob`] on this queue's
    /// backend, delivering each message through **this queue's configured
    /// `mailer`** (the [`Mailer`] passed to [`MailQueue::new`]) — the injected
    /// transport is used for real, not just held. Register a real
    /// [`FileMailer`](crate::FileMailer)/SMTP mailer and every drained message is
    /// delivered through it.
    ///
    /// Returns `Ok(())` when the drain loop completes, regardless of how many
    /// individual jobs were delivered vs. dead-lettered. Use
    /// [`process_report`](Self::process_report) when you need to distinguish
    /// successful deliveries from permanently-failed (dead-lettered) ones.
    pub async fn process(&self) -> Result<(), MailError> {
        self.process_report().await?;
        Ok(())
    }

    /// Process queued emails and return a [`DeliveryReport`] with outcome counts.
    ///
    /// Drains every currently-pending [`SendMailJob`] exactly like
    /// [`process`](Self::process) does, but returns a [`DeliveryReport`]
    /// distinguishing jobs that **delivered** successfully from those that
    /// **dead-lettered** after exhausting retries.
    ///
    /// # Counting method
    ///
    /// The counts are derived from the queue backend:
    ///
    /// - `total_pending` = `queue.size("mail")` sampled **before** the drain.
    /// - `dead_lettered` = growth in `queue.failed()` across the drain.
    /// - `delivered` = `total_pending - dead_lettered`.
    ///
    /// Jobs that are still being retried at the time `process_report` is called
    /// are processed to completion (delivered or dead-lettered) before the
    /// drain loop exits, so every initially-pending job is accounted for in
    /// exactly one of the two counters.
    ///
    /// # Backend note
    ///
    /// Backends that do not override [`Queue::failed`](rf_queue::Queue::failed)
    /// always return an empty list; on those backends `dead_lettered` will be
    /// reported as `0` even when jobs exhaust retries. The in-memory
    /// [`rf_queue::MemoryQueue`] fully tracks the dead-letter list and produces
    /// accurate counts.
    pub async fn process_report(&self) -> Result<DeliveryReport, MailError> {
        // Snapshot the pending count and the failed-jobs baseline BEFORE the drain
        // so we can compute deltas after the loop exits.
        let total_pending = self
            .queue
            .size("mail")
            .await
            .map_err(|e| MailError::SendFailed(format!("queue.size failed: {e}")))?;

        let before_failed = self
            .queue
            .failed()
            .await
            .map_err(|e| MailError::SendFailed(format!("queue.failed failed: {e}")))?
            .len();

        let mailer = Arc::clone(&self.mailer);
        let worker = Worker::new(Arc::clone(&self.queue))
            .queues(vec!["mail".to_string()])
            .handle::<SendMailJob>(move |job| {
                let mailer = Arc::clone(&mailer);
                Box::pin(async move {
                    mailer
                        .send(job.mail)
                        .await
                        .map_err(|e| QueueError::JobFailed(e.to_string()))
                })
            });
        while worker
            .work_once()
            .await
            .map_err(|e| MailError::SendFailed(e.to_string()))?
        {}

        // Count newly dead-lettered jobs (those that were added to failed() by
        // this drain run).
        let after_failed = self
            .queue
            .failed()
            .await
            .map_err(|e| MailError::SendFailed(format!("queue.failed failed: {e}")))?
            .len();

        let dead_lettered = after_failed.saturating_sub(before_failed);
        let delivered = total_pending.saturating_sub(dead_lettered);

        Ok(DeliveryReport { delivered, dead_lettered })
    }
}

#[cfg(feature = "queue")]
/// Job for sending email in the background
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMailJob {
    /// The mail to send
    pub mail: Mail,
}

#[cfg(feature = "queue")]
#[async_trait]
impl Job for SendMailJob {
    /// Deliver this job through the **process-global** mail transport chain.
    ///
    /// This method is invoked when a [`Worker`] is set up with
    /// [`Worker::register::<SendMailJob>()`] — the type-erased global dispatch
    /// path. It routes through [`crate::facade::deliver_mail`]:
    /// mail-fake recorder (if active) ➜ configured SMTP ➜ the default
    /// `.eml`-on-disk [`FileMailer`](crate::FileMailer).
    ///
    /// **Instance path** (`MailQueue::process`): when you use
    /// [`MailQueue::process`], the worker is given a closure handler that
    /// delivers through the *injected* [`Mailer`](crate::Mailer) instead of
    /// calling this `handle` method — so the two paths are consistent as long
    /// as the injected mailer and the global facade are configured to the same
    /// transport.
    async fn handle(&self) -> Result<(), QueueError> {
        tracing::info!(subject = %self.mail.subject, "Processing SendMailJob via global facade");
        crate::facade::deliver_mail(self.mail.clone())
            .map_err(|e| QueueError::JobFailed(e.to_string()))
    }

    fn job_type(&self) -> &'static str {
        "send_mail"
    }

    fn queue(&self) -> &str {
        "mail"
    }

    fn max_retries(&self) -> u32 {
        3
    }
}

// Non-feature version - provides stubs
#[cfg(not(feature = "queue"))]
pub struct MailQueue;

#[cfg(not(feature = "queue"))]
impl MailQueue {
    /// Stub: instance push (feature disabled).
    pub async fn push(&self, _mail: crate::Mail) -> Result<(), crate::MailError> {
        Err(crate::MailError::ConfigError(
            "Queue feature not enabled. Enable the 'queue' feature to use mail queuing.".into(),
        ))
    }

    /// Stub: global push (feature disabled).
    pub async fn push_global(_mail: crate::Mail) -> Result<(), crate::MailError> {
        Err(crate::MailError::ConfigError(
            "Queue feature not enabled. Enable the 'queue' feature to use mail queuing.".into(),
        ))
    }
}

#[cfg(test)]
#[cfg(feature = "queue")]
mod tests {
    use super::*;
    use crate::{Address, Mail, MailBody};

    fn sample_mail() -> Mail {
        Mail {
            // Unique id per call so concurrent deliveries never produce the
            // same filename (FileMailer uses {timestamp}-{id}.eml).
            id: uuid::Uuid::new_v4().to_string(),
            to: vec![Address::new("test@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Test".into(),
            body: MailBody::Text("Hello".into()),
            attachments: vec![],
        }
    }

    #[test]
    fn test_send_mail_job() {
        let job = SendMailJob { mail: sample_mail() };

        assert_eq!(job.queue(), "mail");
        assert_eq!(job.job_type(), "send_mail");
        assert_eq!(job.max_retries(), 3);
    }

    #[test]
    fn test_serialize_job() {
        let job = SendMailJob { mail: sample_mail() };

        // Should be serializable
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("test@example.com"));

        // Should be deserializable
        let deserialized: SendMailJob = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.mail.subject, "Test");
    }

    #[tokio::test]
    async fn test_process_delivers_via_injected_mailer_to_eml() {
        use crate::FileMailer;
        use rf_queue::{dispatch, MemoryQueue, Queue};

        // Fresh, local queue + a real FileMailer pointed at a unique temp dir —
        // no process-global state, so this never races other tests.
        let dir = std::env::temp_dir().join(format!("rf-mail-queue-eml-{}", uuid::Uuid::new_v4()));
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        dispatch(Arc::clone(&queue), SendMailJob { mail: sample_mail() })
            .expect("enqueue SendMailJob");

        let mailer: Arc<dyn Mailer> = Arc::new(FileMailer::new(&dir));
        MailQueue::new(Arc::clone(&queue), mailer)
            .process()
            .await
            .expect("process drains the mail queue");

        // The queued mail must land as a REAL .eml via the INJECTED mailer, not
        // vanish into the in-memory MemoryMailer.
        let emls: Vec<_> = std::fs::read_dir(&dir)
            .expect("mailbox dir exists")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("eml"))
            .collect();
        assert_eq!(
            emls.len(),
            1,
            "queued mail must produce exactly one .eml via the injected mailer"
        );
        let body = std::fs::read_to_string(emls[0].path()).unwrap();
        assert!(body.contains("Test"), "eml should carry the subject: {body}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_push_actually_enqueues() {
        use rf_queue::{MemoryQueue, Queue};

        // Use instance push: directly onto a fresh in-memory queue, no globals.
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let mailer: Arc<dyn Mailer> = Arc::new(crate::MemoryMailer::new());
        let mail_queue = MailQueue::new(Arc::clone(&queue), mailer);

        let before = queue.size("mail").await.unwrap();

        // Instance push: enqueues onto self.queue, not the global default.
        mail_queue.push(sample_mail()).await.unwrap();

        let after = queue.size("mail").await.unwrap();
        assert_eq!(
            after,
            before + 1,
            "MailQueue::push must enqueue a SendMailJob onto self.queue (not the global)"
        );

        // And a worker can reserve + run it (proving the payload round-trips).
        let worker = Worker::new(Arc::clone(&queue))
            .queues(vec!["mail".to_string()])
            .register::<SendMailJob>();
        assert!(worker.work_once().await.unwrap(), "worker reserved the job");
        assert_eq!(
            queue.size("mail").await.unwrap(),
            before,
            "queue drained after the worker ran the job"
        );
    }

    /// Proves the core self-consistency invariant: MailQueue::push (instance) and
    /// MailQueue::process both operate on self.queue — no global set_default_queue
    /// wiring is needed or used. Before the fix, push dispatched to the
    /// process-global DEFAULT_QUEUE while process drained self.queue, causing
    /// silent loss when they diverged under concurrent instantiation.
    #[tokio::test]
    async fn test_push_process_self_consistent_no_external_wiring() {
        use crate::FileMailer;
        use rf_queue::{MemoryQueue, Queue};

        let dir = std::env::temp_dir()
            .join(format!("rf-mail-no-wiring-{}", uuid::Uuid::new_v4()));

        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let mailer: Arc<dyn Mailer> = Arc::new(FileMailer::new(&dir));

        // Create MailQueue — new() does NOT touch the global default queue.
        // No set_default_queue call is needed or made anywhere here.
        let mail_queue = MailQueue::new(Arc::clone(&queue), mailer);

        let before = queue.size("mail").await.unwrap();

        // Instance push: lands on self.queue (not the global), safe with no
        // external wiring.
        mail_queue.push(sample_mail()).await.unwrap();

        let after = queue.size("mail").await.unwrap();
        assert_eq!(
            after,
            before + 1,
            "instance push must land on self.queue without any global set_default_queue \
             wiring (0 jobs after push = still broken)"
        );

        // Process drains self.queue (the same queue push landed on) and delivers
        // via the injected mailer → must produce a real .eml on disk.
        mail_queue.process().await.expect("process must drain the queue");

        let emls: Vec<_> = std::fs::read_dir(&dir)
            .expect("mailbox dir must exist after process()")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("eml"))
            .collect();
        assert_eq!(
            emls.len(),
            1,
            "process() must deliver exactly one .eml via the injected mailer \
             (0 = queued mail silently lost; >1 = cross-contamination)"
        );
        let body = std::fs::read_to_string(emls[0].path()).unwrap();
        assert!(body.contains("Test"), "eml must carry the mail subject: {body}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `process_report()` must return the correct `delivered` and `dead_lettered`
    /// counts for a mixed-outcome batch: some mails succeed, some exhaust retries
    /// and land in the dead-letter list.
    ///
    /// The key regression being guarded: `process()` previously returned `Ok(())`
    /// with no indication of partial failure; callers had to separately call
    /// `queue.failed()` to detect dead letters.  Now `process_report()` surfaces
    /// the outcome directly, and `process()` delegates to it so the drain logic
    /// lives in exactly one place.
    #[tokio::test]
    async fn test_process_report_mixed_delivery() {
        use crate::{Mail, MailBody, MailError, Mailer};
        use async_trait::async_trait;
        use rf_queue::{MemoryQueue, Queue};
        use std::sync::atomic::{AtomicUsize, Ordering};

        // A mailer that fails for any mail whose subject starts with "FAIL:".
        // Deterministic: the same subject always produces the same outcome, so
        // retries of a "FAIL:" job always fail until it is dead-lettered.
        struct SubjectFilterMailer {
            delivered: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl Mailer for SubjectFilterMailer {
            async fn send(&self, mail: Mail) -> Result<(), MailError> {
                if mail.subject.starts_with("FAIL:") {
                    Err(MailError::SendFailed(
                        "simulated delivery failure".into(),
                    ))
                } else {
                    self.delivered.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        }

        let delivered_counter = Arc::new(AtomicUsize::new(0));
        let mailer: Arc<dyn Mailer> = Arc::new(SubjectFilterMailer {
            delivered: Arc::clone(&delivered_counter),
        });

        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let mq = MailQueue::new(Arc::clone(&queue), mailer);

        // Push 3 mails that will deliver and 2 that will always fail (dead-letter).
        let mut good_subjects = vec!["Good #1", "Good #2", "Good #3"];
        let mut fail_subjects = vec!["FAIL: #1", "FAIL: #2"];
        for subject in good_subjects.drain(..) {
            mq.push(Mail {
                id: uuid::Uuid::new_v4().to_string(),
                to: vec![Address::new("to@example.com")],
                cc: vec![],
                bcc: vec![],
                from: Address::new("from@example.com"),
                reply_to: None,
                subject: subject.into(),
                body: MailBody::Text("body".into()),
                attachments: vec![],
            })
            .await
            .unwrap();
        }
        for subject in fail_subjects.drain(..) {
            mq.push(Mail {
                id: uuid::Uuid::new_v4().to_string(),
                to: vec![Address::new("to@example.com")],
                cc: vec![],
                bcc: vec![],
                from: Address::new("from@example.com"),
                reply_to: None,
                subject: subject.into(),
                body: MailBody::Text("body".into()),
                attachments: vec![],
            })
            .await
            .unwrap();
        }

        // Drain and observe the outcome report.
        let report = mq.process_report().await.expect("process_report must not error");

        assert_eq!(
            report.delivered, 3,
            "3 good mails must be reported as delivered (got {})",
            report.delivered
        );
        assert_eq!(
            report.dead_lettered, 2,
            "2 always-failing mails must be reported as dead-lettered (got {})",
            report.dead_lettered
        );

        // Cross-check: the mailer's own counter must agree with the report.
        assert_eq!(
            delivered_counter.load(Ordering::Relaxed),
            3,
            "mailer's internal counter must match report.delivered"
        );

        // Cross-check: queue.failed() must list the same 2 dead-lettered jobs.
        let failed = queue.failed().await.unwrap();
        assert_eq!(
            failed.len(),
            2,
            "queue.failed() must hold the 2 dead-lettered jobs; report.dead_lettered \
             must match (report={}, queue.failed={})",
            report.dead_lettered,
            failed.len()
        );

        // The queue is fully drained.
        assert_eq!(
            queue.size("mail").await.unwrap(),
            0,
            "queue must be empty after process_report()"
        );
    }

    /// Regression: multiple MailQueue instances operating CONCURRENTLY must NEVER
    /// contaminate each other's queues. Before the fix, MailQueue::new called
    /// set_default_queue and push dispatched to the process-global — so the last
    /// new() winner owned everyone's pushes. This test fails under default cargo
    /// test parallelism without the fix, and passes after.
    #[tokio::test]
    async fn test_parallel_push_no_race() {
        use crate::FileMailer;
        use rf_queue::{MemoryQueue, Queue};
        use std::sync::Arc;

        const INSTANCES: usize = 4;
        const MAILS_PER_INSTANCE: usize = 3;

        // Spawn N concurrent tasks each owning a fully independent MailQueue
        // (separate queue + separate mailbox dir). They all run in parallel
        // inside one Tokio runtime — same parallelism as default cargo test.
        let handles: Vec<_> = (0..INSTANCES)
            .map(|i| {
                tokio::spawn(async move {
                    let dir = std::env::temp_dir().join(format!(
                        "rf-mail-parallel-{i}-{}",
                        uuid::Uuid::new_v4()
                    ));
                    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
                    let mailer: Arc<dyn Mailer> = Arc::new(FileMailer::new(&dir));
                    let mq = MailQueue::new(Arc::clone(&queue), Arc::clone(&mailer));

                    // Push MAILS_PER_INSTANCE mails onto THIS instance's queue.
                    for _ in 0..MAILS_PER_INSTANCE {
                        mq.push(sample_mail()).await.expect("instance push must not fail");
                    }

                    // Verify the queue holds exactly the right count before draining.
                    let queued = queue.size("mail").await.unwrap();
                    assert_eq!(
                        queued, MAILS_PER_INSTANCE,
                        "instance {i}: queue must hold exactly {MAILS_PER_INSTANCE} jobs \
                         after push (got {queued}) — cross-instance contamination or loss"
                    );

                    // Process drains self.queue through the injected mailer.
                    mq.process().await.expect("process must drain without error");

                    // Count .eml files produced by the injected FileMailer.
                    let count = std::fs::read_dir(&dir)
                        .map(|rd| {
                            rd.filter_map(|e| e.ok())
                                .filter(|e| {
                                    e.path().extension().and_then(|x| x.to_str()) == Some("eml")
                                })
                                .count()
                        })
                        .unwrap_or(0);

                    let _ = std::fs::remove_dir_all(&dir);
                    (i, count)
                })
            })
            .collect();

        for handle in handles {
            let (i, count) = handle.await.expect("parallel task must not panic");
            assert_eq!(
                count, MAILS_PER_INSTANCE,
                "instance {i}: expected {MAILS_PER_INSTANCE} .eml files, got {count} — \
                 push/process divergence or cross-instance contamination detected"
            );
        }
    }
}
