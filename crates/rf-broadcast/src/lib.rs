//! Real-time event broadcasting for RustForge
//!
//! Provides WebSocket-based broadcasting with channel support, presence tracking,
//! and multiple backend drivers.
//!
//! # Features
//!
//! - Event broadcasting to channels
//! - WebSocket support via Axum
//! - Public, private, and presence channels
//! - Memory backend for development
//! - Channel subscriptions and presence tracking
//!
//! # Quick Start
//!
//! ```no_run
//! use rf_broadcast::*;
//! use std::sync::Arc;
//! use serde_json::json;
//!
//! // Create broadcaster
//! let broadcaster = Arc::new(MemoryBroadcaster::new());
//!
//! // Subscribe connection using synchronous API
//! let channel = channel("users");
//! subscribe(
//!     Arc::clone(&broadcaster),
//!     &channel,
//!     "conn-123".to_string(),
//!     None,
//! ).expect("Failed to subscribe");
//!
//! // Broadcast event using synchronous API
//! let event = SimpleEvent::new(
//!     "user.created",
//!     json!({"id": 123, "name": "John"}),
//!     vec![channel.clone()],
//! );
//!
//! broadcast(broadcaster, &channel, &event)
//!     .expect("Failed to broadcast");
//! ```
//!
//! # WebSocket Integration
//!
//! ```no_run
//! use rf_broadcast::*;
//! use axum::Router;
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let broadcaster = Arc::new(MemoryBroadcaster::new());
//!
//! // WebSocket router still uses async (server-side)
//! let app = Router::new()
//!     .merge(websocket_router(broadcaster));
//!
//! // But broadcasting from your app uses sync API
//! // Start server...
//! # }
//! ```

pub mod api;
pub mod auth;
mod broadcaster;
mod channel;
mod error;
mod event;
mod memory;
mod websocket;

#[cfg(feature = "redis-backend")]
mod redis;

// Phase 19: Pusher driver
#[cfg(feature = "pusher")]
pub mod pusher;

pub use api::{
    broadcast, channel, connections, presence, subscribe, unsubscribe, BroadcastFacade,
};
pub use auth::{AllowAllAuthorizer, ChannelAuthorizer, PublicOnlyAuthorizer, WebSocketAuth};
pub use broadcaster::{Broadcaster, ConnectionId, PresenceInfo, UserId};
pub use channel::Channel;
pub use error::{BroadcastError, BroadcastResult};
pub use event::{Event, SimpleEvent};
pub use memory::{BroadcastMessage, MemoryBroadcaster};
pub use websocket::{websocket_router, WsMessage, WsState};

#[cfg(feature = "redis-backend")]
pub use redis::RedisBroadcaster;

#[cfg(feature = "pusher")]
pub use pusher::{PusherBatchResponse, PusherBroadcaster, PusherConfig, PusherEvent};
