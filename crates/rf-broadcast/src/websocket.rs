//! WebSocket integration for broadcasting

use crate::{Broadcaster, Channel, MemoryBroadcaster};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// WebSocket state
#[derive(Clone)]
pub struct WsState {
    pub broadcaster: Arc<MemoryBroadcaster>,
}

/// WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "subscribe")]
    Subscribe { channel: String },

    #[serde(rename = "unsubscribe")]
    Unsubscribe { channel: String },

    #[serde(rename = "event")]
    Event {
        channel: String,
        event: String,
        data: serde_json::Value,
    },

    #[serde(rename = "subscribed")]
    Subscribed { channel: String },

    #[serde(rename = "unsubscribed")]
    Unsubscribed { channel: String },

    #[serde(rename = "error")]
    Error { message: String },
}

/// WebSocket handler
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: WsState) {
    let connection_id = uuid::Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast events
    let mut event_rx = state.broadcaster.subscribe_to_events();

    // Clone connection_id for tasks
    let connection_id_clone = connection_id.clone();
    let connection_id_clone2 = connection_id.clone();

    // Track subscribed channels for this connection
    let subscribed_channels = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let subscribed_channels_clone = subscribed_channels.clone();

    // Spawn task to forward broadcast events to WebSocket
    let broadcaster_clone = state.broadcaster.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = event_rx.recv().await {
            // Only send if this connection is subscribed to the channel
            if msg.connections.contains(&connection_id_clone) {
                let ws_msg = WsMessage::Event {
                    channel: msg.channel.name().to_string(),
                    event: msg.event_name,
                    data: serde_json::from_str(&msg.data).unwrap_or_default(),
                };

                if let Ok(json) = serde_json::to_string(&ws_msg) {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages
    let broadcaster = state.broadcaster.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::Subscribe { channel } => {
                            let ch = Channel::public(channel.clone());

                            // Subscribe
                            if broadcaster
                                .subscribe(&ch, connection_id_clone2.clone(), None)
                                .await
                                .is_ok()
                            {
                                // Track subscription
                                subscribed_channels_clone.lock().await.push(ch);

                                tracing::info!(
                                    connection_id = %connection_id_clone2,
                                    channel = %channel,
                                    "WebSocket subscribed to channel"
                                );
                            }
                        }
                        WsMessage::Unsubscribe { channel } => {
                            let ch = Channel::public(channel.clone());

                            let _ = broadcaster
                                .unsubscribe(&ch, &connection_id_clone2)
                                .await;

                            // Remove from tracked subscriptions
                            subscribed_channels_clone.lock().await.retain(|c| c != &ch);

                            tracing::info!(
                                connection_id = %connection_id_clone2,
                                channel = %channel,
                                "WebSocket unsubscribed from channel"
                            );
                        }
                        _ => {}
                    }
                }
            } else if let Message::Close(_) = msg {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    // Cleanup: unsubscribe from all channels
    let channels = subscribed_channels.lock().await;
    for channel in channels.iter() {
        let _ = broadcaster_clone.unsubscribe(channel, &connection_id).await;
    }

    tracing::info!(
        connection_id = %connection_id,
        "WebSocket connection closed"
    );
}

/// Create WebSocket router
///
/// # Example
///
/// ```no_run
/// use rf_broadcast::{MemoryBroadcaster, websocket_router};
/// use std::sync::Arc;
///
/// # async fn example() {
/// let broadcaster = Arc::new(MemoryBroadcaster::new());
/// let router = websocket_router(broadcaster);
///
/// // Merge with your app router
/// // let app = Router::new().merge(router);
/// # }
/// ```
pub fn websocket_router(broadcaster: Arc<MemoryBroadcaster>) -> Router {
    let state = WsState { broadcaster };

    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_serialization() {
        let msg = WsMessage::Subscribe {
            channel: "test".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_ws_message_deserialization() {
        let json = r#"{"type":"subscribe","channel":"test"}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();

        match msg {
            WsMessage::Subscribe { channel } => assert_eq!(channel, "test"),
            _ => panic!("Wrong message type"),
        }
    }
}
