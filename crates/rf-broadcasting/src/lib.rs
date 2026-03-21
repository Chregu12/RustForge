//! Broadcasting and WebSocket support for RustForge
//!
//! This crate provides real-time event broadcasting capabilities similar to Laravel's
//! broadcasting system, with support for WebSockets and Redis Pub/Sub.
//!
//! # Features
//!
//! - WebSocket server for real-time client connections
//! - Redis Pub/Sub driver for distributed broadcasting
//! - Channel authorization (public, private, presence)
//! - Event broadcasting with custom channels and data
//!
//! # Example
//!
//! ```rust,no_run
//! use rf_broadcasting::{Broadcaster, Broadcast};
//! use serde_json::json;
//!
//! #[derive(Debug)]
//! struct OrderShipped {
//!     order_id: u64,
//!     customer_id: u64,
//! }
//!
//! impl Broadcast for OrderShipped {
//!     fn broadcast_on(&self) -> Vec<String> {
//!         vec![format!("orders.{}", self.customer_id)]
//!     }
//!
//!     fn broadcast_with(&self) -> serde_json::Value {
//!         json!({
//!             "order_id": self.order_id,
//!             "customer_id": self.customer_id,
//!         })
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let redis_pool = todo!();
//! let broadcaster = Broadcaster::new(redis_pool);
//!
//! let event = OrderShipped {
//!     order_id: 123,
//!     customer_id: 456,
//! };
//!
//! broadcaster.broadcast(event).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

pub mod auth;
pub mod drivers;
pub mod websocket;

// Re-exports
pub use auth::{ChannelAuthorization, ChannelType};
pub use drivers::redis::RedisBroadcastDriver;
pub use websocket::{WebSocketConfig, WebSocketServer};

/// Result type for broadcasting operations
pub type BroadcastResult<T> = Result<T, BroadcastError>;

/// Errors that can occur during broadcasting
#[derive(Debug, Error)]
pub enum BroadcastError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Channel authorization failed")]
    Unauthorized,

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Trait for events that can be broadcasted
#[async_trait]
pub trait Broadcast: Send + Sync {
    /// Which channels should receive this event
    fn broadcast_on(&self) -> Vec<String>;

    /// Custom event name (defaults to struct name)
    fn broadcast_as(&self) -> Option<String> {
        None
    }

    /// Event data to broadcast
    fn broadcast_with(&self) -> serde_json::Value;

    /// Whether to exclude the current user (useful for echo prevention)
    fn exclude_current(&self) -> bool {
        false
    }
}

/// Trait for broadcast drivers (Redis, Pusher, etc.)
#[async_trait]
pub trait BroadcastDriver: Send + Sync {
    /// Broadcast an event to the specified channels
    async fn broadcast(
        &self,
        channels: &[String],
        event: &str,
        data: serde_json::Value,
    ) -> BroadcastResult<()>;

    /// Subscribe to channels (for receiving messages)
    async fn subscribe(&self, channels: &[String]) -> BroadcastResult<()>;

    /// Unsubscribe from channels
    async fn unsubscribe(&self, channels: &[String]) -> BroadcastResult<()>;
}

/// Main broadcaster that coordinates event broadcasting
pub struct Broadcaster {
    driver: Arc<dyn BroadcastDriver>,
}

impl Broadcaster {
    /// Create a new broadcaster with the given driver
    pub fn new(driver: Arc<dyn BroadcastDriver>) -> Self {
        Self { driver }
    }

    /// Create a broadcaster backed by a Redis Pub/Sub driver.
    ///
    /// This is the most convenient way to get started with Redis broadcasting:
    ///
    /// ```rust,no_run
    /// use rf_broadcasting::Broadcaster;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let broadcaster = Broadcaster::from_redis_url("redis://localhost:6379")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_redis_url(url: &str) -> BroadcastResult<Self> {
        let driver = drivers::redis::RedisBroadcastDriver::from_url(url)?;
        Ok(Self {
            driver: Arc::new(driver),
        })
    }

    /// Create a broadcaster backed by Redis, reading `REDIS_URL` from the environment.
    ///
    /// Falls back to `redis://localhost:6379` if the variable is not set.
    ///
    /// ```rust,no_run
    /// use rf_broadcasting::Broadcaster;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let broadcaster = Broadcaster::from_env()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> BroadcastResult<Self> {
        let url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        Self::from_redis_url(&url)
    }

    /// Broadcast an event
    pub async fn broadcast<T: Broadcast>(&self, event: T) -> BroadcastResult<()> {
        let channels = event.broadcast_on();
        let event_name = event
            .broadcast_as()
            .unwrap_or_else(|| std::any::type_name::<T>().to_string());
        let data = event.broadcast_with();

        tracing::debug!(
            event = %event_name,
            channels = ?channels,
            "Broadcasting event"
        );

        self.driver.broadcast(&channels, &event_name, data).await
    }

    /// Broadcast to specific channels with custom data
    pub async fn to_channels(
        &self,
        channels: &[String],
        event: &str,
        data: serde_json::Value,
    ) -> BroadcastResult<()> {
        self.driver.broadcast(channels, event, data).await
    }
}

/// Message received from WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "lowercase")]
pub enum ClientMessage {
    /// Subscribe to a channel
    Subscribe {
        channel: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<String>,
    },
    /// Unsubscribe from a channel
    Unsubscribe { channel: String },
    /// Ping to keep connection alive
    Ping,
}

/// Message sent to WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    /// Event broadcast to channel
    Event {
        channel: String,
        event: String,
        data: serde_json::Value,
    },
    /// Subscription confirmation
    Subscribed { channel: String },
    /// Unsubscription confirmation
    Unsubscribed { channel: String },
    /// Pong response
    Pong,
    /// Error message
    Error { message: String },
}

impl ServerMessage {
    /// Convert to JSON string
    pub fn to_json(&self) -> BroadcastResult<String> {
        Ok(serde_json::to_string(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug)]
    struct TestEvent {
        message: String,
    }

    impl Broadcast for TestEvent {
        fn broadcast_on(&self) -> Vec<String> {
            vec!["test-channel".to_string()]
        }

        fn broadcast_with(&self) -> serde_json::Value {
            json!({ "message": self.message })
        }
    }

    #[test]
    fn test_event_trait() {
        let event = TestEvent {
            message: "Hello".to_string(),
        };

        assert_eq!(event.broadcast_on(), vec!["test-channel"]);
        assert_eq!(event.broadcast_with()["message"], "Hello");
        assert_eq!(event.broadcast_as(), None);
    }

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Subscribe {
            channel: "orders".to_string(),
            auth: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("subscribe"));
        assert!(json.contains("orders"));
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Event {
            channel: "orders".to_string(),
            event: "OrderShipped".to_string(),
            data: json!({"order_id": 123}),
        };

        let json = msg.to_json().unwrap();
        assert!(json.contains("event"));
        assert!(json.contains("OrderShipped"));
    }
}
