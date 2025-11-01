use crate::channels::Channel;
use crate::notification::{Notification, NotificationError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Notification manager
pub struct NotificationManager {
    channels: HashMap<String, Arc<dyn Channel>>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    /// Register a notification channel
    pub fn register_channel(&mut self, channel: Arc<dyn Channel>) {
        let name = channel.name().to_string();
        debug!(channel = %name, "Registering notification channel");
        self.channels.insert(name, channel);
    }

    /// Send notification to a recipient
    pub async fn send(
        &self,
        notification: &dyn Notification,
        recipient: &dyn std::any::Any,
    ) -> Result<(), NotificationError> {
        let channels = notification.via();

        for channel_name in channels {
            if let Some(channel) = self.channels.get(&channel_name) {
                if !channel.is_ready().await {
                    error!(channel = %channel_name, "Channel not ready");
                    continue;
                }

                match channel.send(notification, recipient).await {
                    Ok(_) => {
                        info!(
                            channel = %channel_name,
                            title = %notification.title(),
                            "Notification sent successfully"
                        );
                    }
                    Err(e) => {
                        error!(
                            channel = %channel_name,
                            error = %e,
                            "Failed to send notification"
                        );
                    }
                }
            } else {
                error!(channel = %channel_name, "Channel not registered");
            }
        }

        Ok(())
    }

    /// Send to multiple recipients
    pub async fn send_to_many(
        &self,
        notification: &dyn Notification,
        recipients: &[&dyn std::any::Any],
    ) -> Result<(), NotificationError> {
        for recipient in recipients {
            self.send(notification, *recipient).await?;
        }
        Ok(())
    }

    /// List registered channels
    pub fn channels(&self) -> Vec<String> {
        self.channels.keys().cloned().collect()
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
    use crate::channels::DatabaseChannel;
    use crate::notification::SimpleNotification;

    #[tokio::test]
    async fn test_notification_manager() {
        let mut manager = NotificationManager::new();
        let channel = Arc::new(DatabaseChannel::new());
        manager.register_channel(channel);

        let notif = SimpleNotification::new("Test", "Message");
        let recipient = ();

        manager.send(&notif, &recipient).await.unwrap();

        let channels = manager.channels();
        assert_eq!(channels.len(), 1);
    }
}
