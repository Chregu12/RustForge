//! Central notification dispatcher

use crate::channels::NotificationChannel;
use crate::{Channel, Notifiable, Notification, NotificationError, NotificationResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Central notification dispatcher that manages all channels
pub struct Notifier {
    channels: HashMap<Channel, Arc<dyn NotificationChannel>>,
}

impl Notifier {
    /// Create a new notifier
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Register a channel handler
    pub fn register_channel(
        &mut self,
        channel: Channel,
        handler: Arc<dyn NotificationChannel>,
    ) -> &mut Self {
        self.channels.insert(channel, handler);
        self
    }

    /// Send notification to a notifiable entity
    pub async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        let channels = notification.via();

        // Send to all channels sequentially
        for channel in channels {
            if let Some(handler) = self.channels.get(&channel) {
                handler.send(notification, notifiable).await?;
            } else {
                return Err(NotificationError::RoutingError(format!(
                    "No handler registered for channel: {:?}",
                    channel
                )));
            }
        }

        Ok(())
    }

    /// Check if a channel is registered
    pub fn has_channel(&self, channel: &Channel) -> bool {
        self.channels.contains_key(channel)
    }

    /// Get registered channels
    pub fn registered_channels(&self) -> Vec<Channel> {
        self.channels.keys().cloned().collect()
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring a notifier
pub struct NotifierBuilder {
    notifier: Notifier,
}

impl NotifierBuilder {
    /// Create a new notifier builder
    pub fn new() -> Self {
        Self {
            notifier: Notifier::new(),
        }
    }

    /// Register a channel
    pub fn channel(mut self, channel: Channel, handler: Arc<dyn NotificationChannel>) -> Self {
        self.notifier.register_channel(channel, handler);
        self
    }

    #[cfg(feature = "mail")]
    /// Register a mail channel
    pub fn mail(
        mut self,
        mailer: Arc<dyn rf_mail::Mailer>,
        default_from: impl Into<String>,
    ) -> Self {
        use crate::channels::MailChannel;
        let channel = Arc::new(MailChannel::new(mailer, default_from));
        self.notifier.register_channel(Channel::Mail, channel);
        self
    }

    #[cfg(feature = "database")]
    /// Register a database channel
    pub fn database(mut self, db: sea_orm::DatabaseConnection) -> Self {
        use crate::channels::DatabaseChannel;
        let channel = Arc::new(DatabaseChannel::new(db));
        self.notifier.register_channel(Channel::Database, channel);
        self
    }

    #[cfg(feature = "sms")]
    /// Register an SMS channel
    pub fn sms(mut self, provider: Arc<dyn crate::channels::SmsProvider>) -> Self {
        use crate::channels::SmsChannel;
        let channel = Arc::new(SmsChannel::new(provider));
        self.notifier.register_channel(Channel::Sms, channel);
        self
    }

    #[cfg(feature = "slack")]
    /// Register a Slack channel with webhook URL
    pub fn slack(mut self, webhook_url: impl Into<String>) -> Self {
        use crate::channels::SlackChannel;
        let channel = Arc::new(SlackChannel::with_webhook(webhook_url));
        self.notifier.register_channel(Channel::Slack, channel);
        self
    }

    /// Build the notifier
    pub fn build(self) -> Notifier {
        self.notifier
    }
}

impl Default for NotifierBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{DatabaseNotification, MailMessage};
    use async_trait::async_trait;

    struct TestUser {
        email: String,
    }

    impl Notifiable for TestUser {
        fn route_notification_for_mail(&self) -> Option<String> {
            Some(self.email.clone())
        }

        fn route_notification_for_database(&self) -> Option<i64> {
            Some(1)
        }
    }

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Mail, Channel::Database]
        }

        async fn to_mail(&self) -> Option<MailMessage> {
            Some(MailMessage::new().subject("Test").greeting("Hello"))
        }

        async fn to_database(&self) -> Option<DatabaseNotification> {
            Some(DatabaseNotification::new().title("Test"))
        }
    }

    // Mock channel for testing
    struct MockChannel;

    #[async_trait]
    impl NotificationChannel for MockChannel {
        async fn send(
            &self,
            _notification: &dyn Notification,
            _notifiable: &dyn Notifiable,
        ) -> NotificationResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_notifier_register_and_send() {
        let mut notifier = Notifier::new();
        notifier.register_channel(Channel::Mail, Arc::new(MockChannel));
        notifier.register_channel(Channel::Database, Arc::new(MockChannel));

        let user = TestUser {
            email: "user@example.com".to_string(),
        };

        let notification = TestNotification;
        let result = notifier.send(&notification, &user).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notifier_missing_channel() {
        let notifier = Notifier::new();
        let user = TestUser {
            email: "user@example.com".to_string(),
        };

        let notification = TestNotification;
        let result = notifier.send(&notification, &user).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_notifier_has_channel() {
        let mut notifier = Notifier::new();
        assert!(!notifier.has_channel(&Channel::Mail));

        notifier.register_channel(Channel::Mail, Arc::new(MockChannel));
        assert!(notifier.has_channel(&Channel::Mail));
    }

    #[test]
    fn test_notifier_builder() {
        let notifier = NotifierBuilder::new()
            .channel(Channel::Mail, Arc::new(MockChannel))
            .channel(Channel::Database, Arc::new(MockChannel))
            .build();

        assert!(notifier.has_channel(&Channel::Mail));
        assert!(notifier.has_channel(&Channel::Database));
        assert!(!notifier.has_channel(&Channel::Sms));
    }
}
