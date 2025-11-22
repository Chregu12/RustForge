//! Mailable trait for reusable email types

use crate::{Mail, MailBuilder, MailError, Mailer};
use async_trait::async_trait;

/// Trait for types that can be sent as email
///
/// This trait provides a clean way to encapsulate email logic into reusable types.
///
/// # Example
///
/// ```
/// use rf_mail::{Mailable, MailBuilder, Address, MailError};
///
/// struct WelcomeEmail {
///     to: String,
///     name: String,
/// }
///
/// impl Mailable for WelcomeEmail {
///     fn build(&self) -> MailBuilder {
///         MailBuilder::new()
///             .to(Address::new(&self.to))
///             .from(Address::new("noreply@example.com"))
///             .subject("Welcome!")
///             .text(format!("Welcome, {}!", self.name))
///     }
/// }
/// ```
pub trait Mailable: Send + Sync {
    /// Build the mail using the MailBuilder
    ///
    /// Returns a configured MailBuilder. Call `.build()` on it to get a Mail.
    fn build(&self) -> MailBuilder;

    /// Send the email using the provided mailer
    ///
    /// # Errors
    ///
    /// Returns an error if building or sending fails.
    fn send(
        &self,
        mailer: &dyn Mailer,
    ) -> impl std::future::Future<Output = Result<(), MailError>> + Send {
        async move {
            let mail = self.build().build()?;

            // Convert Mail to Message
            let message = match mail.body {
                crate::MailBody::Html(html) => crate::MessageBuilder::new()
                    .from(mail.from.clone())
                    .to_many(mail.to.clone())
                    .subject(mail.subject.clone())
                    .html(html)
                    .build()?,
                crate::MailBody::Text(text) => crate::MessageBuilder::new()
                    .from(mail.from.clone())
                    .to_many(mail.to.clone())
                    .subject(mail.subject.clone())
                    .text(text)
                    .build()?,
                crate::MailBody::Both { html, text } => crate::MessageBuilder::new()
                    .from(mail.from.clone())
                    .to_many(mail.to.clone())
                    .subject(mail.subject.clone())
                    .html(html)
                    .text(text)
                    .build()?,
            };

            mailer.send(message.into()).await
        }
    }

    /// Queue the email for background sending
    ///
    /// This requires the `queue` feature to be enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if queuing fails.
    #[cfg(feature = "queue")]
    fn queue(&self) -> impl std::future::Future<Output = Result<(), MailError>> + Send {
        async move {
            let mail = self.build().build()?;
            crate::queue::MailQueue::push(mail).await?;
            Ok(())
        }
    }

    /// Queue name for this mailable (for queue routing)
    fn queue_name(&self) -> Option<&str> {
        None
    }
}

/// Simplified Mailable for immediate sending without async build
///
/// This is the main trait users will implement. The async send is handled internally.
#[async_trait]
pub trait MailableAsync: Send + Sync {
    /// Build the mail asynchronously
    async fn build(&self) -> Result<Mail, MailError>;

    /// Send the email using the provided mailer
    async fn send(&self, mailer: &dyn Mailer) -> Result<(), MailError> {
        let mail = self.build().await?;

        // Convert Mail to Message
        let message = match mail.body {
            crate::MailBody::Html(html) => crate::MessageBuilder::new()
                .from(mail.from.clone())
                .to_many(mail.to.clone())
                .subject(mail.subject.clone())
                .html(html)
                .build()?,
            crate::MailBody::Text(text) => crate::MessageBuilder::new()
                .from(mail.from.clone())
                .to_many(mail.to.clone())
                .subject(mail.subject.clone())
                .text(text)
                .build()?,
            crate::MailBody::Both { html, text } => crate::MessageBuilder::new()
                .from(mail.from.clone())
                .to_many(mail.to.clone())
                .subject(mail.subject.clone())
                .html(html)
                .text(text)
                .build()?,
        };

        mailer.send(message.into()).await
    }

    /// Queue the email for background sending
    #[cfg(feature = "queue")]
    async fn queue(&self) -> Result<(), MailError> {
        let mail = self.build().await?;
        crate::queue::MailQueue::push(mail).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MemoryMailer};

    struct TestMail {
        to: String,
    }

    impl Mailable for TestMail {
        fn build(&self) -> MailBuilder {
            MailBuilder::new()
                .from(Address::new("test@example.com"))
                .to(Address::new(&self.to))
                .subject("Test")
                .text("Hello")
        }
    }

    #[tokio::test]
    async fn test_mailable_send() {
        let mailer = MemoryMailer::new();
        let mailable = TestMail {
            to: "user@example.com".into(),
        };

        mailable.send(&mailer).await.unwrap();

        let sent = mailer.sent_messages();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to[0].email, "user@example.com");
    }
}
