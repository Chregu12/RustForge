//! Mock mailer backend for testing

use crate::{MailError, Mailer, Message};
use async_trait::async_trait;

/// Mock mailer for testing
///
/// Can be configured to succeed or fail for testing error handling.
///
/// # Example
///
/// ```
/// use rf_mail::{MockMailer, Mailer, MessageBuilder, Address};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Success case
/// let mailer = MockMailer::new();
/// let message = MessageBuilder::new()
///     .from(Address::new("sender@example.com"))
///     .to(Address::new("recipient@example.com"))
///     .subject("Test")
///     .text("Hello")
///     .build()?;
///
/// assert!(mailer.send(&message).await.is_ok());
///
/// // Failure case
/// let failing_mailer = MockMailer::with_failure();
/// assert!(failing_mailer.send(&message).await.is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct MockMailer {
    should_fail: bool,
}

impl MockMailer {
    /// Create new mock mailer that succeeds
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    /// Create mock mailer that always fails
    pub fn with_failure() -> Self {
        Self { should_fail: true }
    }
}

impl Default for MockMailer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Mailer for MockMailer {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        if self.should_fail {
            return Err(MailError::SendFailed("Mock failure".into()));
        }

        tracing::debug!(
            to = ?message.to,
            subject = %message.subject,
            "Mock email sent"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MessageBuilder};

    #[tokio::test]
    async fn test_mock_mailer_success() {
        let mailer = MockMailer::new();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert!(mailer.send(&message).await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_mailer_failure() {
        let mailer = MockMailer::with_failure();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert!(mailer.send(&message).await.is_err());
    }
}
