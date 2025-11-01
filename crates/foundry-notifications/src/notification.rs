use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Notification trait
#[async_trait]
pub trait Notification: Send + Sync {
    /// Get notification title
    fn title(&self) -> String;

    /// Get notification body/message
    fn body(&self) -> String;

    /// Get notification data
    fn data(&self) -> NotificationData {
        NotificationData::default()
    }

    /// Get channels to send through
    fn via(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    /// Convert to database format
    fn to_database(&self) -> DatabaseNotification {
        DatabaseNotification {
            id: Uuid::new_v4().to_string(),
            title: self.title(),
            body: self.body(),
            data: self.data(),
            read_at: None,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Notification data (key-value pairs)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotificationData {
    pub fields: HashMap<String, serde_json::Value>,
}

impl NotificationData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.fields.get(key)
    }
}

/// Database notification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseNotification {
    pub id: String,
    pub title: String,
    pub body: String,
    pub data: NotificationData,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Simple notification implementation
pub struct SimpleNotification {
    pub title: String,
    pub body: String,
    pub data: NotificationData,
    pub channels: Vec<String>,
}

impl SimpleNotification {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            data: NotificationData::new(),
            channels: vec!["database".to_string()],
        }
    }

    pub fn with_data(mut self, data: NotificationData) -> Self {
        self.data = data;
        self
    }

    pub fn with_channels(mut self, channels: Vec<String>) -> Self {
        self.channels = channels;
        self
    }
}

#[async_trait]
impl Notification for SimpleNotification {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn body(&self) -> String {
        self.body.clone()
    }

    fn data(&self) -> NotificationData {
        self.data.clone()
    }

    fn via(&self) -> Vec<String> {
        self.channels.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Send error: {0}")]
    Send(String),

    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_notification() {
        let notif = SimpleNotification::new("Test", "Message");
        assert_eq!(notif.title(), "Test");
        assert_eq!(notif.body(), "Message");
    }

    #[test]
    fn test_notification_data() {
        let data = NotificationData::new()
            .with("key", "value")
            .with("count", 42);

        assert!(data.get("key").is_some());
        assert!(data.get("count").is_some());
    }
}
