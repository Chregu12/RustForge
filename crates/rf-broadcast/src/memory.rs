//! In-memory broadcaster for development and testing

use crate::{BroadcastError, Broadcaster, Channel, ConnectionId, Event, PresenceInfo, UserId};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Broadcast message sent through channel
#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    pub channel: Channel,
    pub event_name: String,
    pub data: String,
    pub connections: Vec<ConnectionId>,
}

/// In-memory broadcaster
///
/// Stores subscriptions and presence in memory. Suitable for development
/// and single-server deployments.
///
/// # Example
///
/// ```
/// use rf_broadcast::{MemoryBroadcaster, Broadcaster, Channel, SimpleEvent};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let broadcaster = MemoryBroadcaster::new();
///
/// // Subscribe connection
/// broadcaster.subscribe(
///     &Channel::public("users"),
///     "conn-123".to_string(),
///     None,
/// ).await?;
///
/// // Broadcast event
/// let event = SimpleEvent::new(
///     "user.created",
///     json!({"id": 123}),
///     vec![Channel::public("users")],
/// );
/// broadcaster.broadcast(&Channel::public("users"), &event).await?;
/// # Ok(())
/// # }
/// ```
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

impl MemoryBroadcaster {
    /// Create new memory broadcaster
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
    ///
    /// Each WebSocket connection should get its own receiver to listen
    /// for broadcast events.
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.sender.subscribe()
    }

    // ---- Synchronous core ------------------------------------------------
    //
    // Every `MemoryBroadcaster` operation is genuinely synchronous under the
    // hood: it locks an in-memory `Mutex` and (for broadcast) pushes onto a
    // `tokio::sync::broadcast` channel — none of it ever yields to an executor.
    // The `#[async_trait] impl Broadcaster` below is just an async veneer that
    // delegates to these inherent methods.
    //
    // The process-global `Broadcast` facade calls these directly so it needs NO
    // ambient runtime and — crucially — does NOT panic with "Cannot start a
    // runtime from within a runtime" when invoked from inside a Tokio context
    // (an Axum handler, or a background job body running on a worker). That is
    // exactly the call a MediaFlow job makes after finishing an upload.

    /// Broadcast an event to a channel (synchronous core; see [`Broadcaster::broadcast`]).
    pub fn broadcast_now(&self, channel: &Channel, event: &dyn Event) -> Result<(), BroadcastError> {
        let connections = {
            let subs = self.subscriptions.lock().unwrap();
            subs.get(channel)
                .map(|s| s.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
        };

        let message = BroadcastMessage {
            channel: channel.clone(),
            event_name: event.event_name().to_string(),
            data: event.to_json()?,
            connections: connections.clone(),
        };

        // Ignore send errors (no receivers is ok).
        let _ = self.sender.send(message);

        tracing::debug!(
            channel = %channel.name(),
            event = %event.event_name(),
            connections = connections.len(),
            "Event broadcasted"
        );

        Ok(())
    }

    /// Subscribe a connection to a channel (synchronous core; see [`Broadcaster::subscribe`]).
    pub fn subscribe_now(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> Result<(), BroadcastError> {
        {
            let mut subs = self.subscriptions.lock().unwrap();
            subs.entry(channel.clone())
                .or_default()
                .insert(connection_id.clone());
        }

        {
            let mut conns = self.connections.lock().unwrap();
            conns.insert(connection_id.clone(), user_id.clone());
        }

        if channel.is_presence() {
            if let Some(ref uid) = user_id {
                let mut pres = self.presence.lock().unwrap();
                pres.entry(channel.clone())
                    .or_default()
                    .insert(uid.clone(), PresenceInfo::new(uid.clone()));
            }
        }

        tracing::debug!(
            channel = %channel.name(),
            connection_id = %connection_id,
            user_id = ?user_id,
            "Connection subscribed"
        );

        Ok(())
    }

    /// Unsubscribe a connection from a channel (synchronous core; see [`Broadcaster::unsubscribe`]).
    pub fn unsubscribe_now(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<(), BroadcastError> {
        let user_id = {
            let conns = self.connections.lock().unwrap();
            conns.get(connection_id).cloned().flatten()
        };

        {
            let mut subs = self.subscriptions.lock().unwrap();
            if let Some(conns) = subs.get_mut(channel) {
                conns.remove(connection_id);
            }
        }

        if channel.is_presence() {
            if let Some(uid) = user_id {
                let mut pres = self.presence.lock().unwrap();
                if let Some(channel_pres) = pres.get_mut(channel) {
                    channel_pres.remove(&uid);
                }
            }
        }

        tracing::debug!(
            channel = %channel.name(),
            connection_id = %connection_id,
            "Connection unsubscribed"
        );

        Ok(())
    }

    /// List the connections subscribed to a channel (synchronous core).
    pub fn connections_now(&self, channel: &Channel) -> Result<Vec<ConnectionId>, BroadcastError> {
        let subs = self.subscriptions.lock().unwrap();
        Ok(subs
            .get(channel)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default())
    }

    /// List presence info for a presence channel (synchronous core).
    pub fn presence_now(&self, channel: &Channel) -> Result<Vec<PresenceInfo>, BroadcastError> {
        if !channel.is_presence() {
            return Err(BroadcastError::InvalidChannel(
                "Not a presence channel".into(),
            ));
        }

        let pres = self.presence.lock().unwrap();
        Ok(pres
            .get(channel)
            .map(|p| p.values().cloned().collect())
            .unwrap_or_default())
    }

    /// Check whether a connection is subscribed to a channel (synchronous core).
    pub fn is_subscribed_now(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<bool, BroadcastError> {
        let subs = self.subscriptions.lock().unwrap();
        Ok(subs
            .get(channel)
            .map(|s| s.contains(connection_id))
            .unwrap_or(false))
    }

    /// Get number of subscriptions (for testing)
    #[cfg(test)]
    pub fn subscription_count(&self, channel: &Channel) -> usize {
        let subs = self.subscriptions.lock().unwrap();
        subs.get(channel).map(|s| s.len()).unwrap_or(0)
    }

    /// Clear all subscriptions (for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.subscriptions.lock().unwrap().clear();
        self.presence.lock().unwrap().clear();
        self.connections.lock().unwrap().clear();
    }
}

impl Default for MemoryBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

// The async `Broadcaster` impl is a thin veneer over the synchronous inherent
// methods above — every operation is non-blocking in-memory work, so there is no
// real awaiting to do.
#[async_trait]
impl Broadcaster for MemoryBroadcaster {
    async fn broadcast(&self, channel: &Channel, event: &dyn Event) -> Result<(), BroadcastError> {
        self.broadcast_now(channel, event)
    }

    async fn subscribe(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> Result<(), BroadcastError> {
        self.subscribe_now(channel, connection_id, user_id)
    }

    async fn unsubscribe(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<(), BroadcastError> {
        self.unsubscribe_now(channel, connection_id)
    }

    async fn connections(&self, channel: &Channel) -> Result<Vec<ConnectionId>, BroadcastError> {
        self.connections_now(channel)
    }

    async fn presence(&self, channel: &Channel) -> Result<Vec<PresenceInfo>, BroadcastError> {
        self.presence_now(channel)
    }

    async fn is_subscribed(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<bool, BroadcastError> {
        self.is_subscribed_now(channel, connection_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimpleEvent;
    use serde_json::json;

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::public("test");
        let conn_id = "conn-1".to_string();

        // Subscribe
        broadcaster
            .subscribe(&channel, conn_id.clone(), None)
            .await
            .unwrap();

        assert!(broadcaster.is_subscribed(&channel, &conn_id).await.unwrap());
        assert_eq!(broadcaster.subscription_count(&channel), 1);

        // Unsubscribe
        broadcaster.unsubscribe(&channel, &conn_id).await.unwrap();
        assert!(!broadcaster.is_subscribed(&channel, &conn_id).await.unwrap());
        assert_eq!(broadcaster.subscription_count(&channel), 0);
    }

    #[tokio::test]
    async fn test_broadcast_event() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::public("users");

        // Subscribe connection
        broadcaster
            .subscribe(&channel, "conn-1".to_string(), None)
            .await
            .unwrap();

        // Create event
        let event = SimpleEvent::new("user.created", json!({"id": 123}), vec![channel.clone()]);

        // Broadcast
        let result = broadcaster.broadcast(&channel, &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_presence_channel() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::presence("chat");

        // Subscribe user
        broadcaster
            .subscribe(&channel, "conn-1".to_string(), Some("user-123".to_string()))
            .await
            .unwrap();

        // Check presence
        let members = broadcaster.presence(&channel).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, "user-123");

        // Unsubscribe
        broadcaster
            .unsubscribe(&channel, &"conn-1".to_string())
            .await
            .unwrap();

        // Check presence again
        let members = broadcaster.presence(&channel).await.unwrap();
        assert_eq!(members.len(), 0);
    }

    #[tokio::test]
    async fn test_presence_on_non_presence_channel() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::public("test");

        let result = broadcaster.presence(&channel).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::public("test");

        broadcaster
            .subscribe(&channel, "conn-1".to_string(), None)
            .await
            .unwrap();
        broadcaster
            .subscribe(&channel, "conn-2".to_string(), None)
            .await
            .unwrap();

        let connections = broadcaster.connections(&channel).await.unwrap();
        assert_eq!(connections.len(), 2);

        broadcaster
            .unsubscribe(&channel, &"conn-1".to_string())
            .await
            .unwrap();

        let connections = broadcaster.connections(&channel).await.unwrap();
        assert_eq!(connections.len(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let broadcaster = MemoryBroadcaster::new();
        let channel = Channel::public("test");

        broadcaster
            .subscribe(&channel, "conn-1".to_string(), None)
            .await
            .unwrap();

        assert_eq!(broadcaster.subscription_count(&channel), 1);

        broadcaster.clear();

        assert_eq!(broadcaster.subscription_count(&channel), 0);
    }
}
