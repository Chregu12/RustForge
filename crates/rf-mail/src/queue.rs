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
    pub async fn process(&self) -> Result<(), MailError> {
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
        Ok(())
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
