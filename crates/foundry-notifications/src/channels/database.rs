use super::{Channel, ChannelResult};
use crate::notification::{DatabaseNotification, Notification};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

/// In-memory database channel (for demonstration)
pub struct DatabaseChannel {
    storage: Arc<Mutex<Vec<DatabaseNotification>>>,
}

impl DatabaseChannel {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_all(&self) -> Vec<DatabaseNotification> {
        self.storage.lock().await.clone()
    }

    pub async fn mark_as_read(&self, id: &str) {
        let mut storage = self.storage.lock().await;
        if let Some(notif) = storage.iter_mut().find(|n| n.id == id) {
            notif.read_at = Some(chrono::Utc::now());
        }
    }
}

impl Default for DatabaseChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Channel for DatabaseChannel {
    fn name(&self) -> &str {
        "database"
    }

    async fn send(&self, notification: &dyn Notification, _recipient: &dyn std::any::Any) -> ChannelResult {
        let db_notif = notification.to_database();
        let mut storage = self.storage.lock().await;
        storage.push(db_notif);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification::SimpleNotification;

    #[tokio::test]
    async fn test_database_channel() {
        let channel = DatabaseChannel::new();
        let notif = SimpleNotification::new("Test", "Message");

        let recipient = ();
        channel.send(&notif, &recipient).await.unwrap();

        let all = channel.get_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "Test");
    }
}
