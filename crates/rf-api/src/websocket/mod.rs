//! WebSocket Real-Time Communication Module
//!
//! Dieses Modul bietet vollständige WebSocket-Unterstützung für RustForge,
//! einschließlich Connection-Management, Broadcasting, Channels und mehr.
//!
//! # Features
//!
//! - Connection Management mit automatischem Cleanup
//! - Broadcasting für Echtzeit-Updates
//! - Channel-basierte Kommunikation
//! - Heartbeat/Ping-Pong für Connection-Health
//! - Message-Serialisierung mit JSON
//! - Integration mit Axum Router
//!
//! # Beispiel
//!
//! ```no_run
//! use rf_api::websocket::{WebSocketManager, WebSocketMessage};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = WebSocketManager::new();
//!
//! // Broadcast an alle verbundenen Clients
//! manager.broadcast(WebSocketMessage::text("Hello everyone!"), None).await;
//!
//! // An spezifischen Channel senden
//! let data = json!({ "user": "Alice", "message": "Hi!" });
//! manager.send_to_channel("chat:room1", WebSocketMessage::json(&data)?).await;
//! # Ok(())
//! # }
//! ```

pub mod channel;
pub mod connection;
pub mod examples;
pub mod handler;
pub mod manager;
pub mod message;

pub use channel::{Channel, ChannelManager};
pub use connection::{Connection, ConnectionId};
pub use handler::{upgrade_websocket, websocket_handler};
pub use manager::{BroadcastOptions, WebSocketManager};
pub use message::{MessageType, WebSocketMessage};

use crate::AppState;
use axum::Router;

/// Erstellt einen Router mit WebSocket-Routen
///
/// # Beispiel
///
/// ```no_run
/// use rf_api::websocket;
/// use axum::Router;
///
/// let ws_router = websocket::websocket_routes();
/// let app = Router::new().merge(ws_router);
/// ```
pub fn websocket_routes() -> Router<AppState> {
    use axum::routing::get;

    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/ws/:channel", get(handler::websocket_channel_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let manager = WebSocketManager::new();
        assert_eq!(manager.connection_count().await, 0);
    }
}
