//! WebSocket Connection Management
//!
//! Verwaltet einzelne WebSocket-Verbindungen mit automatischem Cleanup.

use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::message::WebSocketMessage;

/// Eine eindeutige Connection-ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    /// Erstellt eine neue zufällige Connection-ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Erstellt eine Connection-ID aus einer UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Gibt die UUID zurück
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Metadaten über eine WebSocket-Verbindung
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// Zeitpunkt der Verbindung
    pub connected_at: i64,
    /// Remote IP-Adresse (optional)
    pub remote_addr: Option<String>,
    /// User-Agent (optional)
    pub user_agent: Option<String>,
    /// Benutzerdefinierte Metadaten
    pub custom: Option<serde_json::Value>,
}

impl ConnectionMetadata {
    /// Erstellt neue Metadaten
    pub fn new() -> Self {
        Self {
            connected_at: chrono::Utc::now().timestamp(),
            remote_addr: None,
            user_agent: None,
            custom: None,
        }
    }

    /// Setzt die Remote-Adresse
    pub fn with_remote_addr(mut self, addr: String) -> Self {
        self.remote_addr = Some(addr);
        self
    }

    /// Setzt den User-Agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Setzt benutzerdefinierte Metadaten
    pub fn with_custom(mut self, custom: serde_json::Value) -> Self {
        self.custom = Some(custom);
        self
    }
}

impl Default for ConnectionMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Eine WebSocket-Verbindung
pub struct Connection {
    /// Die eindeutige ID dieser Verbindung
    pub id: ConnectionId,

    /// Metadaten über die Verbindung
    pub metadata: ConnectionMetadata,

    /// Sender für Nachrichten an den Client
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,

    /// Channel zum Empfangen von Nachrichten
    rx: mpsc::UnboundedReceiver<WebSocketMessage>,
}

impl Connection {
    /// Erstellt eine neue Connection aus einem WebSocket
    ///
    /// # Argumente
    ///
    /// * `socket` - Der WebSocket
    /// * `metadata` - Metadaten über die Verbindung
    ///
    /// # Rückgabe
    ///
    /// Ein Tuple aus (Connection, Sender für Messages)
    pub fn new(
        socket: WebSocket,
        metadata: ConnectionMetadata,
    ) -> (Self, mpsc::UnboundedSender<WebSocketMessage>) {
        let (sender, mut receiver) = socket.split();
        let (tx, rx) = mpsc::unbounded_channel();
        let id = ConnectionId::new();

        let connection = Self {
            id,
            metadata,
            sender: Arc::new(Mutex::new(sender)),
            rx,
        };

        // Spawn einen Task zum Empfangen von Client-Nachrichten
        let tx_clone = tx.clone();
        let conn_id = id;
        tokio::spawn(async move {
            while let Some(result) = receiver.next().await {
                match result {
                    Ok(msg) => {
                        if let Err(e) = Self::handle_incoming_message(msg, &tx_clone, conn_id).await
                        {
                            warn!(connection_id = %conn_id, error = %e, "Error handling incoming message");
                        }
                    }
                    Err(e) => {
                        error!(connection_id = %conn_id, error = %e, "WebSocket error");
                        break;
                    }
                }
            }
            debug!(connection_id = %conn_id, "Client receiver task ended");
        });

        (connection, tx)
    }

    /// Behandelt eingehende Nachrichten vom Client
    async fn handle_incoming_message(
        msg: Message,
        tx: &mpsc::UnboundedSender<WebSocketMessage>,
        conn_id: ConnectionId,
    ) -> anyhow::Result<()> {
        match msg {
            Message::Text(text) => {
                debug!(connection_id = %conn_id, "Received text message");
                let text_str = text.to_string();
                if let Ok(ws_msg) = WebSocketMessage::from_json_string(&text_str) {
                    let _ = tx.send(ws_msg);
                } else {
                    // Fallback: Als einfache Text-Nachricht behandeln
                    let _ = tx.send(WebSocketMessage::text(text_str));
                }
            }
            Message::Binary(data) => {
                debug!(connection_id = %conn_id, bytes = data.len(), "Received binary message");
                // Binary-Messages können später unterstützt werden
            }
            Message::Ping(data) => {
                debug!(connection_id = %conn_id, "Received ping");
                let _ = tx.send(WebSocketMessage::ping());
            }
            Message::Pong(_) => {
                debug!(connection_id = %conn_id, "Received pong");
            }
            Message::Close(frame) => {
                info!(connection_id = %conn_id, ?frame, "Received close frame");
            }
        }
        Ok(())
    }

    /// Sendet eine Nachricht an den Client
    pub async fn send(&self, msg: WebSocketMessage) -> anyhow::Result<()> {
        let json = msg.to_json_string()?;
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(json.into())).await?;
        Ok(())
    }

    /// Empfängt die nächste Nachricht vom Client (blocking)
    pub async fn recv(&mut self) -> Option<WebSocketMessage> {
        self.rx.recv().await
    }

    /// Versucht, eine Nachricht zu empfangen (non-blocking)
    pub fn try_recv(&mut self) -> Option<WebSocketMessage> {
        self.rx.try_recv().ok()
    }

    /// Schließt die Verbindung
    pub async fn close(self) -> anyhow::Result<()> {
        let mut sender = self.sender.lock().await;
        sender.close().await?;
        info!(connection_id = %self.id, "Connection closed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id_creation() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_connection_id_display() {
        let id = ConnectionId::new();
        let display = format!("{}", id);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = ConnectionMetadata::new()
            .with_remote_addr("127.0.0.1".to_string())
            .with_user_agent("TestAgent/1.0".to_string());

        assert_eq!(metadata.remote_addr, Some("127.0.0.1".to_string()));
        assert_eq!(metadata.user_agent, Some("TestAgent/1.0".to_string()));
    }
}
