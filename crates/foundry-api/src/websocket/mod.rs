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
//! use foundry_api::websocket::{WebSocketManager, WebSocketMessage};
//!
//! let manager = WebSocketManager::new();
//!
//! // Broadcast an alle verbundenen Clients
//! manager.broadcast("global", WebSocketMessage::text("Hello everyone!")).await;
//!
//! // An spezifischen Channel senden
//! manager.send_to_channel("chat:room1", WebSocketMessage::json(&data)?).await;
//! ```

pub mod connection;
pub mod handler;
pub mod manager;
pub mod message;
pub mod channel;
pub mod examples;

pub use connection::{Connection, ConnectionId};
pub use handler::{websocket_handler, upgrade_websocket};
pub use manager::{WebSocketManager, BroadcastOptions};
pub use message::{WebSocketMessage, MessageType};
pub use channel::{Channel, ChannelManager};

use axum::Router;
use crate::AppState;

/// Erstellt einen Router mit WebSocket-Routen
///
/// # Beispiel
///
/// ```no_run
/// use foundry_api::websocket;
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
