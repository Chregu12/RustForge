//! WebSocket Connection Manager
//!
//! Der zentrale Manager für alle WebSocket-Verbindungen, Broadcasting und Channels.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use super::channel::ChannelManager;
use super::connection::{Connection, ConnectionId, ConnectionMetadata};
use super::message::WebSocketMessage;

/// Optionen für Broadcasting
#[derive(Debug, Clone)]
pub struct BroadcastOptions {
    /// Sende nur an spezifische Connections
    pub only_to: Option<Vec<ConnectionId>>,
    /// Schließe spezifische Connections aus
    pub exclude: Option<Vec<ConnectionId>>,
    /// Sende nur an Connections in einem bestimmten Channel
    pub channel: Option<String>,
}

impl Default for BroadcastOptions {
    fn default() -> Self {
        Self {
            only_to: None,
            exclude: None,
            channel: None,
        }
    }
}

impl BroadcastOptions {
    /// Erstellt neue Standard-Optionen
    pub fn new() -> Self {
        Self::default()
    }

    /// Sendet nur an die angegebenen Connections
    pub fn only_to(mut self, connection_ids: Vec<ConnectionId>) -> Self {
        self.only_to = Some(connection_ids);
        self
    }

    /// Schließt die angegebenen Connections vom Broadcast aus
    pub fn exclude(mut self, connection_ids: Vec<ConnectionId>) -> Self {
        self.exclude = Some(connection_ids);
        self
    }

    /// Sendet nur an Connections in einem bestimmten Channel
    pub fn to_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// Prüft, ob eine Connection-ID inkludiert werden soll
    fn should_include(&self, id: &ConnectionId) -> bool {
        // Wenn only_to gesetzt ist, nur diese IDs verwenden
        if let Some(ref only) = self.only_to {
            if !only.contains(id) {
                return false;
            }
        }

        // Wenn exclude gesetzt ist, diese IDs ausschließen
        if let Some(ref exclude) = self.exclude {
            if exclude.contains(id) {
                return false;
            }
        }

        true
    }
}

/// Der zentrale WebSocket Manager
#[derive(Clone)]
pub struct WebSocketManager {
    /// Alle aktiven Verbindungen
    connections: Arc<RwLock<HashMap<ConnectionId, mpsc::UnboundedSender<WebSocketMessage>>>>,
    /// Connection-Metadaten
    metadata: Arc<RwLock<HashMap<ConnectionId, ConnectionMetadata>>>,
    /// Channel-Manager
    channel_manager: Arc<ChannelManager>,
}

impl WebSocketManager {
    /// Erstellt einen neuen WebSocket Manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            channel_manager: Arc::new(ChannelManager::new()),
        }
    }

    /// Registriert eine neue Verbindung
    ///
    /// # Argumente
    ///
    /// * `connection` - Die Connection
    /// * `sender` - Der Message-Sender für diese Connection
    ///
    /// # Rückgabe
    ///
    /// Die Connection-ID
    pub async fn register_connection(
        &self,
        connection: &Connection,
        sender: mpsc::UnboundedSender<WebSocketMessage>,
    ) -> ConnectionId {
        let id = connection.id;
        let metadata = connection.metadata.clone();

        let mut connections = self.connections.write().await;
        connections.insert(id, sender);

        let mut meta = self.metadata.write().await;
        meta.insert(id, metadata);

        info!(connection_id = %id, "WebSocket connection registered");

        id
    }

    /// Entfernt eine Verbindung
    ///
    /// # Argumente
    ///
    /// * `connection_id` - Die Connection-ID
    pub async fn unregister_connection(&self, connection_id: ConnectionId) {
        let mut connections = self.connections.write().await;
        connections.remove(&connection_id);

        let mut metadata = self.metadata.write().await;
        metadata.remove(&connection_id);

        // Von allen Channels deabonnieren
        self.channel_manager.unsubscribe_all(connection_id).await;

        info!(connection_id = %connection_id, "WebSocket connection unregistered");
    }

    /// Sendet eine Nachricht an eine spezifische Connection
    ///
    /// # Argumente
    ///
    /// * `connection_id` - Die Connection-ID
    /// * `message` - Die zu sendende Nachricht
    ///
    /// # Rückgabe
    ///
    /// `Ok(())` wenn erfolgreich gesendet, `Err` wenn Connection nicht existiert
    pub async fn send_to_connection(
        &self,
        connection_id: ConnectionId,
        message: WebSocketMessage,
    ) -> anyhow::Result<()> {
        let connections = self.connections.read().await;

        if let Some(sender) = connections.get(&connection_id) {
            sender.send(message)?;
            debug!(connection_id = %connection_id, "Message sent to connection");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Connection not found: {}", connection_id))
        }
    }

    /// Broadcast eine Nachricht an alle Verbindungen
    ///
    /// # Argumente
    ///
    /// * `message` - Die zu sendende Nachricht
    /// * `options` - Broadcast-Optionen (optional)
    ///
    /// # Rückgabe
    ///
    /// Die Anzahl der Connections, die die Nachricht erhalten haben
    pub async fn broadcast(&self, message: WebSocketMessage, options: Option<BroadcastOptions>) -> usize {
        let options = options.unwrap_or_default();
        let mut sent_count = 0;

        // Wenn ein Channel angegeben ist, nur an Channel-Subscriber senden
        let target_ids = if let Some(channel) = &options.channel {
            self.channel_manager.get_subscribers(channel).await
        } else {
            let connections = self.connections.read().await;
            connections.keys().copied().collect()
        };

        let connections = self.connections.read().await;

        for id in target_ids {
            if !options.should_include(&id) {
                continue;
            }

            if let Some(sender) = connections.get(&id) {
                if let Err(e) = sender.send(message.clone()) {
                    error!(connection_id = %id, error = %e, "Failed to send broadcast message");
                } else {
                    sent_count += 1;
                }
            }
        }

        debug!(count = sent_count, "Broadcast message sent");
        sent_count
    }

    /// Sendet eine Nachricht an alle Subscriber eines Channels
    ///
    /// # Argumente
    ///
    /// * `channel` - Der Channel-Name
    /// * `message` - Die zu sendende Nachricht
    ///
    /// # Rückgabe
    ///
    /// Die Anzahl der Connections, die die Nachricht erhalten haben
    pub async fn send_to_channel(&self, channel: &str, message: WebSocketMessage) -> usize {
        let options = BroadcastOptions::new().to_channel(channel);
        self.broadcast(message, Some(options)).await
    }

    /// Gibt die Anzahl der aktiven Verbindungen zurück
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Gibt eine Liste aller Connection-IDs zurück
    pub async fn get_connection_ids(&self) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        connections.keys().copied().collect()
    }

    /// Gibt die Metadaten einer Connection zurück
    pub async fn get_connection_metadata(&self, connection_id: ConnectionId) -> Option<ConnectionMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(&connection_id).cloned()
    }

    /// Gibt den Channel-Manager zurück
    pub fn channel_manager(&self) -> Arc<ChannelManager> {
        self.channel_manager.clone()
    }

    /// Sendet eine Ping-Nachricht an alle aktiven Verbindungen
    pub async fn ping_all(&self) -> usize {
        let ping_msg = WebSocketMessage::ping();
        self.broadcast(ping_msg, None).await
    }

    /// Schließt eine spezifische Verbindung
    pub async fn close_connection(&self, connection_id: ConnectionId) {
        self.unregister_connection(connection_id).await;
    }

    /// Schließt alle Verbindungen
    pub async fn close_all(&self) {
        let ids = self.get_connection_ids().await;
        for id in ids {
            self.close_connection(id).await;
        }
        info!("All WebSocket connections closed");
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = WebSocketManager::new();
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_options() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();

        let options = BroadcastOptions::new().exclude(vec![id1]);
        assert!(!options.should_include(&id1));
        assert!(options.should_include(&id2));

        let options = BroadcastOptions::new().only_to(vec![id1]);
        assert!(options.should_include(&id1));
        assert!(!options.should_include(&id2));
    }
}
