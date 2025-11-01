use crate::domain::Message;
use crate::mailable::{Mailable, MailableError};
use crate::transports::{MailTransport, TransportError, TransportResponse};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Mailer for sending emails
pub struct Mailer {
    transport: Arc<dyn MailTransport>,
}

impl Mailer {
    pub fn new(transport: Arc<dyn MailTransport>) -> Self {
        Self { transport }
    }

    pub async fn send(&self, message: &Message) -> Result<TransportResponse, MailerError> {
        debug!(message_id = %message.id, "Sending email");

        let response = self.transport.send(message).await?;

        info!(
            message_id = %message.id,
            accepted = response.accepted.len(),
            rejected = response.rejected.len(),
            "Email sent successfully"
        );

        Ok(response)
    }

    pub async fn send_mailable<M: Mailable>(&self, mailable: M) -> Result<TransportResponse, MailerError> {
        let message = mailable.build().await?;
        self.send(&message).await
    }

    pub async fn test_connection(&self) -> Result<(), MailerError> {
        self.transport.test_connection().await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MailerError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Mailable error: {0}")]
    Mailable(#[from] MailableError),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Queue-based mailer (sends via queue)
pub struct QueuedMailer {
    queue: Arc<dyn QueuePort>,
}

#[async_trait]
pub trait QueuePort: Send + Sync {
    async fn push(&self, job: QueuedMailJob) -> Result<(), String>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueuedMailJob {
    pub message: Message,
}

impl QueuedMailer {
    pub fn new(queue: Arc<dyn QueuePort>) -> Self {
        Self { queue }
    }

    pub async fn send(&self, message: Message) -> Result<(), MailerError> {
        let job = QueuedMailJob { message };
        self.queue
            .push(job)
            .await
            .map_err(|e| MailerError::Config(e))?;
        Ok(())
    }

    pub async fn send_mailable<M: Mailable>(&self, mailable: M) -> Result<(), MailerError> {
        let message = mailable.build().await?;
        self.send(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Address, Content};
    use crate::transports::TransportResult;

    struct MockTransport;

    #[async_trait]
    impl MailTransport for MockTransport {
        async fn send(&self, message: &Message) -> TransportResult {
            Ok(TransportResponse {
                message_id: message.id.clone(),
                accepted: vec!["recipient@example.com".to_string()],
                rejected: Vec::new(),
            })
        }

        async fn test_connection(&self) -> Result<(), TransportError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mailer_send() {
        let transport = Arc::new(MockTransport);
        let mailer = Mailer::new(transport);

        let message = Message::new(
            Address::new("sender@example.com"),
            Address::new("recipient@example.com"),
            "Test",
            Content::text("Hello"),
        );

        let result = mailer.send(&message).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.accepted.len(), 1);
    }
}
