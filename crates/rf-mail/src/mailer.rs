//! Mailer trait definitions

use crate::{Mail, MailError};
use async_trait::async_trait;

/// Mailer backend trait
///
/// Implement this trait to create custom email backends.
#[async_trait]
pub trait Mailer: Send + Sync {
    /// Send an email message
    ///
    /// # Errors
    ///
    /// Returns an error if the message fails to send.
    async fn send(&self, mail: Mail) -> Result<(), MailError>;

    /// Send multiple messages
    ///
    /// Default implementation sends messages sequentially.
    async fn send_batch(&self, messages: Vec<Mail>) -> Result<(), MailError> {
        for message in messages {
            self.send(message).await?;
        }
        Ok(())
    }
}

// Note: The async Mailable trait has been moved to mailable.rs as MailableAsync.
// Use rf_mail::MailableAsync for async mail building, or rf_mail::Mailable for sync.
