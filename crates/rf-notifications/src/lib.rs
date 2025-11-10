//! Notifications System for RustForge
//!
//! This crate provides multi-channel notification delivery.

use async_trait::async_trait;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
};
use thiserror::Error;

/// Notification errors
#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Routing error: {0}")]
    RoutingError(String),

    #[error("Send error: {0}")]
    SendError(String),
}

pub type NotificationResult<T> = Result<T, NotificationError>;

/// Notification channels
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    Email,
    Sms,
    Push,
    Database,
}

/// Mail message
#[derive(Debug, Clone)]
pub struct MailMessage {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
    pub from: Option<String>,
}

impl MailMessage {
    pub fn new() -> Self {
        Self {
            to: Vec::new(),
            subject: String::new(),
            body: String::new(),
            from: None,
        }
    }

    pub fn to(mut self, email: impl Into<String>) -> Self {
        self.to.push(email.into());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }
}

impl Default for MailMessage {
    fn default() -> Self {
        Self::new()
    }
}

/// SMS message
#[derive(Debug, Clone)]
pub struct SmsMessage {
    pub to: String,
    pub body: String,
}

impl SmsMessage {
    pub fn new(to: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            body: body.into(),
        }
    }
}

/// Push notification
#[derive(Debug, Clone)]
pub struct PushMessage {
    pub title: String,
    pub body: String,
    pub data: HashMap<String, String>,
}

impl PushMessage {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            data: HashMap::new(),
        }
    }

    pub fn data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

/// Database notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseNotification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl DatabaseNotification {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: String::new(),
            body: String::new(),
            data: serde_json::Value::Null,
            read_at: None,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn mark_as_read(&mut self) {
        self.read_at = Some(chrono::Utc::now());
    }

    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }
}

impl Default for DatabaseNotification {
    fn default() -> Self {
        Self::new()
    }
}

/// Notifiable entity (user)
pub trait Notifiable: Send + Sync {
    /// Get email address
    fn email(&self) -> Option<String> {
        None
    }

    /// Get phone number
    fn phone(&self) -> Option<String> {
        None
    }

    /// Get push token
    fn push_token(&self) -> Option<String> {
        None
    }

    /// Get user ID for database notifications
    fn id(&self) -> String;
}

/// Notification trait
#[async_trait]
pub trait Notification: Send + Sync {
    /// Determine which channels to use
    fn via(&self, notifiable: &dyn Notifiable) -> Vec<Channel>;

    /// Convert to mail message
    fn to_mail(&self, _notifiable: &dyn Notifiable) -> NotificationResult<MailMessage> {
        Err(NotificationError::ChannelError(
            "Mail channel not implemented".to_string(),
        ))
    }

    /// Convert to SMS message
    fn to_sms(&self, _notifiable: &dyn Notifiable) -> NotificationResult<SmsMessage> {
        Err(NotificationError::ChannelError(
            "SMS channel not implemented".to_string(),
        ))
    }

    /// Convert to push notification
    fn to_push(&self, _notifiable: &dyn Notifiable) -> NotificationResult<PushMessage> {
        Err(NotificationError::ChannelError(
            "Push channel not implemented".to_string(),
        ))
    }

    /// Convert to database notification
    fn to_database(&self, _notifiable: &dyn Notifiable) -> NotificationResult<DatabaseNotification> {
        Err(NotificationError::ChannelError(
            "Database channel not implemented".to_string(),
        ))
    }

    /// Check if notification should be queued
    fn should_queue(&self) -> bool {
        false
    }
}

/// Channel handler trait
#[async_trait]
pub trait ChannelHandler: Send + Sync {
    /// Send via this channel
    async fn send(&self, notification: &dyn Notification, notifiable: &dyn Notifiable) -> NotificationResult<()>;
}

/// Email channel handler
pub struct EmailChannel {
    // In real implementation, this would hold SMTP config
}

impl EmailChannel {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for EmailChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelHandler for EmailChannel {
    async fn send(&self, notification: &dyn Notification, notifiable: &dyn Notifiable) -> NotificationResult<()> {
        let message = notification.to_mail(notifiable)?;

        // In real implementation, send via SMTP
        // For now, just log
        println!("Sending email to {:?}: {}", message.to, message.subject);

        Ok(())
    }
}

/// SMS channel handler
pub struct SmsChannel {
    // In real implementation, this would hold Twilio/SNS config
}

impl SmsChannel {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SmsChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelHandler for SmsChannel {
    async fn send(&self, notification: &dyn Notification, notifiable: &dyn Notifiable) -> NotificationResult<()> {
        let message = notification.to_sms(notifiable)?;

        // In real implementation, send via Twilio/SNS
        println!("Sending SMS to {}: {}", message.to, message.body);

        Ok(())
    }
}

/// Push channel handler
pub struct PushChannel {
    // In real implementation, this would hold FCM/APNS config
}

impl PushChannel {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PushChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelHandler for PushChannel {
    async fn send(&self, notification: &dyn Notification, notifiable: &dyn Notifiable) -> NotificationResult<()> {
        let message = notification.to_push(notifiable)?;

        // In real implementation, send via FCM/APNS
        println!("Sending push: {}", message.title);

        Ok(())
    }
}

/// Database channel handler (stores in memory for testing)
pub struct DatabaseChannel {
    notifications: Arc<tokio::sync::RwLock<HashMap<String, Vec<DatabaseNotification>>>>,
}

impl DatabaseChannel {
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Get notifications for a user
    pub async fn get_notifications(&self, user_id: &str) -> Vec<DatabaseNotification> {
        let notifications = self.notifications.read().await;
        notifications
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Mark notification as read
    pub async fn mark_as_read(&self, user_id: &str, notification_id: &str) -> NotificationResult<()> {
        let mut notifications = self.notifications.write().await;

        if let Some(user_notifications) = notifications.get_mut(user_id) {
            if let Some(notification) = user_notifications.iter_mut().find(|n| n.id == notification_id) {
                notification.mark_as_read();
                return Ok(());
            }
        }

        Err(NotificationError::SendError("Notification not found".to_string()))
    }

    /// Get unread count
    pub async fn unread_count(&self, user_id: &str) -> usize {
        let notifications = self.notifications.read().await;
        notifications
            .get(user_id)
            .map(|n| n.iter().filter(|notif| !notif.is_read()).count())
            .unwrap_or(0)
    }
}

impl Default for DatabaseChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelHandler for DatabaseChannel {
    async fn send(&self, notification: &dyn Notification, notifiable: &dyn Notifiable) -> NotificationResult<()> {
        let message = notification.to_database(notifiable)?;
        let user_id = notifiable.id();

        let mut notifications = self.notifications.write().await;
        notifications
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(message);

        Ok(())
    }
}

/// Notification manager
pub struct NotificationManager {
    channels: HashMap<Channel, Arc<dyn ChannelHandler>>,
    templates: Handlebars<'static>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            templates: Handlebars::new(),
        }
    }

    /// Register a channel handler
    pub fn register_channel(&mut self, channel: Channel, handler: Arc<dyn ChannelHandler>) {
        self.channels.insert(channel, handler);
    }

    /// Register a template
    pub fn register_template(&mut self, name: &str, template: &str) -> NotificationResult<()> {
        self.templates
            .register_template_string(name, template)
            .map_err(|e| NotificationError::TemplateError(e.to_string()))
    }

    /// Send notification to a notifiable entity
    pub async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        let channels = notification.via(notifiable);

        for channel in channels {
            if let Some(handler) = self.channels.get(&channel) {
                handler.send(notification, notifiable).await?;
            } else {
                return Err(NotificationError::RoutingError(format!(
                    "No handler for channel: {:?}",
                    channel
                )));
            }
        }

        Ok(())
    }

    /// Render a template
    pub fn render_template(
        &self,
        name: &str,
        data: &serde_json::Value,
    ) -> NotificationResult<String> {
        self.templates
            .render(name, data)
            .map_err(|e| NotificationError::TemplateError(e.to_string()))
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestUser {
        id: String,
        email: String,
        phone: String,
    }

    impl Notifiable for TestUser {
        fn email(&self) -> Option<String> {
            Some(self.email.clone())
        }

        fn phone(&self) -> Option<String> {
            Some(self.phone.clone())
        }

        fn id(&self) -> String {
            self.id.clone()
        }
    }

    struct WelcomeNotification;

    #[async_trait]
    impl Notification for WelcomeNotification {
        fn via(&self, _notifiable: &dyn Notifiable) -> Vec<Channel> {
            vec![Channel::Email, Channel::Database]
        }

        fn to_mail(&self, notifiable: &dyn Notifiable) -> NotificationResult<MailMessage> {
            Ok(MailMessage::new()
                .to(notifiable.email().unwrap())
                .subject("Welcome!")
                .body("Welcome to RustForge"))
        }

        fn to_database(&self, _notifiable: &dyn Notifiable) -> NotificationResult<DatabaseNotification> {
            Ok(DatabaseNotification::new()
                .title("Welcome")
                .body("Welcome to RustForge"))
        }
    }

    #[tokio::test]
    async fn test_mail_message_builder() {
        let message = MailMessage::new()
            .to("test@example.com")
            .subject("Test")
            .body("Hello")
            .from("sender@example.com");

        assert_eq!(message.to, vec!["test@example.com"]);
        assert_eq!(message.subject, "Test");
        assert_eq!(message.body, "Hello");
        assert_eq!(message.from, Some("sender@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_sms_message() {
        let message = SmsMessage::new("+1234567890", "Test message");
        assert_eq!(message.to, "+1234567890");
        assert_eq!(message.body, "Test message");
    }

    #[tokio::test]
    async fn test_push_message() {
        let message = PushMessage::new("Title", "Body")
            .data("key", "value");

        assert_eq!(message.title, "Title");
        assert_eq!(message.body, "Body");
        assert_eq!(message.data.get("key"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_database_notification() {
        let mut notification = DatabaseNotification::new()
            .title("Test")
            .body("Test body");

        assert!(!notification.is_read());
        notification.mark_as_read();
        assert!(notification.is_read());
    }

    #[tokio::test]
    async fn test_notification_manager() {
        let mut manager = NotificationManager::new();
        let email_channel = Arc::new(EmailChannel::new());
        let db_channel = Arc::new(DatabaseChannel::new());

        manager.register_channel(Channel::Email, email_channel);
        manager.register_channel(Channel::Database, db_channel.clone());

        let user = TestUser {
            id: "1".to_string(),
            email: "user@example.com".to_string(),
            phone: "+1234567890".to_string(),
        };

        let notification = WelcomeNotification;
        manager.send(&notification, &user).await.unwrap();

        // Check database notification was stored
        let notifications = db_channel.get_notifications("1").await;
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].title, "Welcome");
    }

    #[tokio::test]
    async fn test_database_channel() {
        let channel = DatabaseChannel::new();
        let user = TestUser {
            id: "1".to_string(),
            email: "user@example.com".to_string(),
            phone: "+1234567890".to_string(),
        };

        let notification = WelcomeNotification;
        channel.send(&notification, &user).await.unwrap();

        let notifications = channel.get_notifications("1").await;
        assert_eq!(notifications.len(), 1);
        assert_eq!(channel.unread_count("1").await, 1);

        channel.mark_as_read("1", &notifications[0].id).await.unwrap();
        assert_eq!(channel.unread_count("1").await, 0);
    }

    #[tokio::test]
    async fn test_template_rendering() {
        let mut manager = NotificationManager::new();
        manager
            .register_template("welcome", "Hello {{name}}!")
            .unwrap();

        let data = serde_json::json!({ "name": "John" });
        let rendered = manager.render_template("welcome", &data).unwrap();
        assert_eq!(rendered, "Hello John!");
    }

    #[tokio::test]
    async fn test_multiple_channels() {
        struct MultiChannelNotification;

        #[async_trait]
        impl Notification for MultiChannelNotification {
            fn via(&self, _notifiable: &dyn Notifiable) -> Vec<Channel> {
                vec![Channel::Email, Channel::Sms, Channel::Database]
            }

            fn to_mail(&self, notifiable: &dyn Notifiable) -> NotificationResult<MailMessage> {
                Ok(MailMessage::new()
                    .to(notifiable.email().unwrap())
                    .subject("Test"))
            }

            fn to_sms(&self, notifiable: &dyn Notifiable) -> NotificationResult<SmsMessage> {
                Ok(SmsMessage::new(notifiable.phone().unwrap(), "Test"))
            }

            fn to_database(&self, _notifiable: &dyn Notifiable) -> NotificationResult<DatabaseNotification> {
                Ok(DatabaseNotification::new().title("Test"))
            }
        }

        let mut manager = NotificationManager::new();
        manager.register_channel(Channel::Email, Arc::new(EmailChannel::new()));
        manager.register_channel(Channel::Sms, Arc::new(SmsChannel::new()));
        manager.register_channel(Channel::Database, Arc::new(DatabaseChannel::new()));

        let user = TestUser {
            id: "1".to_string(),
            email: "user@example.com".to_string(),
            phone: "+1234567890".to_string(),
        };

        let notification = MultiChannelNotification;
        manager.send(&notification, &user).await.unwrap();
    }

    #[tokio::test]
    async fn test_missing_channel_handler() {
        let manager = NotificationManager::new();
        let user = TestUser {
            id: "1".to_string(),
            email: "user@example.com".to_string(),
            phone: "+1234567890".to_string(),
        };

        let notification = WelcomeNotification;
        let result = manager.send(&notification, &user).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unread_count() {
        let channel = DatabaseChannel::new();
        let user = TestUser {
            id: "1".to_string(),
            email: "user@example.com".to_string(),
            phone: "+1234567890".to_string(),
        };

        // Send 3 notifications
        for _ in 0..3 {
            channel.send(&WelcomeNotification, &user).await.unwrap();
        }

        assert_eq!(channel.unread_count("1").await, 3);

        let notifications = channel.get_notifications("1").await;
        channel.mark_as_read("1", &notifications[0].id).await.unwrap();

        assert_eq!(channel.unread_count("1").await, 2);
    }
}
