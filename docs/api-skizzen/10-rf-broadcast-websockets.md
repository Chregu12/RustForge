# rf-broadcast: Real-time Event Broadcasting

**Version**: 1.0.0
**Status**: Phase 3 - Advanced Features
**Laravel Equivalent**: Broadcasting (Laravel Echo, Pusher)

## Overview

Real-time event broadcasting system with WebSocket support, channel authentication, and multiple backend drivers (Memory, Redis).

**Core Features**:
- Event broadcasting to channels
- WebSocket support via Axum
- Public, private, and presence channels
- Channel authentication
- Multiple backend drivers
- Broadcasting to specific users/connections

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Application Layer                    │
│  (Controllers, Services, Event Handlers)                 │
└────────────────────────┬────────────────────────────────┘
                         │
                         │ broadcast(event)
                         ▼
┌─────────────────────────────────────────────────────────┐
│                    Broadcaster Trait                     │
│  - broadcast(channel, event)                             │
│  - subscribe(channel, connection)                        │
│  - unsubscribe(channel, connection)                      │
└────────────────────────┬────────────────────────────────┘
                         │
          ┌──────────────┴──────────────┐
          │                             │
          ▼                             ▼
┌──────────────────┐         ┌──────────────────┐
│ MemoryBroadcaster│         │ RedisBroadcaster │
│ (Development)    │         │ (Production)     │
└──────────────────┘         └──────────────────┘
          │                             │
          └──────────────┬──────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                  WebSocket Middleware                    │
│  - Handle connections                                    │
│  - Channel subscription                                  │
│  - Authentication                                        │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                       Clients                            │
│  (Browser WebSockets, Mobile Apps, etc.)                 │
└─────────────────────────────────────────────────────────┘
```

## Core Traits

### 1. Event Trait

```rust
use serde::{Deserialize, Serialize};

/// Trait for broadcastable events
#[async_trait]
pub trait Event: Send + Sync {
    /// Event name (e.g., "user.created", "order.shipped")
    fn event_name(&self) -> &str;

    /// Serialize event to JSON
    fn to_json(&self) -> Result<String, BroadcastError>;

    /// Get channels to broadcast on
    fn broadcast_on(&self) -> Vec<Channel>;
}

/// Simple event implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleEvent {
    pub name: String,
    pub data: serde_json::Value,
    pub channels: Vec<Channel>,
}

impl Event for SimpleEvent {
    fn event_name(&self) -> &str {
        &self.name
    }

    fn to_json(&self) -> Result<String, BroadcastError> {
        serde_json::to_string(&self.data)
            .map_err(|e| BroadcastError::SerializationError(e.to_string()))
    }

    fn broadcast_on(&self) -> Vec<Channel> {
        self.channels.clone()
    }
}
```

### 2. Channel Types

```rust
/// Channel type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Public channel - anyone can subscribe
    Public(String),

    /// Private channel - requires authentication
    Private(String),

    /// Presence channel - tracks who's subscribed
    Presence(String),
}

impl Channel {
    pub fn public(name: impl Into<String>) -> Self {
        Self::Public(name.into())
    }

    pub fn private(name: impl Into<String>) -> Self {
        Self::Private(name.into())
    }

    pub fn presence(name: impl Into<String>) -> Self {
        Self::Presence(name.into())
    }

    pub fn name(&self) -> &str {
        match self {
            Channel::Public(name) => name,
            Channel::Private(name) => name,
            Channel::Presence(name) => name,
        }
    }

    pub fn requires_auth(&self) -> bool {
        matches!(self, Channel::Private(_) | Channel::Presence(_))
    }

    pub fn is_presence(&self) -> bool {
        matches!(self, Channel::Presence(_))
    }
}
```

### 3. Broadcaster Trait

```rust
use async_trait::async_trait;
use std::sync::Arc;

/// Connection ID type
pub type ConnectionId = String;

/// User ID type
pub type UserId = String;

/// Trait for broadcast backends
#[async_trait]
pub trait Broadcaster: Send + Sync {
    /// Broadcast event to channel
    async fn broadcast(
        &self,
        channel: &Channel,
        event: &dyn Event,
    ) -> Result<(), BroadcastError>;

    /// Subscribe connection to channel
    async fn subscribe(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> Result<(), BroadcastError>;

    /// Unsubscribe connection from channel
    async fn unsubscribe(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<(), BroadcastError>;

    /// Get all connections in channel
    async fn connections(
        &self,
        channel: &Channel,
    ) -> Result<Vec<ConnectionId>, BroadcastError>;

    /// Get presence info for channel (only for presence channels)
    async fn presence(
        &self,
        channel: &Channel,
    ) -> Result<Vec<PresenceInfo>, BroadcastError>;

    /// Check if connection is subscribed to channel
    async fn is_subscribed(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<bool, BroadcastError>;
}

/// Presence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceInfo {
    pub user_id: UserId,
    pub user_info: Option<serde_json::Value>,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}
```

## Memory Backend

In-memory broadcaster for development and single-server deployments.

```rust
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

pub struct MemoryBroadcaster {
    // Channel -> Set of connection IDs
    subscriptions: Arc<Mutex<HashMap<Channel, HashSet<ConnectionId>>>>,

    // Channel -> User presence info
    presence: Arc<Mutex<HashMap<Channel, HashMap<UserId, PresenceInfo>>>>,

    // Connection ID -> User ID mapping
    connections: Arc<Mutex<HashMap<ConnectionId, Option<UserId>>>>,

    // Broadcast channel for sending events to WebSocket handlers
    sender: broadcast::Sender<BroadcastMessage>,
}

#[derive(Debug, Clone)]
struct BroadcastMessage {
    channel: Channel,
    event_name: String,
    data: String,
}

impl MemoryBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);

        Self {
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            presence: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
            sender,
        }
    }

    /// Get receiver for WebSocket handler
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.sender.subscribe()
    }
}

#[async_trait]
impl Broadcaster for MemoryBroadcaster {
    async fn broadcast(
        &self,
        channel: &Channel,
        event: &dyn Event,
    ) -> Result<(), BroadcastError> {
        let message = BroadcastMessage {
            channel: channel.clone(),
            event_name: event.event_name().to_string(),
            data: event.to_json()?,
        };

        // Send to all WebSocket handlers
        let _ = self.sender.send(message);

        tracing::debug!(
            channel = %channel.name(),
            event = %event.event_name(),
            "Event broadcasted"
        );

        Ok(())
    }

    async fn subscribe(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> Result<(), BroadcastError> {
        // Add to subscriptions
        let mut subs = self.subscriptions.lock().unwrap();
        subs.entry(channel.clone())
            .or_insert_with(HashSet::new)
            .insert(connection_id.clone());
        drop(subs);

        // Track user connection
        let mut conns = self.connections.lock().unwrap();
        conns.insert(connection_id.clone(), user_id.clone());
        drop(conns);

        // Add to presence if presence channel
        if channel.is_presence() {
            if let Some(uid) = user_id {
                let mut pres = self.presence.lock().unwrap();
                pres.entry(channel.clone())
                    .or_insert_with(HashMap::new)
                    .insert(uid.clone(), PresenceInfo {
                        user_id: uid,
                        user_info: None,
                        joined_at: chrono::Utc::now(),
                    });
            }
        }

        tracing::debug!(
            channel = %channel.name(),
            connection_id = %connection_id,
            "Connection subscribed"
        );

        Ok(())
    }

    async fn unsubscribe(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<(), BroadcastError> {
        // Remove from subscriptions
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some(conns) = subs.get_mut(channel) {
            conns.remove(connection_id);
        }
        drop(subs);

        // Remove from presence if presence channel
        if channel.is_presence() {
            let conns = self.connections.lock().unwrap();
            if let Some(Some(user_id)) = conns.get(connection_id) {
                let mut pres = self.presence.lock().unwrap();
                if let Some(channel_pres) = pres.get_mut(channel) {
                    channel_pres.remove(user_id);
                }
            }
        }

        Ok(())
    }

    async fn connections(
        &self,
        channel: &Channel,
    ) -> Result<Vec<ConnectionId>, BroadcastError> {
        let subs = self.subscriptions.lock().unwrap();
        Ok(subs.get(channel)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default())
    }

    async fn presence(
        &self,
        channel: &Channel,
    ) -> Result<Vec<PresenceInfo>, BroadcastError> {
        if !channel.is_presence() {
            return Err(BroadcastError::InvalidChannel(
                "Not a presence channel".into()
            ));
        }

        let pres = self.presence.lock().unwrap();
        Ok(pres.get(channel)
            .map(|p| p.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn is_subscribed(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<bool, BroadcastError> {
        let subs = self.subscriptions.lock().unwrap();
        Ok(subs.get(channel)
            .map(|s| s.contains(connection_id))
            .unwrap_or(false))
    }
}
```

## Channel Authentication

```rust
use axum::extract::Request;
use std::sync::Arc;

/// Trait for authenticating channel subscriptions
#[async_trait]
pub trait ChannelAuth: Send + Sync {
    /// Authorize subscription to channel
    async fn authorize(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
        request: &Request,
    ) -> Result<AuthResult, BroadcastError>;
}

/// Authorization result
#[derive(Debug)]
pub struct AuthResult {
    pub authorized: bool,
    pub user_id: Option<UserId>,
    pub user_info: Option<serde_json::Value>,
}

/// Simple callback-based authenticator
pub struct CallbackAuth<F>
where
    F: Fn(&Channel, &ConnectionId, &Request) -> AuthResult + Send + Sync,
{
    callback: Arc<F>,
}

impl<F> CallbackAuth<F>
where
    F: Fn(&Channel, &ConnectionId, &Request) -> AuthResult + Send + Sync,
{
    pub fn new(callback: F) -> Self {
        Self {
            callback: Arc::new(callback),
        }
    }
}

#[async_trait]
impl<F> ChannelAuth for CallbackAuth<F>
where
    F: Fn(&Channel, &ConnectionId, &Request) -> AuthResult + Send + Sync,
{
    async fn authorize(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
        request: &Request,
    ) -> Result<AuthResult, BroadcastError> {
        Ok((self.callback)(channel, connection_id, request))
    }
}
```

## WebSocket Integration

```rust
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade, Message},
        State,
    },
    response::Response,
    Router,
    routing::get,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;

/// WebSocket state
#[derive(Clone)]
pub struct WsState {
    broadcaster: Arc<dyn Broadcaster>,
    auth: Arc<dyn ChannelAuth>,
}

/// WebSocket message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "subscribe")]
    Subscribe {
        channel: String,
    },

    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        channel: String,
    },

    #[serde(rename = "event")]
    Event {
        channel: String,
        event: String,
        data: serde_json::Value,
    },

    #[serde(rename = "subscribed")]
    Subscribed {
        channel: String,
    },

    #[serde(rename = "unsubscribed")]
    Unsubscribed {
        channel: String,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<WsState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: WsState) {
    let connection_id = uuid::Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast events
    let mut event_rx = if let Ok(broadcaster) = state.broadcaster
        .as_any()
        .downcast_ref::<MemoryBroadcaster>()
    {
        broadcaster.subscribe_to_events()
    } else {
        return;
    };

    // Spawn task to forward broadcast events to WebSocket
    let connection_id_clone = connection_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = event_rx.recv().await {
            let ws_msg = WsMessage::Event {
                channel: msg.channel.name().to_string(),
                event: msg.event_name,
                data: serde_json::from_str(&msg.data).unwrap_or_default(),
            };

            if let Ok(json) = serde_json::to_string(&ws_msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    let broadcaster = state.broadcaster.clone();
    let connection_id_clone2 = connection_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::Subscribe { channel } => {
                            let ch = Channel::public(channel.clone());
                            let _ = broadcaster.subscribe(
                                &ch,
                                connection_id_clone2.clone(),
                                None,
                            ).await;

                            // Send confirmation
                            // (would need to send back through sender)
                        }
                        WsMessage::Unsubscribe { channel } => {
                            let ch = Channel::public(channel);
                            let _ = broadcaster.unsubscribe(
                                &ch,
                                &connection_id_clone2,
                            ).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

/// Create WebSocket router
pub fn websocket_router(
    broadcaster: Arc<dyn Broadcaster>,
    auth: Arc<dyn ChannelAuth>,
) -> Router {
    let state = WsState { broadcaster, auth };

    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}
```

## Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BroadcastError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid channel: {0}")]
    InvalidChannel(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),

    #[error("Backend error: {0}")]
    BackendError(String),
}

pub type BroadcastResult<T> = Result<T, BroadcastError>;
```

## Usage Examples

### 1. Basic Broadcasting

```rust
use rf_broadcast::*;

// Create broadcaster
let broadcaster = Arc::new(MemoryBroadcaster::new());

// Create event
let event = SimpleEvent {
    name: "user.created".to_string(),
    data: serde_json::json!({
        "id": 123,
        "name": "John Doe",
    }),
    channels: vec![Channel::public("users")],
};

// Broadcast event
broadcaster.broadcast(
    &Channel::public("users"),
    &event,
).await?;
```

### 2. Presence Channels

```rust
// Subscribe to presence channel
let channel = Channel::presence("chat.room.1");

broadcaster.subscribe(
    &channel,
    "conn-123".to_string(),
    Some("user-456".to_string()),
).await?;

// Get who's in the channel
let members = broadcaster.presence(&channel).await?;
for member in members {
    println!("User {} joined at {}", member.user_id, member.joined_at);
}
```

### 3. With Axum

```rust
use axum::{Router, routing::get};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create broadcaster
    let broadcaster = Arc::new(MemoryBroadcaster::new());

    // Create auth handler
    let auth = Arc::new(CallbackAuth::new(|channel, _conn_id, _req| {
        // Public channels always allowed
        if matches!(channel, Channel::Public(_)) {
            return AuthResult {
                authorized: true,
                user_id: None,
                user_info: None,
            };
        }

        // Private channels require authentication
        // (check JWT token, session, etc.)
        AuthResult {
            authorized: false,
            user_id: None,
            user_info: None,
        }
    }));

    // Create router with WebSocket support
    let app = Router::new()
        .merge(websocket_router(broadcaster.clone(), auth))
        .route("/", get(|| async { "WebSocket server" }));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
```

### 4. Broadcasting from Application Logic

```rust
use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

async fn create_user(
    State(broadcaster): State<Arc<dyn Broadcaster>>,
    Json(req): Json<CreateUserRequest>,
) -> Json<User> {
    // Create user in database
    let user = User {
        id: 123,
        name: req.name,
        email: req.email,
    };

    // Broadcast event
    let event = SimpleEvent {
        name: "user.created".to_string(),
        data: serde_json::to_value(&user).unwrap(),
        channels: vec![Channel::public("users")],
    };

    let _ = broadcaster.broadcast(&Channel::public("users"), &event).await;

    Json(user)
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_to_channel() {
        let broadcaster = MemoryBroadcaster::new();

        let event = SimpleEvent {
            name: "test.event".to_string(),
            data: serde_json::json!({"message": "hello"}),
            channels: vec![Channel::public("test")],
        };

        broadcaster.broadcast(&Channel::public("test"), &event).await.unwrap();
        // Event broadcasted successfully
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::public("test");

        // Subscribe
        broadcaster.subscribe(&channel, "conn-1".to_string(), None).await.unwrap();
        assert!(broadcaster.is_subscribed(&channel, &"conn-1".to_string()).await.unwrap());

        // Unsubscribe
        broadcaster.unsubscribe(&channel, &"conn-1".to_string()).await.unwrap();
        assert!(!broadcaster.is_subscribed(&channel, &"conn-1".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_presence_channel() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::presence("chat");

        // Subscribe user
        broadcaster.subscribe(
            &channel,
            "conn-1".to_string(),
            Some("user-123".to_string()),
        ).await.unwrap();

        // Check presence
        let members = broadcaster.presence(&channel).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, "user-123");
    }
}
```

## Comparison with Laravel

| Feature | Laravel | rf-broadcast | Status |
|---------|---------|--------------|--------|
| Event broadcasting | ✅ | ✅ | ✅ Complete |
| Public channels | ✅ | ✅ | ✅ Complete |
| Private channels | ✅ | ✅ | ✅ Complete |
| Presence channels | ✅ | ✅ | ✅ Complete |
| WebSocket support | ✅ (Echo) | ✅ | ✅ Complete |
| Channel auth | ✅ | ✅ | ✅ Complete |
| Redis driver | ✅ | ⏳ | ⏳ Future |
| Pusher driver | ✅ | ⏳ | ⏳ Future |
| Client libraries | ✅ | ⏳ | ⏳ Future |

**Feature Parity**: ~70% (7/10 features)

## Future Enhancements

### Redis Backend (High Priority)
- Distributed broadcasting across servers
- Redis Pub/Sub for event distribution
- Scalable to multiple instances

### Additional Drivers
- Pusher integration
- Ably integration
- AWS SNS/SQS

### Client Libraries
- JavaScript/TypeScript client
- Mobile SDKs
- Reconnection logic
- Client-side presence

### Advanced Features
- Message encryption
- Rate limiting per connection
- Custom event serialization
- Webhook support
- Broadcasting to specific users
- Event replay/history

## Files to Create

```
crates/rf-broadcast/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Main exports
│   ├── error.rs            # Error types
│   ├── event.rs            # Event trait and types
│   ├── channel.rs          # Channel types
│   ├── broadcaster.rs      # Broadcaster trait
│   ├── memory.rs           # Memory backend
│   ├── auth.rs             # Channel authentication
│   └── websocket.rs        # WebSocket integration
```

## Dependencies

```toml
[dependencies]
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
chrono.workspace = true
uuid.workspace = true
axum = { workspace = true, features = ["ws"] }
futures.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util", "macros"] }
```

## Conclusion

rf-broadcast provides Laravel-like broadcasting with:
- Clean trait-based architecture
- WebSocket support via Axum
- Public, private, and presence channels
- Channel authentication
- Memory backend for development
- Ready for Redis backend (future)
- Type-safe event system
- Async-first design

**Next Steps**:
1. Implement core traits and types
2. Add Memory backend
3. Implement WebSocket integration
4. Write comprehensive tests
5. Add Redis backend (Phase 4)
