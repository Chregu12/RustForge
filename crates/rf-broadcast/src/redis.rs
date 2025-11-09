//! Redis-backed broadcaster for distributed deployments

use crate::{Broadcaster, BroadcastError, Channel, ConnectionId, Event, PresenceInfo, UserId};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Redis-backed broadcaster
///
/// Uses Redis Pub/Sub to broadcast events across multiple servers.
/// Presence and subscription data is stored in Redis.
///
/// # Example
///
/// ```no_run
/// use rf_broadcast::{RedisBroadcaster, Broadcaster, Channel, SimpleEvent};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let broadcaster = RedisBroadcaster::new("redis://localhost").await?;
///
/// broadcaster.subscribe(
///     &Channel::public("users"),
///     "conn-123".to_string(),
///     None,
/// ).await?;
///
/// let event = SimpleEvent::new(
///     "user.created",
///     json!({"id": 123}),
///     vec![Channel::public("users")],
/// );
///
/// broadcaster.broadcast(&Channel::public("users"), &event).await?;
/// # Ok(())
/// # }
/// ```
pub struct RedisBroadcaster {
    pool: Pool,
    local_subscriptions: Arc<Mutex<HashMap<Channel, HashMap<ConnectionId, Option<UserId>>>>>,
}

impl RedisBroadcaster {
    /// Create new Redis broadcaster
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    pub async fn new(redis_url: &str) -> Result<Self, BroadcastError> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Test connection
        let mut conn = pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        Ok(Self {
            pool,
            local_subscriptions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get Redis key for channel subscriptions
    fn subscriptions_key(channel: &Channel) -> String {
        format!("broadcast:subscriptions:{}", channel.name())
    }

    /// Get Redis key for channel presence
    fn presence_key(channel: &Channel) -> String {
        format!("broadcast:presence:{}", channel.name())
    }

    /// Get Redis Pub/Sub channel name
    fn pubsub_channel(channel: &Channel) -> String {
        format!("broadcast:events:{}", channel.name())
    }
}

#[async_trait]
impl Broadcaster for RedisBroadcaster {
    async fn broadcast(&self, channel: &Channel, event: &dyn Event) -> Result<(), BroadcastError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Serialize event
        let event_data = serde_json::json!({
            "event": event.event_name(),
            "data": event.to_json()?,
        });

        let event_json = serde_json::to_string(&event_data)
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Publish to Redis Pub/Sub
        let _: () = conn
            .publish(Self::pubsub_channel(channel), event_json)
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        tracing::debug!(
            channel = %channel.name(),
            event = %event.event_name(),
            "Event published to Redis"
        );

        Ok(())
    }

    async fn subscribe(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> Result<(), BroadcastError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Add to local subscriptions
        {
            let mut subs = self.local_subscriptions.lock().await;
            subs.entry(channel.clone())
                .or_insert_with(HashMap::new)
                .insert(connection_id.clone(), user_id.clone());
        }

        // Add to Redis subscriptions set
        let _: () = conn
            .sadd(
                Self::subscriptions_key(channel),
                connection_id.as_str(),
            )
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Add to presence if presence channel
        if channel.is_presence() {
            if let Some(uid) = &user_id {
                let presence_data = serde_json::json!({
                    "user_id": uid,
                    "joined_at": chrono::Utc::now().to_rfc3339(),
                });

                let _: () = conn
                    .hset(
                        Self::presence_key(channel),
                        uid.as_str(),
                        serde_json::to_string(&presence_data).unwrap(),
                    )
                    .await
                    .map_err(|e| BroadcastError::BackendError(e.to_string()))?;
            }
        }

        tracing::debug!(
            channel = %channel.name(),
            connection_id = %connection_id,
            user_id = ?user_id,
            "Connection subscribed (Redis)"
        );

        Ok(())
    }

    async fn unsubscribe(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<(), BroadcastError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Get user_id from local subscriptions
        let user_id = {
            let mut subs = self.local_subscriptions.lock().await;
            if let Some(channel_subs) = subs.get_mut(channel) {
                channel_subs.remove(connection_id).flatten()
            } else {
                None
            }
        };

        // Remove from Redis subscriptions
        let _: () = conn
            .srem(Self::subscriptions_key(channel), connection_id.as_str())
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        // Remove from presence if presence channel
        if channel.is_presence() {
            if let Some(uid) = user_id {
                let _: () = conn
                    .hdel(Self::presence_key(channel), uid.as_str())
                    .await
                    .map_err(|e| BroadcastError::BackendError(e.to_string()))?;
            }
        }

        tracing::debug!(
            channel = %channel.name(),
            connection_id = %connection_id,
            "Connection unsubscribed (Redis)"
        );

        Ok(())
    }

    async fn connections(&self, channel: &Channel) -> Result<Vec<ConnectionId>, BroadcastError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        let connections: Vec<String> = conn
            .smembers(Self::subscriptions_key(channel))
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        Ok(connections)
    }

    async fn presence(&self, channel: &Channel) -> Result<Vec<PresenceInfo>, BroadcastError> {
        if !channel.is_presence() {
            return Err(BroadcastError::InvalidChannel(
                "Not a presence channel".into(),
            ));
        }

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        let presence_data: HashMap<String, String> = conn
            .hgetall(Self::presence_key(channel))
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        let mut presence_list = Vec::new();

        for (user_id, data_json) in presence_data {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&data_json) {
                if let Some(joined_str) = data.get("joined_at").and_then(|v| v.as_str()) {
                    if let Ok(joined_at) = chrono::DateTime::parse_from_rfc3339(joined_str) {
                        presence_list.push(PresenceInfo {
                            user_id,
                            user_info: None,
                            joined_at: joined_at.into(),
                        });
                    }
                }
            }
        }

        Ok(presence_list)
    }

    async fn is_subscribed(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> Result<bool, BroadcastError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        let is_member: bool = conn
            .sismember(Self::subscriptions_key(channel), connection_id.as_str())
            .await
            .map_err(|e| BroadcastError::BackendError(e.to_string()))?;

        Ok(is_member)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimpleEvent;
    use serde_json::json;

    // Note: These tests require a running Redis instance
    // Run with: docker run -d -p 6379:6379 redis

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_subscribe_unsubscribe() {
        let broadcaster = RedisBroadcaster::new("redis://localhost").await.unwrap();
        let channel = Channel::public("test");
        let conn_id = "conn-1".to_string();

        // Subscribe
        broadcaster
            .subscribe(&channel, conn_id.clone(), None)
            .await
            .unwrap();

        assert!(broadcaster.is_subscribed(&channel, &conn_id).await.unwrap());

        // Unsubscribe
        broadcaster.unsubscribe(&channel, &conn_id).await.unwrap();
        assert!(!broadcaster.is_subscribed(&channel, &conn_id).await.unwrap());
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_presence_channel() {
        let broadcaster = RedisBroadcaster::new("redis://localhost").await.unwrap();
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

        // Presence should be empty
        let members = broadcaster.presence(&channel).await.unwrap();
        assert_eq!(members.len(), 0);
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_broadcast() {
        let broadcaster = RedisBroadcaster::new("redis://localhost").await.unwrap();
        let channel = Channel::public("users");

        // Subscribe
        broadcaster
            .subscribe(&channel, "conn-1".to_string(), None)
            .await
            .unwrap();

        // Broadcast event
        let event = SimpleEvent::new("user.created", json!({"id": 123}), vec![channel.clone()]);

        let result = broadcaster.broadcast(&channel, &event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires Redis
    async fn test_redis_connections() {
        let broadcaster = RedisBroadcaster::new("redis://localhost").await.unwrap();
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
}
