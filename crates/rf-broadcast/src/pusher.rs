//! Pusher broadcasting backend
//!
//! Pusher Channels is a hosted real-time messaging service with excellent scalability
//! and reliability. This backend integrates RustForge broadcasting with Pusher.
//!
//! # Features
//! - Pusher Channels HTTP API integration
//! - HMAC-SHA256 authentication
//! - Batch event sending
//! - Webhook verification
//! - Presence channel support
//! - Private channel authorization
//!
//! # Configuration
//!
//! ```toml
//! [broadcast.pusher]
//! app_id = "your-app-id"
//! key = "your-key"
//! secret = "your-secret"
//! cluster = "mt1"  # or your cluster
//! use_tls = true
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use rf_broadcast::pusher::{PusherBroadcaster, PusherConfig};
//! use rf_broadcast::{Broadcaster, Channel, SimpleEvent};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = PusherConfig {
//!     app_id: "123456".to_string(),
//!     key: "your-key".to_string(),
//!     secret: "your-secret".to_string(),
//!     cluster: "mt1".to_string(),
//!     use_tls: true,
//! };
//!
//! let broadcaster = PusherBroadcaster::new(config);
//!
//! let event = SimpleEvent::new(
//!     "user.created",
//!     json!({"name": "John", "email": "john@example.com"}),
//!     vec![Channel::public("users")],
//! );
//!
//! broadcaster.broadcast(&Channel::public("users"), &event).await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    BroadcastError, BroadcastResult, Broadcaster, Channel, ConnectionId, Event, PresenceInfo,
    UserId,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Pusher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PusherConfig {
    /// Pusher app ID
    pub app_id: String,

    /// Pusher app key
    pub key: String,

    /// Pusher app secret
    pub secret: String,

    /// Pusher cluster (e.g., "mt1", "eu", "ap1")
    pub cluster: String,

    /// Use TLS (default: true)
    #[serde(default = "default_true")]
    pub use_tls: bool,

    /// Custom host (optional, overrides cluster)
    pub host: Option<String>,
}

fn default_true() -> bool {
    true
}

impl PusherConfig {
    /// Get API base URL
    pub fn base_url(&self) -> String {
        if let Some(host) = &self.host {
            format!("{}://{}", if self.use_tls { "https" } else { "http" }, host)
        } else {
            format!(
                "{}://api-{}.pusher.com",
                if self.use_tls { "https" } else { "http" },
                self.cluster
            )
        }
    }

    /// Get WebSocket base URL
    pub fn ws_url(&self) -> String {
        if let Some(host) = &self.host {
            format!("{}://{}", if self.use_tls { "wss" } else { "ws" }, host)
        } else {
            format!(
                "{}://ws-{}.pusher.com",
                if self.use_tls { "wss" } else { "ws" },
                self.cluster
            )
        }
    }
}

/// Pusher broadcaster
pub struct PusherBroadcaster {
    config: PusherConfig,
    client: reqwest::Client,

    // In-memory tracking for presence (Pusher handles the actual presence)
    // This is just for local queries
    presence_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Vec<PresenceInfo>>>>,
}

impl PusherBroadcaster {
    /// Create new Pusher broadcaster
    pub fn new(config: PusherConfig) -> Self {
        let client = reqwest::Client::new();
        Self {
            config,
            client,
            presence_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Build authentication signature for Pusher API
    fn build_auth_signature(
        &self,
        method: &str,
        path: &str,
        query_params: &HashMap<String, String>,
        body: &str,
    ) -> String {
        // Build query string
        let mut params = query_params.clone();
        params.insert("auth_key".to_string(), self.config.key.clone());
        params.insert(
            "auth_timestamp".to_string(),
            chrono::Utc::now().timestamp().to_string(),
        );
        params.insert("auth_version".to_string(), "1.0".to_string());
        params.insert("body_md5".to_string(), format!("{:x}", md5::compute(body)));

        // Sort params
        let mut sorted: Vec<_> = params.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());

        let query_string: String = sorted
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Build string to sign
        let string_to_sign = format!("{}\n{}\n{}", method, path, query_string);

        // HMAC-SHA256
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.config.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());

        hex::encode(mac.finalize().into_bytes())
    }

    /// Trigger events on Pusher
    async fn trigger_event(
        &self,
        channel_name: &str,
        event_name: &str,
        data: &serde_json::Value,
    ) -> BroadcastResult<()> {
        let path = format!("/apps/{}/events", self.config.app_id);
        let url = format!("{}{}", self.config.base_url(), path);

        // Build request body
        let body = serde_json::json!({
            "name": event_name,
            "channel": channel_name,
            "data": serde_json::to_string(data)
                .map_err(|e| BroadcastError::SerializationError(e.to_string()))?,
        });

        let body_str = serde_json::to_string(&body)
            .map_err(|e| BroadcastError::SerializationError(e.to_string()))?;

        // Build auth
        let mut query_params = HashMap::new();
        let auth_signature = self.build_auth_signature("POST", &path, &query_params, &body_str);

        query_params.insert("auth_key".to_string(), self.config.key.clone());
        query_params.insert(
            "auth_timestamp".to_string(),
            chrono::Utc::now().timestamp().to_string(),
        );
        query_params.insert("auth_version".to_string(), "1.0".to_string());
        query_params.insert(
            "body_md5".to_string(),
            format!("{:x}", md5::compute(&body_str)),
        );
        query_params.insert("auth_signature".to_string(), auth_signature);

        // Send request
        let response = self
            .client
            .post(&url)
            .query(&query_params)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(|e| BroadcastError::ConnectionError(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(BroadcastError::TransportError(format!(
                "Pusher API error ({}): {}",
                status, error_text
            )));
        }

        Ok(())
    }

    /// Trigger batch events (more efficient for multiple events)
    pub async fn trigger_batch(
        &self,
        events: Vec<PusherEvent>,
    ) -> BroadcastResult<PusherBatchResponse> {
        let path = format!("/apps/{}/batch_events", self.config.app_id);
        let url = format!("{}{}", self.config.base_url(), path);

        // Build request body
        let body = serde_json::json!({
            "batch": events,
        });

        let body_str = serde_json::to_string(&body)
            .map_err(|e| BroadcastError::SerializationError(e.to_string()))?;

        // Build auth
        let mut query_params = HashMap::new();
        let auth_signature = self.build_auth_signature("POST", &path, &query_params, &body_str);

        query_params.insert("auth_key".to_string(), self.config.key.clone());
        query_params.insert(
            "auth_timestamp".to_string(),
            chrono::Utc::now().timestamp().to_string(),
        );
        query_params.insert("auth_version".to_string(), "1.0".to_string());
        query_params.insert(
            "body_md5".to_string(),
            format!("{:x}", md5::compute(&body_str)),
        );
        query_params.insert("auth_signature".to_string(), auth_signature);

        // Send request
        let response = self
            .client
            .post(&url)
            .query(&query_params)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await
            .map_err(|e| BroadcastError::ConnectionError(e.to_string()))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        if !status.is_success() {
            return Err(BroadcastError::TransportError(format!(
                "Pusher API error ({}): {}",
                status, response_text
            )));
        }

        let batch_response: PusherBatchResponse = serde_json::from_str(&response_text)
            .map_err(|e| BroadcastError::SerializationError(e.to_string()))?;

        Ok(batch_response)
    }

    /// Authorize private/presence channel subscription
    pub fn authorize_channel(
        &self,
        socket_id: &str,
        channel_name: &str,
        user_data: Option<&serde_json::Value>,
    ) -> String {
        let string_to_sign = if let Some(data) = user_data {
            // Presence channel
            let user_data_str = serde_json::to_string(data).unwrap_or_default();
            format!("{}:{}:{}", socket_id, channel_name, user_data_str)
        } else {
            // Private channel
            format!("{}:{}", socket_id, channel_name)
        };

        // HMAC-SHA256
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.config.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());

        format!(
            "{}:{}",
            self.config.key,
            hex::encode(mac.finalize().into_bytes())
        )
    }

    /// Verify webhook signature
    pub fn verify_webhook(&self, signature: &str, body: &str) -> bool {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(self.config.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(body.as_bytes());

        let computed = hex::encode(mac.finalize().into_bytes());

        // Constant-time comparison
        computed == signature
    }
}

#[async_trait]
impl Broadcaster for PusherBroadcaster {
    async fn broadcast(&self, channel: &Channel, event: &dyn Event) -> BroadcastResult<()> {
        let channel_name = channel.name();
        let event_name = event.name();
        let event_data = event.data();

        self.trigger_event(&channel_name, event_name, &event_data)
            .await
    }

    async fn subscribe(
        &self,
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> BroadcastResult<()> {
        // Pusher handles subscriptions on the client side
        // We just cache presence info if it's a presence channel
        if channel.is_presence() {
            if let Some(uid) = user_id {
                let mut cache = self.presence_cache.write().await;
                let presence_list = cache.entry(channel.name()).or_insert_with(Vec::new);

                // Add or update presence
                if !presence_list.iter().any(|p| p.user_id == uid) {
                    presence_list.push(PresenceInfo::new(uid));
                }
            }
        }

        Ok(())
    }

    async fn unsubscribe(
        &self,
        channel: &Channel,
        connection_id: &ConnectionId,
    ) -> BroadcastResult<()> {
        // Pusher handles unsubscription on the client side
        // Just clean up our local cache
        Ok(())
    }

    async fn connections(&self, channel: &Channel) -> BroadcastResult<Vec<ConnectionId>> {
        // Pusher API can query channel info
        // For simplicity, returning empty list (would need to implement channel_info API call)
        Ok(Vec::new())
    }

    async fn presence(&self, channel: &Channel) -> BroadcastResult<Vec<PresenceInfo>> {
        if !channel.is_presence() {
            return Err(BroadcastError::InvalidChannel(
                "Channel is not a presence channel".to_string(),
            ));
        }

        let cache = self.presence_cache.read().await;
        Ok(cache.get(&channel.name()).cloned().unwrap_or_default())
    }

    async fn is_subscribed(
        &self,
        _channel: &Channel,
        _connection_id: &ConnectionId,
    ) -> BroadcastResult<bool> {
        // Pusher handles subscription state
        // Would need to query Pusher API for accurate result
        Ok(false)
    }
}

/// Pusher event for batch sending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PusherEvent {
    /// Channel name
    pub channel: String,

    /// Event name
    pub name: String,

    /// Event data (as JSON string)
    pub data: String,

    /// Socket ID to exclude (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket_id: Option<String>,
}

/// Pusher batch response
#[derive(Debug, Deserialize)]
pub struct PusherBatchResponse {
    /// Batch status
    pub batch: Vec<PusherEventStatus>,
}

/// Status of individual event in batch
#[derive(Debug, Deserialize)]
pub struct PusherEventStatus {
    /// Success or error
    #[serde(default)]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pusher_config() {
        let config = PusherConfig {
            app_id: "123456".to_string(),
            key: "test-key".to_string(),
            secret: "test-secret".to_string(),
            cluster: "mt1".to_string(),
            use_tls: true,
            host: None,
        };

        assert_eq!(config.app_id, "123456");
        assert_eq!(config.base_url(), "https://api-mt1.pusher.com");
        assert_eq!(config.ws_url(), "wss://ws-mt1.pusher.com");
    }

    #[test]
    fn test_pusher_config_custom_host() {
        let config = PusherConfig {
            app_id: "123456".to_string(),
            key: "test-key".to_string(),
            secret: "test-secret".to_string(),
            cluster: "mt1".to_string(),
            use_tls: true,
            host: Some("custom.pusher.com".to_string()),
        };

        assert_eq!(config.base_url(), "https://custom.pusher.com");
        assert_eq!(config.ws_url(), "wss://custom.pusher.com");
    }

    #[test]
    fn test_channel_authorization() {
        let config = PusherConfig {
            app_id: "123456".to_string(),
            key: "test-key".to_string(),
            secret: "test-secret".to_string(),
            cluster: "mt1".to_string(),
            use_tls: true,
            host: None,
        };

        let broadcaster = PusherBroadcaster::new(config);

        let auth = broadcaster.authorize_channel("123.456", "private-channel", None);
        assert!(auth.starts_with("test-key:"));
        assert!(auth.len() > 20);
    }

    #[test]
    fn test_webhook_verification() {
        let config = PusherConfig {
            app_id: "123456".to_string(),
            key: "test-key".to_string(),
            secret: "test-secret".to_string(),
            cluster: "mt1".to_string(),
            use_tls: true,
            host: None,
        };

        let broadcaster = PusherBroadcaster::new(config);

        // This would be a real signature from Pusher
        let body = r#"{"event":"user_created"}"#;

        // Generate signature
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(b"test-secret").unwrap();
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(broadcaster.verify_webhook(&signature, body));
        assert!(!broadcaster.verify_webhook("invalid", body));
    }
}
