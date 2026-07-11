//! Redis Pub/Sub for event-driven communication between services.
//!
//! This module is separate from the key-value [`Cache`](crate::Cache) layer.
//! Use it to publish and subscribe to domain events across microservices.
//!
//! # Example
//!
//! ```rust,no_run
//! use rf_cache::pubsub::RedisPubSub;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pubsub = RedisPubSub::new("redis://localhost:6379").await?;
//!
//! // Publish a domain event
//! pubsub.publish("myapp:orders:created", r#"{"id":"abc","total":42}"#).await?;
//!
//! // Subscribe to a channel
//! let mut rx = pubsub.subscribe("myapp:orders:created").await?;
//! while let Some(msg) = rx.recv().await {
//!     println!("[{}] {}", msg.channel, msg.payload);
//! }
//! # Ok(())
//! # }
//! ```

use crate::CacheError;
use futures::StreamExt;
use redis::{aio::ConnectionManager, AsyncCommands};
use tokio::sync::mpsc;

/// A message received from a Redis Pub/Sub channel.
#[derive(Debug, Clone)]
pub struct PubSubMessage {
    /// The channel the message was published to.
    pub channel: String,
    /// The message payload (typically serialised JSON).
    pub payload: String,
}

/// Redis-backed Pub/Sub client.
///
/// Clone-cheap: the internal [`ConnectionManager`] is already `Arc`-wrapped.
#[derive(Clone)]
pub struct RedisPubSub {
    redis_url: String,
    publisher: ConnectionManager,
}

impl RedisPubSub {
    /// Connect to Redis and return a new `RedisPubSub`.
    pub async fn new(redis_url: &str) -> Result<Self, CacheError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| CacheError::Backend(format!("Redis connect: {e}")))?;
        let publisher = client
            .get_connection_manager()
            .await
            .map_err(|e| CacheError::Backend(format!("Redis manager: {e}")))?;
        Ok(Self {
            redis_url: redis_url.to_string(),
            publisher,
        })
    }

    /// Publish `message` to `channel`.
    ///
    /// The message is broadcast to every active subscriber of that channel.
    pub async fn publish(&self, channel: &str, message: &str) -> Result<(), CacheError> {
        let mut conn = self.publisher.clone();
        conn.publish::<_, _, ()>(channel, message)
            .await
            .map_err(|e| CacheError::Backend(format!("Redis publish: {e}")))?;
        Ok(())
    }

    /// Subscribe to an exact channel name.
    ///
    /// Returns an [`mpsc::UnboundedReceiver`] that yields [`PubSubMessage`]s
    /// as they arrive. The background listener is dropped when the receiver
    /// is dropped.
    pub async fn subscribe(
        &self,
        channel: &str,
    ) -> Result<mpsc::UnboundedReceiver<PubSubMessage>, CacheError> {
        let mut pubsub = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| CacheError::Backend(format!("Redis connect: {e}")))?
            .get_async_pubsub()
            .await
            .map_err(|e| CacheError::Backend(format!("Redis subscribe conn: {e}")))?;

        pubsub
            .subscribe(channel)
            .await
            .map_err(|e| CacheError::Backend(format!("Redis subscribe: {e}")))?;

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = pubsub.into_on_message();
            while let Some(msg) = stream.next().await {
                let channel = msg.get_channel_name().to_string();
                if let Ok(payload) = msg.get_payload::<String>() {
                    if tx.send(PubSubMessage { channel, payload }).is_err() {
                        break;
                    }
                }
            }
        });
        Ok(rx)
    }

    /// Subscribe to a Redis glob pattern (e.g. `"myapp:orders:*"`).
    ///
    /// Every message whose channel matches the pattern is forwarded.
    /// The [`PubSubMessage::channel`] field contains the actual channel name.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_cache::pubsub::RedisPubSub;
    /// # async fn example(pubsub: RedisPubSub) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut rx = pubsub.psubscribe("wedding_quest:session:*").await?;
    /// while let Some(msg) = rx.recv().await {
    ///     println!("room {} → {}", msg.channel, msg.payload);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn psubscribe(
        &self,
        pattern: &str,
    ) -> Result<mpsc::UnboundedReceiver<PubSubMessage>, CacheError> {
        let mut pubsub = redis::Client::open(self.redis_url.as_str())
            .map_err(|e| CacheError::Backend(format!("Redis connect: {e}")))?
            .get_async_pubsub()
            .await
            .map_err(|e| CacheError::Backend(format!("Redis psubscribe conn: {e}")))?;

        pubsub
            .psubscribe(pattern)
            .await
            .map_err(|e| CacheError::Backend(format!("Redis psubscribe: {e}")))?;

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = pubsub.into_on_message();
            while let Some(msg) = stream.next().await {
                let channel = msg.get_channel_name().to_string();
                if let Ok(payload) = msg.get_payload::<String>() {
                    if tx.send(PubSubMessage { channel, payload }).is_err() {
                        break;
                    }
                }
            }
        });
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Return the Redis URL to use for live tests.
    ///
    /// Priority:
    ///   1. `REDIS_URL` environment variable (set by the live-cloud CI job from
    ///      the GitHub secret, or by the developer for any remote Redis).
    ///   2. Default local URL `redis://127.0.0.1:6379` (used by the
    ///      docker-compose test environment started with `scripts/test-env-up.sh`).
    fn redis_test_url() -> String {
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
    }

    /// Helper to check whether a live Redis is reachable for testing.
    ///
    /// Parses `host:port` from `redis_test_url()` and probes with a TCP
    /// connect. The live test SKIPS (prints a skip line and passes) when Redis
    /// is down, and runs a full round-trip when it is up. Bring the local
    /// service up with `scripts/test-env-up.sh` (redis on 6379 in
    /// `docker-compose.test.yml`), or set `REDIS_URL` to a remote instance.
    async fn redis_available() -> bool {
        let url = redis_test_url();
        // Strip the redis:// (or rediss://) scheme and any trailing path to get host:port.
        let hostport = url
            .trim_start_matches("rediss://")
            .trim_start_matches("redis://")
            .split('/')
            .next()
            .unwrap_or("127.0.0.1:6379");
        tokio::net::TcpStream::connect(hostport).await.is_ok()
    }

    #[tokio::test]
    async fn test_pubsub_publish_subscribe_roundtrip() {
        if !redis_available().await {
            eprintln!("Skipping test_pubsub_publish_subscribe_roundtrip: Redis not available");
            eprintln!("  Local:  Start services with: ./scripts/test-env-up.sh");
            eprintln!("  Cloud:  Set REDIS_URL=redis://<host>:6379 to test against a real instance");
            return;
        }

        let pubsub = RedisPubSub::new(&redis_test_url())
            .await
            .expect("connect to live redis");

        // Unique channel so parallel test runs never cross-talk.
        let channel = format!("rf_cache:test:pubsub:{}", std::process::id());
        let mut rx = pubsub.subscribe(&channel).await.expect("subscribe");

        // Give the background listener a moment to attach before publishing so
        // the message isn't emitted before the subscription is fully live.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let payload = r#"{"id":"abc","total":42}"#;
        pubsub.publish(&channel, payload).await.expect("publish");

        let msg = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for a pubsub message over live redis")
            .expect("pubsub channel closed before a message arrived");

        assert_eq!(msg.channel, channel);
        assert_eq!(msg.payload, payload);
    }
}
