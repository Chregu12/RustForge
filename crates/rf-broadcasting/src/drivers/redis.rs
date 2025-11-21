//! Redis Pub/Sub broadcast driver

use crate::{BroadcastDriver, BroadcastError, BroadcastResult, ServerMessage};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use futures::StreamExt;
use redis::AsyncCommands;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Redis broadcast driver configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis URL (e.g., "redis://localhost:6379")
    pub url: String,
    /// Maximum pool size
    pub pool_size: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 10,
        }
    }
}

/// Redis broadcast driver using Pub/Sub
pub struct RedisBroadcastDriver {
    pool: Pool,
}

impl RedisBroadcastDriver {
    /// Create a new Redis broadcast driver
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Create a Redis driver from config
    pub fn from_config(config: RedisConfig) -> BroadcastResult<Self> {
        let cfg = Config::from_url(config.url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| BroadcastError::Other(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Create a Redis driver from URL
    pub fn from_url(url: &str) -> BroadcastResult<Self> {
        Self::from_config(RedisConfig {
            url: url.to_string(),
            ..Default::default()
        })
    }
}

#[async_trait]
impl BroadcastDriver for RedisBroadcastDriver {
    async fn broadcast(
        &self,
        channels: &[String],
        event: &str,
        data: serde_json::Value,
    ) -> BroadcastResult<()> {
        let mut conn = self.pool.get().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to get Redis connection");
            BroadcastError::Other(e.to_string())
        })?;

        let message = json!({
            "event": event,
            "data": data,
        });

        let message_str = serde_json::to_string(&message)?;

        for channel in channels {
            tracing::debug!(channel = %channel, event = %event, "Publishing to Redis");

            conn.publish::<_, _, ()>(channel, &message_str)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, channel = %channel, "Failed to publish");
                    BroadcastError::Redis(e)
                })?;
        }

        Ok(())
    }

    async fn subscribe(&self, _channels: &[String]) -> BroadcastResult<()> {
        // Note: For deadpool-redis, subscription is handled in RedisSubscriber
        // This is because pub/sub requires a dedicated connection
        tracing::debug!("Subscribe called (handled by RedisSubscriber)");
        Ok(())
    }

    async fn unsubscribe(&self, _channels: &[String]) -> BroadcastResult<()> {
        // Note: For deadpool-redis, unsubscription is handled in RedisSubscriber
        tracing::debug!("Unsubscribe called (handled by RedisSubscriber)");
        Ok(())
    }
}

/// Redis subscriber that forwards messages to WebSocket clients
pub struct RedisSubscriber {
    redis_url: String,
    subscribed_channels: Arc<RwLock<Vec<String>>>,
}

impl RedisSubscriber {
    /// Create a new Redis subscriber
    pub fn new(redis_url: String) -> Self {
        Self {
            redis_url,
            subscribed_channels: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create from pool (extracts URL from config)
    pub fn from_pool(_pool: &Pool) -> BroadcastResult<Self> {
        // For now, we'll require the URL to be passed explicitly
        // In production, you'd extract it from the pool's configuration
        Err(BroadcastError::Other(
            "Use RedisSubscriber::new(url) instead".to_string(),
        ))
    }

    /// Start subscribing to Redis channels and forward to WebSocket registry
    pub async fn start<F>(
        self,
        channels: Vec<String>,
        mut callback: F,
    ) -> BroadcastResult<()>
    where
        F: FnMut(String, String) + Send + 'static,
    {
        // Get a dedicated connection for pub/sub
        let client = redis::Client::open(self.redis_url.clone())
            .map_err(|e| BroadcastError::Redis(e))?;

        let conn = client
            .get_async_connection()
            .await
            .map_err(|e| BroadcastError::Redis(e))?;

        let mut pubsub = conn.into_pubsub();

        // Subscribe to all channels
        for channel in &channels {
            pubsub.subscribe(channel).await?;
            tracing::info!(channel = %channel, "Redis subscriber: subscribed");
        }

        *self.subscribed_channels.write().await = channels;

        // Listen for messages
        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(msg) => {
                    let channel_name = msg.get_channel_name().to_string();
                    let payload: String = match msg.get_payload() {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to parse Redis message");
                            continue;
                        }
                    };

                    tracing::debug!(
                        channel = %channel_name,
                        payload = %payload,
                        "Received Redis message"
                    );

                    // Parse the message and convert to ServerMessage
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&payload) {
                        let event = data["event"].as_str().unwrap_or("message").to_string();
                        let event_data = data["data"].clone();

                        let server_message = ServerMessage::Event {
                            channel: channel_name.clone(),
                            event,
                            data: event_data,
                        };

                        if let Ok(message_json) = server_message.to_json() {
                            callback(channel_name, message_json);
                        }
                    }
                }
                None => {
                    tracing::warn!("Redis subscription ended");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get list of subscribed channels
    pub async fn channels(&self) -> Vec<String> {
        self.subscribed_channels.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to check if Redis is available for testing
    async fn redis_available() -> bool {
        tokio::net::TcpStream::connect("127.0.0.1:6379")
            .await
            .is_ok()
    }

    #[tokio::test]
async fn test_redis_broadcast() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_broadcast: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
        let driver = RedisBroadcastDriver::from_url("redis://localhost:6379").unwrap();

        let channels = vec!["test-channel".to_string()];
        let event = "TestEvent";
        let data = json!({"message": "Hello"});

        driver.broadcast(&channels, event, data).await.unwrap();
    }

    #[tokio::test]
async fn test_redis_subscriber() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_subscriber: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
        let subscriber = RedisSubscriber::new("redis://localhost:6379".to_string());

        // This would run indefinitely, so we just test creation
        assert_eq!(subscriber.channels().await.len(), 0);
    }
}
