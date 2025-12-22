//! Synchronous public API for broadcasting
//!
//! This module provides a synchronous public API while using async operations internally.
//! This follows Laravel's pattern where broadcast operations appear synchronous to the user.

use crate::{
    BroadcastResult, Broadcaster, Channel, ConnectionId, Event, PresenceInfo, UserId,
};
use rf_core::runtime::block_on;
use std::sync::Arc;

/// Synchronous broadcaster API facade
pub struct BroadcastFacade<B: Broadcaster> {
    broadcaster: Arc<B>,
}

impl<B: Broadcaster> BroadcastFacade<B> {
    /// Create new broadcast facade
    pub fn new(broadcaster: Arc<B>) -> Self {
        Self { broadcaster }
    }

    /// Broadcast event to channel (synchronous API)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_broadcast::{MemoryBroadcaster, BroadcastFacade, Channel, SimpleEvent};
    /// use serde_json::json;
    /// use std::sync::Arc;
    ///
    /// let broadcaster = Arc::new(MemoryBroadcaster::new());
    /// let facade = BroadcastFacade::new(broadcaster);
    ///
    /// let channel = Channel::public("users");
    /// let event = SimpleEvent::new(
    ///     "user.created",
    ///     json!({"id": 123}),
    ///     vec![channel.clone()],
    /// );
    ///
    /// facade.broadcast(&channel, &event).unwrap();
    /// ```
    pub fn broadcast(&self, channel: &Channel, event: &dyn Event) -> BroadcastResult<()> {
        let broadcaster = Arc::clone(&self.broadcaster);
        let channel = channel.clone();
        block_on(async move { broadcaster.broadcast(&channel, event).await })
    }

    /// Subscribe connection to channel (synchronous API)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_broadcast::{MemoryBroadcaster, BroadcastFacade, Channel};
    /// use std::sync::Arc;
    ///
    /// let broadcaster = Arc::new(MemoryBroadcaster::new());
    /// let facade = BroadcastFacade::new(broadcaster);
    ///
    /// let channel = Channel::public("users");
    /// facade.subscribe(&channel, "conn-123".to_string(), None).unwrap();
    /// ```
    pub fn subscribe(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> BroadcastResult<()> {
        let broadcaster = Arc::clone(&self.broadcaster);
        let channel = channel.clone();
        block_on(
            async move { broadcaster.subscribe(&channel, connection_id, user_id).await },
        )
    }

    /// Unsubscribe connection from channel (synchronous API)
    pub fn unsubscribe(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> BroadcastResult<()> {
        let broadcaster = Arc::clone(&self.broadcaster);
        let channel = channel.clone();
        let conn_id = connection_id.clone();
        block_on(async move { broadcaster.unsubscribe(&channel, &conn_id).await })
    }

    /// Get all connections in channel (synchronous API)
    pub fn connections(&self, channel: &Channel) -> BroadcastResult<Vec<ConnectionId>> {
        let broadcaster = Arc::clone(&self.broadcaster);
        let channel = channel.clone();
        block_on(async move { broadcaster.connections(&channel).await })
    }

    /// Get presence info for channel (synchronous API)
    pub fn presence(&self, channel: &Channel) -> BroadcastResult<Vec<PresenceInfo>> {
        let broadcaster = Arc::clone(&self.broadcaster);
        let channel = channel.clone();
        block_on(async move { broadcaster.presence(&channel).await })
    }

    /// Check if connection is subscribed (synchronous API)
    pub fn is_subscribed(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> BroadcastResult<bool> {
        let broadcaster = Arc::clone(&self.broadcaster);
        let channel = channel.clone();
        let conn_id = connection_id.clone();
        block_on(async move { broadcaster.is_subscribed(&channel, &conn_id).await })
    }
}

/// Helper function to create a channel (synchronous API)
///
/// # Example
///
/// ```
/// use rf_broadcast::channel;
///
/// let public_channel = channel("users");
/// let private_channel = channel("private-users.123");
/// let presence_channel = channel("presence-chat.456");
/// ```
pub fn channel(name: &str) -> Channel {
    Channel::new(name)
}

/// Helper function to broadcast an event (synchronous API)
///
/// # Example
///
/// ```no_run
/// use rf_broadcast::{MemoryBroadcaster, broadcast, Channel, SimpleEvent};
/// use serde_json::json;
/// use std::sync::Arc;
///
/// let broadcaster = Arc::new(MemoryBroadcaster::new());
/// let channel = Channel::public("users");
/// let event = SimpleEvent::new(
///     "user.created",
///     json!({"id": 123}),
///     vec![channel.clone()],
/// );
///
/// broadcast(broadcaster, &channel, &event).unwrap();
/// ```
pub fn broadcast<B: Broadcaster>(
    broadcaster: Arc<B>,
    channel: &Channel,
    event: &dyn Event,
) -> BroadcastResult<()> {
    let facade = BroadcastFacade::new(broadcaster);
    facade.broadcast(channel, event)
}

/// Helper function to subscribe to a channel (synchronous API)
///
/// # Example
///
/// ```no_run
/// use rf_broadcast::{MemoryBroadcaster, subscribe, Channel};
/// use std::sync::Arc;
///
/// let broadcaster = Arc::new(MemoryBroadcaster::new());
/// let channel = Channel::public("users");
///
/// subscribe(broadcaster, &channel, "conn-123".to_string(), None).unwrap();
/// ```
pub fn subscribe<B: Broadcaster>(
    broadcaster: Arc<B>,
    channel: &Channel,
    connection_id: ConnectionId,
    user_id: Option<UserId>,
) -> BroadcastResult<()> {
    let facade = BroadcastFacade::new(broadcaster);
    facade.subscribe(channel, connection_id, user_id)
}

/// Helper function to unsubscribe from a channel (synchronous API)
pub fn unsubscribe<B: Broadcaster>(
    broadcaster: Arc<B>,
    channel: &Channel,
    connection_id: &ConnectionId,
) -> BroadcastResult<()> {
    let facade = BroadcastFacade::new(broadcaster);
    facade.unsubscribe(channel, connection_id)
}

/// Helper function to get channel connections (synchronous API)
pub fn connections<B: Broadcaster>(
    broadcaster: Arc<B>,
    channel: &Channel,
) -> BroadcastResult<Vec<ConnectionId>> {
    let facade = BroadcastFacade::new(broadcaster);
    facade.connections(channel)
}

/// Helper function to get presence info (synchronous API)
pub fn presence<B: Broadcaster>(
    broadcaster: Arc<B>,
    channel: &Channel,
) -> BroadcastResult<Vec<PresenceInfo>> {
    let facade = BroadcastFacade::new(broadcaster);
    facade.presence(channel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryBroadcaster, SimpleEvent};
    use serde_json::json;

    #[test]
    fn test_sync_channel() {
        let ch = channel("users");
        assert_eq!(ch.name(), "users");
    }

    #[test]
    fn test_sync_broadcast() {
        let broadcaster = Arc::new(MemoryBroadcaster::new());
        let channel = Channel::public("users");
        let event = SimpleEvent::new("user.created", json!({"id": 123}), vec![channel.clone()]);

        let result = broadcast(broadcaster, &channel, &event);
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_subscribe() {
        let broadcaster = Arc::new(MemoryBroadcaster::new());
        let facade = BroadcastFacade::new(broadcaster);
        let channel = Channel::public("users");

        let result = facade.subscribe(&channel, "conn-123".to_string(), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_connections() {
        let broadcaster = Arc::new(MemoryBroadcaster::new());
        let facade = BroadcastFacade::new(broadcaster);
        let channel = Channel::public("users");

        // Subscribe first
        facade
            .subscribe(&channel, "conn-123".to_string(), None)
            .unwrap();

        // Get connections
        let conns = facade.connections(&channel).unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0], "conn-123");
    }

    #[test]
    fn test_facade_unsubscribe() {
        let broadcaster = Arc::new(MemoryBroadcaster::new());
        let facade = BroadcastFacade::new(broadcaster);
        let channel = Channel::public("users");

        // Subscribe first
        facade
            .subscribe(&channel, "conn-123".to_string(), None)
            .unwrap();

        // Unsubscribe
        let result = facade.unsubscribe(&channel, &"conn-123".to_string());
        assert!(result.is_ok());

        // Should have no connections
        let conns = facade.connections(&channel).unwrap();
        assert_eq!(conns.len(), 0);
    }
}
