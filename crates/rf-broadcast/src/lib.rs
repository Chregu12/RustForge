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
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create broadcaster
//! let broadcaster = Arc::new(MemoryBroadcaster::new());
//!
//! // Subscribe connection
//! broadcaster.subscribe(
//!     &Channel::public("users"),
//!     "conn-123".to_string(),
//!     None,
//! ).await?;
//!
//! // Broadcast event
//! let event = SimpleEvent::new(
//!     "user.created",
//!     json!({"id": 123, "name": "John"}),
//!     vec![Channel::public("users")],
//! );
//!
//! broadcaster.broadcast(&Channel::public("users"), &event).await?;
//! # Ok(())
//! # }
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
//! let app = Router::new()
//!     .merge(websocket_router(broadcaster));
//!
//! // Start server...
//! # }
//! ```

mod broadcaster;
mod channel;
mod error;
mod event;
mod memory;
mod websocket;

pub use broadcaster::{Broadcaster, ConnectionId, PresenceInfo, UserId};
pub use channel::Channel;
pub use error::{BroadcastError, BroadcastResult};
pub use event::{Event, SimpleEvent};
pub use memory::{BroadcastMessage, MemoryBroadcaster};
pub use websocket::{websocket_router, WsMessage, WsState};
