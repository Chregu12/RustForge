//! Chat Example - Real-Time Chat System
//!
//! Ein vollständiges Beispiel für einen Echtzeit-Chat mit WebSockets.

use crate::websocket::{
    manager::WebSocketManager,
    channel::Channel,
    message::WebSocketMessage,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Eine Chat-Nachricht
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Benutzername
    pub username: String,
    /// Nachrichteninhalt
    pub message: String,
    /// Zeitstempel
    pub timestamp: i64,
    /// Optional: Raum/Channel
    pub room: Option<String>,
}

impl ChatMessage {
    /// Erstellt eine neue Chat-Nachricht
    pub fn new(username: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            message: message.into(),
            timestamp: chrono::Utc::now().timestamp(),
            room: None,
        }
    }

    /// Setzt den Raum
    pub fn in_room(mut self, room: impl Into<String>) -> Self {
        self.room = Some(room.into());
        self
    }

    /// Konvertiert zu WebSocket-Nachricht
    pub fn to_websocket_message(&self) -> serde_json::Result<WebSocketMessage> {
        WebSocketMessage::json(self)
    }
}

/// Chat-Service für Real-Time Messaging
pub struct ChatService {
    manager: WebSocketManager,
}

impl ChatService {
    /// Erstellt einen neuen Chat-Service
    pub fn new(manager: WebSocketManager) -> Self {
        Self { manager }
    }

    /// Erstellt einen Chat-Raum
    ///
    /// # Beispiel
    ///
    /// ```no_run
    /// use foundry_api::websocket::examples::chat::ChatService;
    /// use foundry_api::websocket::WebSocketManager;
    ///
    /// # async fn example() {
    /// let manager = WebSocketManager::new();
    /// let chat = ChatService::new(manager);
    /// chat.create_room("general", "General Discussion").await;
    /// # }
    /// ```
    pub async fn create_room(&self, room_name: &str, description: &str) {
        let channel = Channel::new(format!("chat:{}", room_name))
            .with_description(description);

        self.manager.channel_manager().create_channel(channel).await;
    }

    /// Sendet eine Chat-Nachricht an einen Raum
    ///
    /// # Beispiel
    ///
    /// ```no_run
    /// use foundry_api::websocket::examples::chat::{ChatService, ChatMessage};
    /// use foundry_api::websocket::WebSocketManager;
    ///
    /// # async fn example() {
    /// let manager = WebSocketManager::new();
    /// let chat = ChatService::new(manager);
    ///
    /// let msg = ChatMessage::new("Alice", "Hello everyone!");
    /// chat.send_message("general", msg).await;
    /// # }
    /// ```
    pub async fn send_message(&self, room: &str, message: ChatMessage) -> anyhow::Result<usize> {
        let channel_name = format!("chat:{}", room);
        let ws_message = message.to_websocket_message()?;

        let count = self.manager.send_to_channel(&channel_name, ws_message).await;

        Ok(count)
    }

    /// Sendet eine System-Nachricht an einen Raum
    pub async fn send_system_message(&self, room: &str, message: &str) -> usize {
        let channel_name = format!("chat:{}", room);
        let ws_message = WebSocketMessage::system(message);

        self.manager.send_to_channel(&channel_name, ws_message).await
    }

    /// Benachrichtigt über einen Benutzer-Beitritt
    pub async fn notify_user_joined(&self, room: &str, username: &str) -> usize {
        let message = format!("{} has joined the room", username);
        self.send_system_message(room, &message).await
    }

    /// Benachrichtigt über einen Benutzer-Austritt
    pub async fn notify_user_left(&self, room: &str, username: &str) -> usize {
        let message = format!("{} has left the room", username);
        self.send_system_message(room, &message).await
    }

    /// Gibt die Anzahl der Benutzer in einem Raum zurück
    pub async fn room_user_count(&self, room: &str) -> usize {
        let channel_name = format!("chat:{}", room);
        self.manager.channel_manager().subscriber_count(&channel_name).await
    }

    /// Liste aller Chat-Räume
    pub async fn list_rooms(&self) -> Vec<String> {
        let channels = self.manager.channel_manager().list_channels().await;
        channels
            .into_iter()
            .filter(|c| c.starts_with("chat:"))
            .map(|c| c.strip_prefix("chat:").unwrap().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_message_creation() {
        let msg = ChatMessage::new("Alice", "Hello");
        assert_eq!(msg.username, "Alice");
        assert_eq!(msg.message, "Hello");
    }

    #[tokio::test]
    async fn test_chat_service_room_creation() {
        let manager = WebSocketManager::new();
        let chat = ChatService::new(manager.clone());

        chat.create_room("test", "Test Room").await;

        let rooms = chat.list_rooms().await;
        assert!(rooms.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_chat_message_serialization() {
        let msg = ChatMessage::new("Bob", "Hi there");
        let ws_msg = msg.to_websocket_message().unwrap();
        assert!(ws_msg.payload.is_some());
    }
}
