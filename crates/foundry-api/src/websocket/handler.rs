//! WebSocket Handler
//!
//! Axum Handler für WebSocket-Verbindungen und Upgrades.

use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::AppState;

use super::{
    connection::{Connection, ConnectionMetadata},
    manager::WebSocketManager,
    message::{MessageType, WebSocketMessage},
};

lazy_static::lazy_static! {
    /// Globaler WebSocket Manager (Singleton)
    static ref WS_MANAGER: Arc<RwLock<Option<WebSocketManager>>> = Arc::new(RwLock::new(None));
}

/// Initialisiert den globalen WebSocket Manager
pub async fn init_websocket_manager() -> WebSocketManager {
    let mut manager_guard = WS_MANAGER.write().await;
    if manager_guard.is_none() {
        *manager_guard = Some(WebSocketManager::new());
    }
    manager_guard.as_ref().unwrap().clone()
}

/// Gibt den globalen WebSocket Manager zurück
pub async fn get_websocket_manager() -> Option<WebSocketManager> {
    let manager_guard = WS_MANAGER.read().await;
    manager_guard.clone()
}

/// WebSocket Upgrade Handler
///
/// Dieser Handler upgraded eine HTTP-Verbindung zu WebSocket.
///
/// # Beispiel
///
/// ```no_run
/// use axum::{Router, routing::get};
/// use foundry_api::websocket::websocket_handler;
///
/// let app = Router::new().route("/ws", get(websocket_handler));
/// ```
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, None))
}

/// WebSocket Handler mit Channel-Support
///
/// Dieser Handler upgraded eine HTTP-Verbindung zu WebSocket und
/// abonniert automatisch einen Channel.
///
/// # Route
///
/// ```no_run
/// use axum::{Router, routing::get};
/// use foundry_api::websocket::handler::websocket_channel_handler;
///
/// let app = Router::new().route("/ws/:channel", get(websocket_channel_handler));
/// ```
pub async fn websocket_channel_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
    Path(channel): Path<String>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, Some(channel)))
}

/// Behandelt eine WebSocket-Verbindung
async fn handle_websocket(socket: WebSocket, auto_subscribe_channel: Option<String>) {
    let manager = match get_websocket_manager().await {
        Some(m) => m,
        None => {
            warn!("WebSocket manager not initialized, initializing now");
            init_websocket_manager().await
        }
    };

    let metadata = ConnectionMetadata::new();
    let (connection, sender) = Connection::new(socket, metadata);
    let connection_id = connection.id;

    // Connection registrieren
    manager.register_connection(&connection, sender).await;

    // Automatisch Channel abonnieren, falls angegeben
    if let Some(channel) = &auto_subscribe_channel {
        manager.channel_manager().subscribe(channel, connection_id).await;

        // Welcome-Nachricht an den Client senden
        let welcome_msg = WebSocketMessage::system(format!(
            "Connected to channel: {}",
            channel
        ));
        if let Err(e) = connection.send(welcome_msg).await {
            error!(connection_id = %connection_id, error = %e, "Failed to send welcome message");
        }
    } else {
        // Normale Welcome-Nachricht
        let welcome_msg = WebSocketMessage::system("WebSocket connected");
        if let Err(e) = connection.send(welcome_msg).await {
            error!(connection_id = %connection_id, error = %e, "Failed to send welcome message");
        }
    }

    info!(
        connection_id = %connection_id,
        channel = ?auto_subscribe_channel,
        "WebSocket connection established"
    );

    // Connection-Loop
    handle_connection(connection, manager.clone()).await;

    // Cleanup nach Verbindungsende
    manager.unregister_connection(connection_id).await;

    info!(connection_id = %connection_id, "WebSocket connection closed");
}

/// Hauptloop für eine WebSocket-Verbindung
async fn handle_connection(mut connection: Connection, manager: WebSocketManager) {
    let connection_id = connection.id;

    loop {
        tokio::select! {
            // Nachricht vom Client empfangen
            msg = connection.recv() => {
                match msg {
                    Some(message) => {
                        if let Err(e) = handle_client_message(message, connection_id, &manager, &connection).await {
                            error!(
                                connection_id = %connection_id,
                                error = %e,
                                "Error handling client message"
                            );
                        }
                    }
                    None => {
                        // Client hat Verbindung geschlossen
                        break;
                    }
                }
            }
        }
    }
}

/// Behandelt eine Nachricht vom Client
async fn handle_client_message(
    message: WebSocketMessage,
    connection_id: super::connection::ConnectionId,
    manager: &WebSocketManager,
    connection: &Connection,
) -> anyhow::Result<()> {
    match message.msg_type {
        MessageType::Ping => {
            // Auf Ping mit Pong antworten
            connection.send(WebSocketMessage::pong()).await?;
        }
        MessageType::Text | MessageType::Json => {
            // Echo zurück an den Client (kann angepasst werden)
            info!(
                connection_id = %connection_id,
                "Received message from client"
            );

            // Beispiel: Echo-Nachricht
            let echo = WebSocketMessage::system("Message received");
            connection.send(echo).await?;
        }
        MessageType::System => {
            // System-Commands vom Client verarbeiten
            if let Some(payload) = &message.payload {
                if let Some(command) = payload.get("command").and_then(|c| c.as_str()) {
                    handle_system_command(command, payload, connection_id, manager, connection).await?;
                }
            }
        }
        _ => {
            // Andere Nachrichtentypen ignorieren oder loggen
        }
    }

    Ok(())
}

/// Behandelt System-Commands vom Client
async fn handle_system_command(
    command: &str,
    payload: &serde_json::Value,
    connection_id: super::connection::ConnectionId,
    manager: &WebSocketManager,
    connection: &Connection,
) -> anyhow::Result<()> {
    match command {
        "subscribe" => {
            // Client möchte einen Channel abonnieren
            if let Some(channel) = payload.get("channel").and_then(|c| c.as_str()) {
                manager.channel_manager().subscribe(channel, connection_id).await;

                let response = WebSocketMessage::system(format!("Subscribed to channel: {}", channel));
                connection.send(response).await?;

                info!(
                    connection_id = %connection_id,
                    channel = %channel,
                    "Client subscribed to channel"
                );
            }
        }
        "unsubscribe" => {
            // Client möchte einen Channel deabonnieren
            if let Some(channel) = payload.get("channel").and_then(|c| c.as_str()) {
                manager.channel_manager().unsubscribe(channel, connection_id).await;

                let response = WebSocketMessage::system(format!("Unsubscribed from channel: {}", channel));
                connection.send(response).await?;

                info!(
                    connection_id = %connection_id,
                    channel = %channel,
                    "Client unsubscribed from channel"
                );
            }
        }
        "list_channels" => {
            // Client fragt nach allen verfügbaren Channels
            let channels = manager.channel_manager().list_channels().await;
            let response = WebSocketMessage::json(&serde_json::json!({
                "channels": channels
            }))?;
            connection.send(response).await?;
        }
        "ping" => {
            // Manueller Ping vom Client
            connection.send(WebSocketMessage::pong()).await?;
        }
        _ => {
            warn!(
                connection_id = %connection_id,
                command = %command,
                "Unknown system command"
            );
        }
    }

    Ok(())
}

/// Upgrade-Funktion für benutzerdefinierte WebSocket-Handler
///
/// # Beispiel
///
/// ```no_run
/// use axum::extract::ws::WebSocketUpgrade;
/// use foundry_api::websocket::upgrade_websocket;
///
/// async fn custom_handler(ws: WebSocketUpgrade) {
///     upgrade_websocket(ws, |socket| async {
///         // Custom logic hier
///     }).await;
/// }
/// ```
pub async fn upgrade_websocket<F, Fut>(ws: WebSocketUpgrade, handler: F) -> Response
where
    F: FnOnce(WebSocket) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    ws.on_upgrade(handler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_websocket_manager() {
        let manager = init_websocket_manager().await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_websocket_manager() {
        init_websocket_manager().await;
        let manager = get_websocket_manager().await;
        assert!(manager.is_some());
    }
}
