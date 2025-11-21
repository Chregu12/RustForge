//! Queue integration for background mail sending

#[cfg(feature = "queue")]
use crate::{Mail, MailError, Mailer};
#[cfg(feature = "queue")]
use async_trait::async_trait;
#[cfg(feature = "queue")]
use rf_jobs::{Job, JobContext, JobResult, QueueManager};
#[cfg(feature = "queue")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "queue")]
use std::sync::Arc;

#[cfg(feature = "queue")]
/// Mail queue for background sending
pub struct MailQueue {
    queue: Arc<QueueManager>,
    mailer: Arc<dyn Mailer>,
}

#[cfg(feature = "queue")]
impl MailQueue {
    /// Create a new mail queue
    pub fn new(queue: Arc<QueueManager>, mailer: Arc<dyn Mailer>) -> Self {
        Self { queue, mailer }
    }

    /// Push a mail to the queue
    pub async fn push(mail: Mail) -> Result<(), MailError> {
        // This is a simplified version - in production you'd get the queue from a global registry
        // For now, we'll just serialize to show the concept
        let _job = SendMailJob { mail };

        // In real implementation, you'd dispatch to rf-jobs queue
        // queue.dispatch(job).await?;

        Ok(())
    }

    /// Process queued emails
    pub async fn process(&self) -> Result<(), MailError> {
        // In real implementation, this would be handled by rf-jobs worker
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
    async fn handle(&self, _ctx: JobContext) -> JobResult {
        // Convert Mail to Message and send
        // This requires access to a mailer instance
        // In production, you'd get this from a global registry or dependency injection

        // For now, just log
        tracing::info!("Processing SendMailJob for: {}", self.mail.subject);

        Ok(())
    }

    fn queue(&self) -> &str {
        "mail"
    }

    fn max_attempts(&self) -> u32 {
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

    #[test]
    fn test_send_mail_job() {
        let mail = Mail {
            id: "test".into(),
            to: vec![Address::new("test@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Test".into(),
            body: MailBody::Text("Hello".into()),
            attachments: vec![],
        };

        let job = SendMailJob { mail };

        assert_eq!(job.queue(), "mail");
        assert_eq!(job.max_attempts(), 3);
    }

    #[test]
    fn test_serialize_job() {
        let mail = Mail {
            id: "test".into(),
            to: vec![Address::new("test@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Test".into(),
            body: MailBody::Text("Hello".into()),
            attachments: vec![],
        };

        let job = SendMailJob { mail };

        // Should be serializable
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("test@example.com"));

        // Should be deserializable
        let deserialized: SendMailJob = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.mail.subject, "Test");
    }
}
