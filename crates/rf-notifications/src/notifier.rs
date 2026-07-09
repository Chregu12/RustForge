//! Central notification dispatcher

use crate::channels::NotificationChannel;
use crate::{Channel, Notifiable, Notification, NotificationError, NotificationResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Per-channel outcome of delivering one notification across its `via()` channels.
///
/// Produced by [`Notifier::send_report`]. Unlike a single `Result`, this records
/// what actually happened on **every** channel, so one down channel (e.g. a Slack
/// webhook that is unreachable) does not hide the fact that the database row and
/// the email were still delivered.
#[derive(Debug, Default)]
pub struct DeliveryReport {
    /// Channels that accepted the notification.
    pub delivered: Vec<Channel>,
    /// Channels that failed, paired with the error message describing why.
    pub failed: Vec<(Channel, String)>,
}

impl DeliveryReport {
    /// `true` when every channel delivered successfully.
    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// Total number of channels attempted.
    pub fn attempted(&self) -> usize {
        self.delivered.len() + self.failed.len()
    }
}

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

    /// Send a notification to a notifiable entity across all of its channels.
    ///
    /// Attempts **every** channel returned by [`Notification::via`] and delivers
    /// each independently: one failing channel (a down Slack webhook, a missing
    /// handler) no longer aborts the rest, so a later `via()` entry such as the
    /// database row or the email still goes out. The returned `Result` is an
    /// aggregate — `Ok(())` when every channel delivered, otherwise `Err`
    /// naming exactly which channels failed and why. Use [`Notifier::send_report`]
    /// for the full per-channel [`DeliveryReport`].
    ///
    /// Note on queuing: [`Notification::should_queue`] is currently advisory only
    /// — this dispatcher always delivers synchronously (see the trait docs for
    /// why a queued-notification path is not yet wired).
    pub async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        let report = self.send_report(notification, notifiable).await;

        if report.is_success() {
            Ok(())
        } else {
            let detail = report
                .failed
                .iter()
                .map(|(channel, err)| format!("{channel:?}: {err}"))
                .collect::<Vec<_>>()
                .join("; ");
            Err(NotificationError::ChannelError(format!(
                "{} of {} channel(s) failed: {}",
                report.failed.len(),
                report.attempted(),
                detail
            )))
        }
    }

    /// Deliver a notification across every channel and return a per-channel
    /// [`DeliveryReport`], never aborting early.
    ///
    /// This is the aggregate primitive behind [`Notifier::send`]: it walks every
    /// channel from [`Notification::via`], attempts delivery on each, and records
    /// the outcome (delivered vs. failed-with-reason). A missing handler or a
    /// channel that returns an error is captured in `failed` — the loop continues
    /// so healthy channels still deliver.
    pub async fn send_report(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> DeliveryReport {
        let mut report = DeliveryReport::default();

        for channel in notification.via() {
            match self.channels.get(&channel) {
                Some(handler) => match handler.send(notification, notifiable).await {
                    Ok(()) => report.delivered.push(channel),
                    Err(err) => report.failed.push((channel, err.to_string())),
                },
                None => {
                    let reason = format!("No handler registered for channel: {channel:?}");
                    report.failed.push((channel, reason));
                }
            }
        }

        report
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

    // A channel that always fails, to prove one down channel does not abort the
    // rest of the delivery.
    struct FailingChannel;
    #[async_trait]
    impl NotificationChannel for FailingChannel {
        async fn send(
            &self,
            _notification: &dyn Notification,
            _notifiable: &dyn Notifiable,
        ) -> NotificationResult<()> {
            Err(NotificationError::ChannelError("simulated down channel".into()))
        }
    }

    // A channel that records every delivery it accepts.
    struct RecordingChannel {
        hits: Arc<std::sync::Mutex<usize>>,
    }
    #[async_trait]
    impl NotificationChannel for RecordingChannel {
        async fn send(
            &self,
            _notification: &dyn Notification,
            _notifiable: &dyn Notifiable,
        ) -> NotificationResult<()> {
            *self.hits.lock().unwrap() += 1;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_failing_channel_does_not_abort_healthy_channel() {
        // Mail channel fails, Database channel is healthy. The old behavior
        // aborted on the first error so the DB row never landed; the aggregate
        // behavior must still deliver the healthy channel.
        let hits = Arc::new(std::sync::Mutex::new(0usize));
        let mut notifier = Notifier::new();
        notifier.register_channel(Channel::Mail, Arc::new(FailingChannel));
        notifier.register_channel(
            Channel::Database,
            Arc::new(RecordingChannel { hits: Arc::clone(&hits) }),
        );

        let user = TestUser {
            email: "user@example.com".to_string(),
        };

        // send() reports failure in aggregate...
        let result = notifier.send(&TestNotification, &user).await;
        assert!(result.is_err(), "aggregate result should report the failed channel");
        // ...but the healthy Database channel still delivered.
        assert_eq!(*hits.lock().unwrap(), 1, "healthy channel must still deliver");

        // The per-channel report names exactly what happened.
        let report = notifier.send_report(&TestNotification, &user).await;
        assert_eq!(report.delivered, vec![Channel::Database]);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.failed[0].0, Channel::Mail);
        assert!(!report.is_success());
        assert_eq!(report.attempted(), 2);
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
