//! WebSocket Channel Management
//!
//! Channels ermöglichen es, Clients in Gruppen zu organisieren und
//! Nachrichten gezielt an bestimmte Gruppen zu senden.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::connection::ConnectionId;
use super::message::WebSocketMessage;

/// Ein WebSocket-Channel für gruppierte Kommunikation
#[derive(Debug, Clone)]
pub struct Channel {
    /// Der Name des Channels
    pub name: String,
    /// Beschreibung des Channels
    pub description: Option<String>,
    /// Ob der Channel privat ist
    pub is_private: bool,
    /// Erstellt am
    pub created_at: i64,
}

impl Channel {
    /// Erstellt einen neuen Channel
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            is_private: false,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Setzt die Beschreibung
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Macht den Channel privat
    pub fn make_private(mut self) -> Self {
        self.is_private = true;
        self
    }
}

/// Verwaltet Channels und ihre Subscriptions
pub struct ChannelManager {
    /// Channels und ihre Subscriber
    channels: Arc<RwLock<HashMap<String, HashSet<ConnectionId>>>>,
    /// Channel-Metadaten
    metadata: Arc<RwLock<HashMap<String, Channel>>>,
}

impl ChannelManager {
    /// Erstellt einen neuen ChannelManager
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Erstellt einen neuen Channel
    ///
    /// # Beispiel
    ///
    /// ```no_run
    /// use foundry_api::websocket::{ChannelManager, Channel};
    ///
    /// # async fn example() {
    /// let manager = ChannelManager::new();
    /// let channel = Channel::new("chat:room1")
    ///     .with_description("Chat Room 1");
    /// manager.create_channel(channel).await;
    /// # }
    /// ```
    pub async fn create_channel(&self, channel: Channel) {
        let channel_name = channel.name.clone();
        let mut metadata = self.metadata.write().await;
        metadata.insert(channel_name.clone(), channel);

        let mut channels = self.channels.write().await;
        channels.entry(channel_name.clone()).or_insert_with(HashSet::new);

        info!(channel = %channel_name, "Channel created");
    }

    /// Abonniert einen Channel
    ///
    /// # Argumente
    ///
    /// * `channel_name` - Der Name des Channels
    /// * `connection_id` - Die Connection-ID des Subscribers
    ///
    /// # Rückgabe
    ///
    /// `true` wenn erfolgreich abonniert, `false` wenn bereits abonniert
    pub async fn subscribe(&self, channel_name: &str, connection_id: ConnectionId) -> bool {
        let mut channels = self.channels.write().await;
        let subscribers = channels.entry(channel_name.to_string()).or_insert_with(HashSet::new);

        let is_new = subscribers.insert(connection_id);

        if is_new {
            info!(
                channel = %channel_name,
                connection_id = %connection_id,
                "Client subscribed to channel"
            );
        }

        is_new
    }

    /// Deabonniert einen Channel
    ///
    /// # Argumente
    ///
    /// * `channel_name` - Der Name des Channels
    /// * `connection_id` - Die Connection-ID des Subscribers
    ///
    /// # Rückgabe
    ///
    /// `true` wenn erfolgreich deabonniert, `false` wenn nicht abonniert war
    pub async fn unsubscribe(&self, channel_name: &str, connection_id: ConnectionId) -> bool {
        let mut channels = self.channels.write().await;

        if let Some(subscribers) = channels.get_mut(channel_name) {
            let was_subscribed = subscribers.remove(&connection_id);

            if was_subscribed {
                info!(
                    channel = %channel_name,
                    connection_id = %connection_id,
                    "Client unsubscribed from channel"
                );
            }

            // Cleanup leerer Channels (optional)
            if subscribers.is_empty() {
                debug!(channel = %channel_name, "Channel is now empty");
            }

            was_subscribed
        } else {
            false
        }
    }

    /// Deabonniert eine Connection von allen Channels
    pub async fn unsubscribe_all(&self, connection_id: ConnectionId) {
        let mut channels = self.channels.write().await;
        let mut unsubscribed_channels = Vec::new();

        for (channel_name, subscribers) in channels.iter_mut() {
            if subscribers.remove(&connection_id) {
                unsubscribed_channels.push(channel_name.clone());
            }
        }

        if !unsubscribed_channels.is_empty() {
            info!(
                connection_id = %connection_id,
                channels = ?unsubscribed_channels,
                "Client unsubscribed from all channels"
            );
        }
    }

    /// Gibt alle Subscriber eines Channels zurück
    pub async fn get_subscribers(&self, channel_name: &str) -> Vec<ConnectionId> {
        let channels = self.channels.read().await;
        channels
            .get(channel_name)
            .map(|subscribers| subscribers.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Gibt alle Channels zurück, die eine Connection abonniert hat
    pub async fn get_subscribed_channels(&self, connection_id: ConnectionId) -> Vec<String> {
        let channels = self.channels.read().await;
        channels
            .iter()
            .filter_map(|(name, subscribers)| {
                if subscribers.contains(&connection_id) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Gibt die Anzahl der Subscriber in einem Channel zurück
    pub async fn subscriber_count(&self, channel_name: &str) -> usize {
        let channels = self.channels.read().await;
        channels
            .get(channel_name)
            .map(|subscribers| subscribers.len())
            .unwrap_or(0)
    }

    /// Gibt alle Channel-Namen zurück
    pub async fn list_channels(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }

    /// Gibt Channel-Metadaten zurück
    pub async fn get_channel_metadata(&self, channel_name: &str) -> Option<Channel> {
        let metadata = self.metadata.read().await;
        metadata.get(channel_name).cloned()
    }

    /// Löscht einen Channel
    pub async fn delete_channel(&self, channel_name: &str) -> bool {
        let mut channels = self.channels.write().await;
        let mut metadata = self.metadata.write().await;

        let removed = channels.remove(channel_name).is_some();
        metadata.remove(channel_name);

        if removed {
            info!(channel = %channel_name, "Channel deleted");
        }

        removed
    }

    /// Prüft, ob ein Channel existiert
    pub async fn channel_exists(&self, channel_name: &str) -> bool {
        let channels = self.channels.read().await;
        channels.contains_key(channel_name)
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_creation() {
        let manager = ChannelManager::new();
        let channel = Channel::new("test-channel");
        manager.create_channel(channel).await;

        assert!(manager.channel_exists("test-channel").await);
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let manager = ChannelManager::new();
        let channel = Channel::new("test-channel");
        manager.create_channel(channel).await;

        let conn_id = ConnectionId::new();

        // Subscribe
        let subscribed = manager.subscribe("test-channel", conn_id).await;
        assert!(subscribed);

        // Check subscriber count
        assert_eq!(manager.subscriber_count("test-channel").await, 1);

        // Unsubscribe
        let unsubscribed = manager.unsubscribe("test-channel", conn_id).await;
        assert!(unsubscribed);

        assert_eq!(manager.subscriber_count("test-channel").await, 0);
    }

    #[tokio::test]
    async fn test_get_subscribed_channels() {
        let manager = ChannelManager::new();

        let channel1 = Channel::new("channel1");
        let channel2 = Channel::new("channel2");
        manager.create_channel(channel1).await;
        manager.create_channel(channel2).await;

        let conn_id = ConnectionId::new();
        manager.subscribe("channel1", conn_id).await;
        manager.subscribe("channel2", conn_id).await;

        let channels = manager.get_subscribed_channels(conn_id).await;
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&"channel1".to_string()));
        assert!(channels.contains(&"channel2".to_string()));
    }

    #[tokio::test]
    async fn test_unsubscribe_all() {
        let manager = ChannelManager::new();

        let channel1 = Channel::new("channel1");
        let channel2 = Channel::new("channel2");
        manager.create_channel(channel1).await;
        manager.create_channel(channel2).await;

        let conn_id = ConnectionId::new();
        manager.subscribe("channel1", conn_id).await;
        manager.subscribe("channel2", conn_id).await;

        manager.unsubscribe_all(conn_id).await;

        let channels = manager.get_subscribed_channels(conn_id).await;
        assert_eq!(channels.len(), 0);
    }
}
