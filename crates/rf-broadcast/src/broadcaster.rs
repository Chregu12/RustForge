//! Broadcaster trait and types

use crate::{BroadcastError, Channel, Event};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Connection ID type
pub type ConnectionId = String;

/// User ID type
pub type UserId = String;

/// Trait for broadcast backends
#[async_trait]
pub trait Broadcaster: Send + Sync {
    /// Broadcast event to channel
    async fn broadcast(&self, channel: &Channel, event: &dyn Event)
        -> Result<(), BroadcastError>;

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
    async fn connections(&self, channel: &Channel) -> Result<Vec<ConnectionId>, BroadcastError>;

    /// Get presence info for channel (only for presence channels)
    async fn presence(&self, channel: &Channel) -> Result<Vec<PresenceInfo>, BroadcastError>;

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

impl PresenceInfo {
    /// Create new presence info
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            user_info: None,
            joined_at: chrono::Utc::now(),
        }
    }

    /// Create with user info
    pub fn with_info(user_id: UserId, user_info: serde_json::Value) -> Self {
        Self {
            user_id,
            user_info: Some(user_info),
            joined_at: chrono::Utc::now(),
        }
    }
}
