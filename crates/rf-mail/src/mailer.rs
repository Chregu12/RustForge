//! Mailer trait and Mailable trait definitions

use crate::{MailError, Message};
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
    async fn send(&self, message: &Message) -> Result<(), MailError>;

    /// Send multiple messages
    ///
    /// Default implementation sends messages sequentially.
    async fn send_batch(&self, messages: &[Message]) -> Result<(), MailError> {
        for message in messages {
            self.send(message).await?;
        }
        Ok(())
    }
}

/// Mailable trait for types that can be sent as email
///
/// This trait allows you to create reusable email types.
///
/// # Example
///
/// ```
/// use rf_mail::{Mailable, Message, MessageBuilder, Address, MailError};
/// use async_trait::async_trait;
///
/// struct WelcomeEmail {
///     to: Address,
///     name: String,
/// }
///
/// #[async_trait]
/// impl Mailable for WelcomeEmail {
///     async fn build(&self) -> Result<Message, MailError> {
///         Ok(MessageBuilder::new()
///             .from(Address::new("noreply@example.com"))
///             .to(self.to.clone())
///             .subject("Welcome!")
///             .text(format!("Welcome, {}!", self.name))
///             .build()?)
///     }
/// }
/// ```
#[async_trait]
pub trait Mailable: Send + Sync {
    /// Build the email message
    async fn build(&self) -> Result<Message, MailError>;

    /// Send the email using the provided mailer
    async fn send(&self, mailer: &dyn Mailer) -> Result<(), MailError> {
        let message = self.build().await?;
        mailer.send(&message).await
    }

    /// Queue name for background sending (optional)
    fn queue(&self) -> Option<&str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MessageBuilder};

    struct TestMailable {
        to: String,
    }

    #[async_trait]
    impl Mailable for TestMailable {
        async fn build(&self) -> Result<Message, MailError> {
            Ok(MessageBuilder::new()
                .from(Address::new("test@example.com"))
                .to(Address::new(&self.to))
                .subject("Test")
                .text("Test")
                .build()?)
        }
    }

    #[tokio::test]
    async fn test_mailable_build() {
        let mailable = TestMailable {
            to: "user@example.com".into(),
        };

        let message = mailable.build().await.unwrap();
        assert_eq!(message.to[0].email, "user@example.com");
    }
}
