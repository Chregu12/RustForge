//! Queue integration for background mail sending

#[cfg(feature = "queue")]
use crate::{Mail, MailError, Mailer};
#[cfg(feature = "queue")]
use async_trait::async_trait;
#[cfg(feature = "queue")]
use rf_queue::{set_default_queue, Job, Jobs, Queue, QueueError, Worker};
#[cfg(feature = "queue")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "queue")]
use std::sync::Arc;

#[cfg(feature = "queue")]
/// Mail queue for background sending
pub struct MailQueue {
    queue: Arc<dyn Queue>,
    mailer: Arc<dyn Mailer>,
}

#[cfg(feature = "queue")]
impl MailQueue {
    /// Create a new mail queue.
    ///
    /// **Self-consistency guarantee**: this constructor immediately installs
    /// `queue` as the process-global default queue (via
    /// [`rf_queue::set_default_queue`]). Because the static
    /// [`MailQueue::push`] dispatches to that same global, callers do NOT
    /// need a separate `rf_queue::set_default_queue` wiring step — push and
    /// process share one queue out of the box.
    pub fn new(queue: Arc<dyn Queue>, mailer: Arc<dyn Mailer>) -> Self {
        // Sync the process-global default queue with this instance's queue so
        // that the static MailQueue::push (Jobs::dispatch -> global) and
        // MailQueue::process (drains self.queue) always target the SAME
        // backend, eliminating silent-loss when the caller never calls
        // set_default_queue explicitly.
        set_default_queue(Arc::clone(&queue));
        Self { queue, mailer }
    }

    /// Push a mail to the queue for background delivery.
    ///
    /// Wraps the message in a [`SendMailJob`] and dispatches it onto the
    /// process-global default queue through the `rf_queue` [`Jobs`] facade.
    ///
    /// **No external wiring required**: [`MailQueue::new`] installs the
    /// injected queue as the process-global default, so calls to `push` and
    /// calls to [`MailQueue::process`] always drain the **same** queue — no
    /// separate [`rf_queue::set_default_queue`] call is needed at the call
    /// site. If you do call `set_default_queue` after `new`, ensure it points
    /// at the same queue instance to avoid silent divergence.
    ///
    /// The job is driven over the shared deadlock-safe async bridge, so this
    /// is safe to call from inside or outside an ambient Tokio runtime.
    pub async fn push(mail: Mail) -> Result<(), MailError> {
        let job = SendMailJob { mail };
        Jobs::dispatch(job)
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
    async fn handle(&self) -> Result<(), QueueError> {
        tracing::info!(subject = %self.mail.subject, "Processing SendMailJob");

        // Deliver through the SAME real transport chain as a synchronous
        // `Mail::send`: mail-fake recorder ➜ configured SMTP ➜ the default
        // `.eml`-on-disk `FileMailer`. This is what makes a queued email actually
        // reach a real transport (a real `.eml` on disk / real SMTP) instead of
        // vanishing into the in-memory `MemoryMailer`.
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
    pub async fn push(_mail: crate::Mail) -> Result<(), crate::MailError> {
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
            id: "test".into(),
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

        // Point the global default queue at a fresh in-memory backend.
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        rf_queue::set_default_queue(Arc::clone(&queue));

        let before = queue.size("mail").await.unwrap();

        // Real dispatch: this must land a job on the queue, not silently drop it.
        MailQueue::push(sample_mail()).await.unwrap();

        let after = queue.size("mail").await.unwrap();
        assert_eq!(
            after,
            before + 1,
            "MailQueue::push must enqueue a SendMailJob onto the real queue"
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

    /// Proves the core invariant fixed in this pass: MailQueue::push and
    /// MailQueue::process are SELF-CONSISTENT with NO external set_default_queue
    /// wiring. Before the fix, push dispatched to the process-global DEFAULT_QUEUE
    /// while process drained self.queue — silent loss when they diverged.
    ///
    /// After the fix, MailQueue::new installs self.queue as the process-global,
    /// so push (which targets the global) and process (which drains self.queue)
    /// share ONE queue. No separate set_default_queue call is needed.
    #[tokio::test]
    async fn test_push_process_self_consistent_no_external_wiring() {
        use crate::FileMailer;
        use rf_queue::{MemoryQueue, Queue};

        let dir = std::env::temp_dir()
            .join(format!("rf-mail-no-wiring-{}", uuid::Uuid::new_v4()));

        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let mailer: Arc<dyn Mailer> = Arc::new(FileMailer::new(&dir));

        // Create MailQueue — new() syncs the global default queue to `queue`.
        // Critically: we do NOT call rf_queue::set_default_queue here; it must
        // not be required.
        let mail_queue = MailQueue::new(Arc::clone(&queue), mailer);

        let before = queue.size("mail").await.unwrap();

        // Push via the static API (dispatches to the global = self.queue).
        MailQueue::push(sample_mail()).await.unwrap();

        let after = queue.size("mail").await.unwrap();
        assert_eq!(
            after,
            before + 1,
            "push must land on self.queue without external set_default_queue wiring \
             (left=after right=before+1; was 0=1 before fix — silent loss)"
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
             (0 means the queued mail was silently lost)"
        );
        let body = std::fs::read_to_string(emls[0].path()).unwrap();
        assert!(body.contains("Test"), "eml must carry the mail subject: {body}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
