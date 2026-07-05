//! Email notification channel using rf-mail

use crate::channels::NotificationChannel;
use crate::{Notifiable, Notification, NotificationError, NotificationResult};
use async_trait::async_trait;
use rf_mail::{Address, MailBuilder, Mailer};
use std::sync::Arc;

/// Mail channel that integrates with rf-mail
pub struct MailChannel {
    mailer: Arc<dyn Mailer>,
    default_from: String,
}

impl MailChannel {
    /// Create a new mail channel
    pub fn new(mailer: Arc<dyn Mailer>, default_from: impl Into<String>) -> Self {
        Self {
            mailer,
            default_from: default_from.into(),
        }
    }
}

#[async_trait]
impl NotificationChannel for MailChannel {
    async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        // Get mail message from notification
        let mail_message = notification.to_mail().await.ok_or_else(|| {
            NotificationError::ChannelError("No mail message provided".to_string())
        })?;

        // Get recipient email
        let to_email = if !mail_message.to.is_empty() {
            mail_message.to[0].clone()
        } else {
            notifiable.route_notification_for_mail().ok_or_else(|| {
                NotificationError::RoutingError("No email address found".to_string())
            })?
        };

        // Build rf-mail message. `Mailer::send` consumes the `Mail` by value.
        let message = MailBuilder::new()
            .from(Address::new(
                mail_message.from.as_ref().unwrap_or(&self.default_from),
            ))
            .to(Address::new(&to_email))
            .subject(&mail_message.subject)
            .html(mail_message.to_html())
            .text(mail_message.to_text())
            .build()?;

        // Send via mailer (delivers for real: FileMailer writes an .eml, etc.)
        self.mailer.send(message).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::MailMessage;
    use rf_mail::MemoryMailer;

    struct TestUser {
        email: String,
    }

    impl Notifiable for TestUser {
        fn route_notification_for_mail(&self) -> Option<String> {
            Some(self.email.clone())
        }
    }

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<crate::Channel> {
            vec![crate::Channel::Mail]
        }

        async fn to_mail(&self) -> Option<MailMessage> {
            Some(
                MailMessage::new()
                    .subject("Test Subject")
                    .greeting("Hello!")
                    .line("This is a test.")
                    .action("Click Here", "https://example.com"),
            )
        }
    }

    #[tokio::test]
    async fn test_mail_channel_send() {
        let mailer = Arc::new(MemoryMailer::new());
        let channel = MailChannel::new(mailer.clone(), "noreply@example.com");

        let user = TestUser {
            email: "user@example.com".to_string(),
        };

        let notification = TestNotification;
        channel.send(&notification, &user).await.unwrap();

        // Verify mail was sent
        let sent = mailer.sent_messages();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject, "Test Subject");
    }
}
