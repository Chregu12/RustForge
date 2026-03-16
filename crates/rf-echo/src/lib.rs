//! Real-time Broadcasting Client for RustForge
//!
//! This crate provides Laravel Echo-like functionality for Rust applications.
//! It enables real-time event broadcasting via WebSockets with support for
//! Pusher, Ably, and custom WebSocket servers.
//!
//! # Features
//!
//! - **WebSocket Client**: Connect to broadcasting servers
//! - **Channel Subscriptions**: Public, private, and presence channels
//! - **Event Listening**: Subscribe to specific events on channels
//! - **Presence Channels**: Track who is online in a channel
//! - **Pusher Protocol**: Compatible with Pusher's broadcasting protocol
//! - **Authentication**: Automatic private/presence channel authentication
//!
//! # Quick Start
//!
//! ```ignore
//! use rf_echo::{Echo, EchoConfig, Connector};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rf_echo::EchoError> {
//!     let echo = Echo::new(EchoConfig {
//!         broadcaster: Connector::Pusher {
//!             key: "your-pusher-key".to_string(),
//!             cluster: "mt1".to_string(),
//!         },
//!         auth_endpoint: Some("/broadcasting/auth".to_string()),
//!         ..Default::default()
//!     });
//!
//!     // Connect to the server
//!     echo.connect().await?;
//!
//!     // Listen to a public channel
//!     echo.channel("orders")
//!         .listen("OrderCreated", |event| {
//!             println!("Order created: {:?}", event);
//!         })
//!         .await?;
//!
//!     // Listen to a private channel
//!     echo.private("orders.123")
//!         .listen("OrderUpdated", |event| {
//!             println!("Order updated: {:?}", event);
//!         })
//!         .await?;
//!
//!     // Join a presence channel
//!     echo.join("chat.room.1")
//!         .here(|users| println!("Users here: {:?}", users))
//!         .joining(|user| println!("User joining: {:?}", user))
//!         .leaving(|user| println!("User leaving: {:?}", user))
//!         .listen("MessageSent", |event| {
//!             println!("Message: {:?}", event);
//!         })
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Channel Types
//!
//! - **Public Channels**: Anyone can subscribe, no authentication required
//! - **Private Channels**: Require authentication, prefixed with `private-`
//! - **Presence Channels**: Like private, but track online users, prefixed with `presence-`

use async_trait::async_trait;
use dashmap::DashMap;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub mod auth;
pub mod channel;
pub mod connector;
pub mod presence;
pub mod pusher;

pub use auth::{AuthProvider, DefaultAuthProvider};
pub use channel::{Channel, ChannelType, PrivateChannel, PresenceChannel};
pub use connector::{Connector, ConnectorConfig};
pub use presence::{PresenceMember, PresenceState};
pub use pusher::PusherProtocol;

/// Echo error types
#[derive(Debug, Error)]
pub enum EchoError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout error: {0}")]
    Timeout(String),
}

pub type EchoResult<T> = Result<T, EchoError>;

/// Echo configuration
#[derive(Debug, Clone)]
pub struct EchoConfig {
    /// The broadcaster/connector to use
    pub broadcaster: Connector,
    /// Authentication endpoint for private/presence channels
    pub auth_endpoint: Option<String>,
    /// Additional headers for authentication requests
    pub auth_headers: HashMap<String, String>,
    /// CSRF token for authentication
    pub csrf_token: Option<String>,
    /// Namespace for events (prepended to event names)
    pub namespace: Option<String>,
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Reconnect delay in milliseconds
    pub reconnect_delay_ms: u64,
    /// Maximum reconnect attempts
    pub max_reconnect_attempts: u32,
}

impl Default for EchoConfig {
    fn default() -> Self {
        Self {
            broadcaster: Connector::Pusher {
                key: String::new(),
                cluster: "mt1".to_string(),
            },
            auth_endpoint: Some("/broadcasting/auth".to_string()),
            auth_headers: HashMap::new(),
            csrf_token: None,
            namespace: None,
            auto_reconnect: true,
            reconnect_delay_ms: 1000,
            max_reconnect_attempts: 10,
        }
    }
}

/// Event data received from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event name
    pub event: String,
    /// Channel name
    pub channel: Option<String>,
    /// Event data
    pub data: serde_json::Value,
}

/// Event handler callback type
pub type EventHandler = Arc<dyn Fn(Event) + Send + Sync>;

/// Presence event handler callback type
pub type PresenceHandler = Arc<dyn Fn(Vec<PresenceMember>) + Send + Sync>;

/// Single presence member handler callback type
pub type MemberHandler = Arc<dyn Fn(PresenceMember) + Send + Sync>;

/// Main Echo client
pub struct Echo {
    config: EchoConfig,
    connection: Arc<RwLock<Option<EchoConnection>>>,
    channels: Arc<DashMap<String, Arc<Channel>>>,
    event_tx: broadcast::Sender<Event>,
    socket_id: Arc<RwLock<Option<String>>>,
    auth_provider: Arc<dyn AuthProvider + Send + Sync>,
}

/// Internal connection state
struct EchoConnection {
    write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    _read_handle: tokio::task::JoinHandle<()>,
}

impl Echo {
    /// Create a new Echo instance
    pub fn new(config: EchoConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let auth_provider = Arc::new(DefaultAuthProvider::new(
            config.auth_endpoint.clone(),
            config.auth_headers.clone(),
            config.csrf_token.clone(),
        ));

        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            channels: Arc::new(DashMap::new()),
            event_tx,
            socket_id: Arc::new(RwLock::new(None)),
            auth_provider,
        }
    }

    /// Create Echo with a custom auth provider
    pub fn with_auth_provider(
        config: EchoConfig,
        auth_provider: Arc<dyn AuthProvider + Send + Sync>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
            channels: Arc::new(DashMap::new()),
            event_tx,
            socket_id: Arc::new(RwLock::new(None)),
            auth_provider,
        }
    }

    /// Connect to the broadcasting server
    pub async fn connect(&self) -> EchoResult<()> {
        let url = self.config.broadcaster.websocket_url();

        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| EchoError::ConnectionError(e.to_string()))?;

        let (write, read) = ws_stream.split();

        let event_tx = self.event_tx.clone();
        let socket_id = self.socket_id.clone();
        let channels = self.channels.clone();

        let read_handle = tokio::spawn(async move {
            Self::handle_messages(read, event_tx, socket_id, channels).await;
        });

        let mut conn = self.connection.write().await;
        *conn = Some(EchoConnection {
            write,
            _read_handle: read_handle,
        });

        Ok(())
    }

    /// Handle incoming WebSocket messages
    async fn handle_messages(
        mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        event_tx: broadcast::Sender<Event>,
        socket_id: Arc<RwLock<Option<String>>>,
        channels: Arc<DashMap<String, Arc<Channel>>>,
    ) {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(event) = serde_json::from_str::<PusherEvent>(&text) {
                        // Handle connection established
                        if event.event == "pusher:connection_established" {
                            if let Ok(data) = serde_json::from_str::<ConnectionData>(&event.data) {
                                let mut sid = socket_id.write().await;
                                *sid = Some(data.socket_id);
                            }
                        }

                        // Broadcast event to listeners
                        let echo_event = Event {
                            event: event.event.clone(),
                            channel: event.channel.clone(),
                            data: serde_json::from_str(&event.data).unwrap_or(serde_json::Value::Null),
                        };

                        let _ = event_tx.send(echo_event);
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    }

    /// Subscribe to a public channel
    pub async fn channel(&self, name: &str) -> EchoResult<Arc<Channel>> {
        let channel_name = name.to_string();

        if let Some(channel) = self.channels.get(&channel_name) {
            return Ok(channel.clone());
        }

        let channel = Arc::new(Channel::new(
            channel_name.clone(),
            ChannelType::Public,
            self.event_tx.subscribe(),
        ));

        self.subscribe_to_channel(&channel_name).await?;
        self.channels.insert(channel_name, channel.clone());

        Ok(channel)
    }

    /// Subscribe to a private channel
    pub async fn private(&self, name: &str) -> EchoResult<Arc<PrivateChannel>> {
        let channel_name = format!("private-{}", name);

        // Authenticate first
        let socket_id = self.socket_id.read().await;
        let socket_id = socket_id.as_ref().ok_or_else(|| {
            EchoError::AuthError("Not connected, no socket ID available".to_string())
        })?;

        let auth = self
            .auth_provider
            .authenticate(&channel_name, socket_id)
            .await?;

        let channel = Arc::new(PrivateChannel::new(
            channel_name.clone(),
            self.event_tx.subscribe(),
            auth,
        ));

        self.subscribe_to_private_channel(&channel_name, &channel.auth)
            .await?;

        Ok(channel)
    }

    /// Join a presence channel
    pub async fn join(&self, name: &str) -> EchoResult<Arc<PresenceChannel>> {
        let channel_name = format!("presence-{}", name);

        // Authenticate with presence data
        let socket_id = self.socket_id.read().await;
        let socket_id = socket_id.as_ref().ok_or_else(|| {
            EchoError::AuthError("Not connected, no socket ID available".to_string())
        })?;

        let user_info = auth::PresenceUserInfo {
            user_id: socket_id.clone(),
            user_info: serde_json::json!({}),
        };

        let presence_auth = self
            .auth_provider
            .authenticate_presence(&channel_name, socket_id, &user_info)
            .await?;

        let mut channel = PresenceChannel::new(
            channel_name.clone(),
            self.event_tx.subscribe(),
            presence_auth.auth,
        );
        channel.channel_data = presence_auth.channel_data;
        let channel = Arc::new(channel);

        self.subscribe_to_presence_channel(&channel_name, &channel.auth, &channel.channel_data)
            .await?;

        Ok(channel)
    }

    /// Leave a channel
    pub async fn leave(&self, name: &str) -> EchoResult<()> {
        self.unsubscribe_from_channel(name).await?;
        self.channels.remove(name);
        Ok(())
    }

    /// Subscribe to a channel
    async fn subscribe_to_channel(&self, channel: &str) -> EchoResult<()> {
        let message = serde_json::json!({
            "event": "pusher:subscribe",
            "data": {
                "channel": channel
            }
        });

        self.send_message(&message.to_string()).await
    }

    /// Subscribe to a private channel
    async fn subscribe_to_private_channel(&self, channel: &str, auth: &str) -> EchoResult<()> {
        let message = serde_json::json!({
            "event": "pusher:subscribe",
            "data": {
                "channel": channel,
                "auth": auth
            }
        });

        self.send_message(&message.to_string()).await
    }

    /// Subscribe to a presence channel
    async fn subscribe_to_presence_channel(
        &self,
        channel: &str,
        auth: &str,
        channel_data: &str,
    ) -> EchoResult<()> {
        let message = serde_json::json!({
            "event": "pusher:subscribe",
            "data": {
                "channel": channel,
                "auth": auth,
                "channel_data": channel_data
            }
        });

        self.send_message(&message.to_string()).await
    }

    /// Unsubscribe from a channel
    async fn unsubscribe_from_channel(&self, channel: &str) -> EchoResult<()> {
        let message = serde_json::json!({
            "event": "pusher:unsubscribe",
            "data": {
                "channel": channel
            }
        });

        self.send_message(&message.to_string()).await
    }

    /// Send a message to the server
    async fn send_message(&self, message: &str) -> EchoResult<()> {
        let mut conn = self.connection.write().await;
        if let Some(ref mut conn) = *conn {
            conn.write
                .send(Message::Text(message.to_string()))
                .await
                .map_err(|e| EchoError::WebSocketError(e.to_string()))?;
        }
        Ok(())
    }

    /// Disconnect from the server
    pub async fn disconnect(&self) -> EchoResult<()> {
        let mut conn = self.connection.write().await;
        if let Some(ref mut conn) = *conn {
            conn.write
                .send(Message::Close(None))
                .await
                .map_err(|e| EchoError::WebSocketError(e.to_string()))?;
        }
        *conn = None;
        Ok(())
    }

    /// Get the socket ID
    pub async fn socket_id(&self) -> Option<String> {
        self.socket_id.read().await.clone()
    }
}

/// Pusher event format
#[derive(Debug, Deserialize)]
struct PusherEvent {
    event: String,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    data: String,
}

/// Connection established data
#[derive(Debug, Deserialize)]
struct ConnectionData {
    socket_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_config_default() {
        let config = EchoConfig::default();
        assert!(config.auto_reconnect);
        assert_eq!(config.reconnect_delay_ms, 1000);
        assert_eq!(config.max_reconnect_attempts, 10);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event {
            event: "OrderCreated".to_string(),
            channel: Some("orders".to_string()),
            data: serde_json::json!({"id": 123}),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("OrderCreated"));
    }
}
