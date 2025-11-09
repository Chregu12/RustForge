//! Memory mailer backend for testing

use crate::{MailError, Mailer, Message};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// In-memory mailer for testing
///
/// Stores all sent messages in memory for inspection.
///
/// # Example
///
/// ```
/// use rf_mail::{MemoryMailer, Mailer, MessageBuilder, Address};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mailer = MemoryMailer::new();
///
/// let message = MessageBuilder::new()
///     .from(Address::new("sender@example.com"))
///     .to(Address::new("recipient@example.com"))
///     .subject("Test")
///     .text("Hello")
///     .build()?;
///
/// mailer.send(&message).await?;
///
/// assert!(mailer.was_sent_to("recipient@example.com"));
/// assert_eq!(mailer.sent_count(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MemoryMailer {
    sent: Arc<Mutex<Vec<Message>>>,
}

impl MemoryMailer {
    /// Create new memory mailer
    pub fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all sent messages
    pub fn sent_messages(&self) -> Vec<Message> {
        self.sent.lock().unwrap().clone()
    }

    /// Get number of sent messages
    pub fn sent_count(&self) -> usize {
        self.sent.lock().unwrap().len()
    }

    /// Clear all sent messages
    pub fn clear(&self) {
        self.sent.lock().unwrap().clear();
    }

    /// Check if any message was sent to the given email
    pub fn was_sent_to(&self, email: &str) -> bool {
        self.sent.lock().unwrap().iter().any(|msg| {
            msg.to.iter().any(|addr| addr.email == email)
        })
    }

    /// Check if any message was sent with the given subject
    pub fn was_sent_with_subject(&self, subject: &str) -> bool {
        self.sent
            .lock()
            .unwrap()
            .iter()
            .any(|msg| msg.subject == subject)
    }

    /// Get last sent message
    pub fn last_message(&self) -> Option<Message> {
        self.sent.lock().unwrap().last().cloned()
    }
}

impl Default for MemoryMailer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Mailer for MemoryMailer {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        self.sent.lock().unwrap().push(message.clone());

        tracing::info!(
            to = ?message.to,
            subject = %message.subject,
            "Email stored in memory"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MessageBuilder};

    #[tokio::test]
    async fn test_memory_mailer() {
        let mailer = MemoryMailer::new();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Test Email")
            .text("Hello")
            .build()
            .unwrap();

        mailer.send(&message).await.unwrap();

        assert_eq!(mailer.sent_count(), 1);
        assert!(mailer.was_sent_to("user@example.com"));
        assert!(mailer.was_sent_with_subject("Test Email"));

        let last = mailer.last_message().unwrap();
        assert_eq!(last.subject, "Test Email");
    }

    #[tokio::test]
    async fn test_memory_mailer_clear() {
        let mailer = MemoryMailer::new();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        mailer.send(&message).await.unwrap();
        assert_eq!(mailer.sent_count(), 1);

        mailer.clear();
        assert_eq!(mailer.sent_count(), 0);
    }
}
