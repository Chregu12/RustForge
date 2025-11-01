//! WebSocket Message Types
//!
//! Dieses Modul definiert die verschiedenen Message-Typen und deren Serialisierung.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Der Typ einer WebSocket-Nachricht
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    /// Text-Nachricht
    Text,
    /// JSON-Nachricht
    Json,
    /// Binär-Daten
    Binary,
    /// Ping (Heartbeat)
    Ping,
    /// Pong (Heartbeat-Antwort)
    Pong,
    /// System-Nachricht
    System,
    /// Event-Nachricht
    Event,
}

/// Eine WebSocket-Nachricht
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    /// Der Typ der Nachricht
    #[serde(rename = "type")]
    pub msg_type: MessageType,

    /// Der Payload der Nachricht
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,

    /// Zeitstempel der Nachricht (Unix Timestamp)
    pub timestamp: i64,

    /// Optionale Metadaten
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl WebSocketMessage {
    /// Erstellt eine neue Text-Nachricht
    ///
    /// # Beispiel
    ///
    /// ```
    /// use foundry_api::websocket::WebSocketMessage;
    ///
    /// let msg = WebSocketMessage::text("Hello World");
    /// ```
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::Text,
            payload: Some(Value::String(content.into())),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// Erstellt eine neue JSON-Nachricht
    ///
    /// # Beispiel
    ///
    /// ```
    /// use foundry_api::websocket::WebSocketMessage;
    /// use serde_json::json;
    ///
    /// let data = json!({"user": "Alice", "message": "Hi!"});
    /// let msg = WebSocketMessage::json(&data).unwrap();
    /// ```
    pub fn json<T: Serialize>(data: &T) -> serde_json::Result<Self> {
        Ok(Self {
            msg_type: MessageType::Json,
            payload: Some(serde_json::to_value(data)?),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        })
    }

    /// Erstellt eine System-Nachricht
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            msg_type: MessageType::System,
            payload: Some(Value::String(content.into())),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// Erstellt eine Event-Nachricht
    ///
    /// # Beispiel
    ///
    /// ```
    /// use foundry_api::websocket::WebSocketMessage;
    /// use serde_json::json;
    ///
    /// let event_data = json!({"event": "user.joined", "userId": 42});
    /// let msg = WebSocketMessage::event("user.joined", event_data);
    /// ```
    pub fn event(event_name: impl Into<String>, data: Value) -> Self {
        Self {
            msg_type: MessageType::Event,
            payload: Some(serde_json::json!({
                "event": event_name.into(),
                "data": data
            })),
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// Erstellt eine Ping-Nachricht
    pub fn ping() -> Self {
        Self {
            msg_type: MessageType::Ping,
            payload: None,
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// Erstellt eine Pong-Nachricht
    pub fn pong() -> Self {
        Self {
            msg_type: MessageType::Pong,
            payload: None,
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
        }
    }

    /// Fügt Metadaten zur Nachricht hinzu
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Konvertiert die Nachricht zu JSON-String
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Erstellt eine Nachricht aus einem JSON-String
    pub fn from_json_string(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_text_message() {
        let msg = WebSocketMessage::text("Hello");
        assert_eq!(msg.msg_type, MessageType::Text);
        assert_eq!(msg.payload, Some(Value::String("Hello".to_string())));
    }

    #[test]
    fn test_json_message() {
        let data = json!({"key": "value"});
        let msg = WebSocketMessage::json(&data).unwrap();
        assert_eq!(msg.msg_type, MessageType::Json);
    }

    #[test]
    fn test_event_message() {
        let data = json!({"userId": 42});
        let msg = WebSocketMessage::event("user.joined", data);
        assert_eq!(msg.msg_type, MessageType::Event);
    }

    #[test]
    fn test_serialization() {
        let msg = WebSocketMessage::text("Test");
        let json_str = msg.to_json_string().unwrap();
        let deserialized = WebSocketMessage::from_json_string(&json_str).unwrap();
        assert_eq!(deserialized.msg_type, msg.msg_type);
    }
}
