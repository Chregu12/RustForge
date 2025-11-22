//! WebSocket server for real-time client connections

use crate::{BroadcastError, BroadcastResult, ClientMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

/// Channel registry that manages WebSocket broadcast channels
#[derive(Clone)]
pub struct ChannelRegistry {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl ChannelRegistry {
    /// Create a new channel registry
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a channel
    pub async fn get_or_create(&self, channel: &str) -> broadcast::Sender<String> {
        let mut channels = self.channels.write().await;

        channels
            .entry(channel.to_string())
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(1000);
                tracing::debug!(channel = %channel, "Created new broadcast channel");
                tx
            })
            .clone()
    }

    /// Broadcast a message to a channel
    pub async fn broadcast(&self, channel: &str, message: String) -> BroadcastResult<()> {
        if let Some(tx) = self.channels.read().await.get(channel) {
            tx.send(message)
                .map_err(|e| BroadcastError::WebSocket(e.to_string()))?;
        }
        Ok(())
    }

    /// Remove a channel if it has no subscribers
    pub async fn cleanup(&self, channel: &str) {
        let mut channels = self.channels.write().await;
        if let Some(tx) = channels.get(channel) {
            if tx.receiver_count() == 0 {
                channels.remove(channel);
                tracing::debug!(channel = %channel, "Removed unused channel");
            }
        }
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket server configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Port to listen on
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Ping interval in seconds
    pub ping_interval: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            port: 6001,
            host: "0.0.0.0".to_string(),
            max_message_size: 1024 * 1024, // 1MB
            ping_interval: 30,
        }
    }
}

/// WebSocket server for real-time communication
pub struct WebSocketServer {
    config: WebSocketConfig,
    registry: ChannelRegistry,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            registry: ChannelRegistry::new(),
        }
    }

    /// Get the channel registry (useful for Redis subscriber)
    pub fn registry(&self) -> &ChannelRegistry {
        &self.registry
    }

    /// Start the WebSocket server
    pub async fn start(self) -> BroadcastResult<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!(addr = %addr, "WebSocket server started");

        let server = Arc::new(self);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr).await {
                            tracing::error!(error = %e, addr = %addr, "Connection error");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to accept connection");
                }
            }
        }
    }

    /// Handle a WebSocket connection
    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> BroadcastResult<()> {
        tracing::info!(addr = %addr, "New WebSocket connection");

        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| BroadcastError::WebSocket(e.to_string()))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let mut subscriptions: HashMap<String, broadcast::Receiver<String>> = HashMap::new();

        // Channel for sending messages to this client
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        // Spawn task to send messages to client
        let send_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = ws_sender.send(Message::Text(message.into())).await {
                    tracing::error!(error = %e, "Failed to send message");
                    break;
                }
            }
        });

        // Handle incoming messages from client
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = self
                        .handle_message(&text, &tx, &mut subscriptions, addr)
                        .await
                    {
                        tracing::error!(error = %e, "Failed to handle message");
                        let error_msg = ServerMessage::Error {
                            message: e.to_string(),
                        };
                        if let Ok(json) = error_msg.to_json() {
                            let _ = tx.send(json);
                        }
                    }
                }
                Ok(Message::Ping(_)) => {
                    let pong = ServerMessage::Pong;
                    if let Ok(json) = pong.to_json() {
                        let _ = tx.send(json);
                    }
                }
                Ok(Message::Close(_)) => {
                    tracing::info!(addr = %addr, "Client disconnected");
                    break;
                }
                Err(e) => {
                    tracing::error!(error = %e, addr = %addr, "WebSocket error");
                    break;
                }
                _ => {}
            }
        }

        // Cleanup
        send_task.abort();
        for channel in subscriptions.keys() {
            self.registry.cleanup(channel).await;
        }

        tracing::info!(addr = %addr, "Connection closed");
        Ok(())
    }

    /// Handle a client message
    async fn handle_message(
        &self,
        text: &str,
        tx: &tokio::sync::mpsc::UnboundedSender<String>,
        subscriptions: &mut HashMap<String, broadcast::Receiver<String>>,
        addr: SocketAddr,
    ) -> BroadcastResult<()> {
        let message: ClientMessage = serde_json::from_str(text)?;

        match message {
            ClientMessage::Subscribe { channel, auth } => {
                // TODO: Implement channel authorization with auth token
                if auth.is_some() {
                    tracing::debug!(channel = %channel, "Authorizing channel subscription");
                }

                let broadcast_tx = self.registry.get_or_create(&channel).await;
                let broadcast_rx = broadcast_tx.subscribe();

                // Send confirmation
                let response = ServerMessage::Subscribed {
                    channel: channel.clone(),
                };
                tx.send(response.to_json()?)
                    .map_err(|e| BroadcastError::WebSocket(e.to_string()))?;

                tracing::info!(channel = %channel, addr = %addr, "Client subscribed");

                // Spawn task to forward broadcasts to this client
                let tx_clone = tx.clone();
                let channel_clone = channel.clone();
                let mut rx_clone = broadcast_rx.resubscribe();
                tokio::spawn(async move {
                    while let Ok(message) = rx_clone.recv().await {
                        if tx_clone.send(message).is_err() {
                            break;
                        }
                    }
                    tracing::debug!(channel = %channel_clone, "Subscription task ended");
                });

                subscriptions.insert(channel, broadcast_rx);
            }
            ClientMessage::Unsubscribe { channel } => {
                subscriptions.remove(&channel);

                let response = ServerMessage::Unsubscribed {
                    channel: channel.clone(),
                };
                tx.send(response.to_json()?)
                    .map_err(|e| BroadcastError::WebSocket(e.to_string()))?;

                self.registry.cleanup(&channel).await;

                tracing::info!(channel = %channel, addr = %addr, "Client unsubscribed");
            }
            ClientMessage::Ping => {
                let response = ServerMessage::Pong;
                tx.send(response.to_json()?)
                    .map_err(|e| BroadcastError::WebSocket(e.to_string()))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_registry() {
        let registry = ChannelRegistry::new();

        let tx1 = registry.get_or_create("test").await;
        let tx2 = registry.get_or_create("test").await;

        // Should be the same channel
        assert_eq!(tx1.receiver_count(), tx2.receiver_count());
    }

    #[tokio::test]
    async fn test_channel_broadcast() {
        let registry = ChannelRegistry::new();

        let tx = registry.get_or_create("test").await;
        let mut rx = tx.subscribe();

        registry
            .broadcast("test", "Hello".to_string())
            .await
            .unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received, "Hello");
    }

    #[tokio::test]
    async fn test_channel_cleanup() {
        let registry = ChannelRegistry::new();

        let tx = registry.get_or_create("test").await;
        drop(tx);

        registry.cleanup("test").await;

        let channels = registry.channels.read().await;
        assert!(channels.is_empty());
    }
}
