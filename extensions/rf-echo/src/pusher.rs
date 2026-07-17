//! Pusher protocol implementation

use serde::{Deserialize, Serialize};

/// Pusher protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PusherMessage {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub data: String,
}

impl PusherMessage {
    /// Create a new message
    pub fn new(event: impl Into<String>, data: impl Serialize) -> Self {
        Self {
            event: event.into(),
            channel: None,
            data: serde_json::to_string(&data).unwrap_or_default(),
        }
    }

    /// Create a message for a channel
    pub fn for_channel(
        event: impl Into<String>,
        channel: impl Into<String>,
        data: impl Serialize,
    ) -> Self {
        Self {
            event: event.into(),
            channel: Some(channel.into()),
            data: serde_json::to_string(&data).unwrap_or_default(),
        }
    }
}

/// Pusher protocol handler
pub struct PusherProtocol;

impl PusherProtocol {
    /// Create a subscribe message
    pub fn subscribe(channel: &str) -> PusherMessage {
        PusherMessage {
            event: "pusher:subscribe".to_string(),
            channel: None,
            data: serde_json::json!({
                "channel": channel
            })
            .to_string(),
        }
    }

    /// Create a subscribe message for private channel
    pub fn subscribe_private(channel: &str, auth: &str) -> PusherMessage {
        PusherMessage {
            event: "pusher:subscribe".to_string(),
            channel: None,
            data: serde_json::json!({
                "channel": channel,
                "auth": auth
            })
            .to_string(),
        }
    }

    /// Create a subscribe message for presence channel
    pub fn subscribe_presence(channel: &str, auth: &str, channel_data: &str) -> PusherMessage {
        PusherMessage {
            event: "pusher:subscribe".to_string(),
            channel: None,
            data: serde_json::json!({
                "channel": channel,
                "auth": auth,
                "channel_data": channel_data
            })
            .to_string(),
        }
    }

    /// Create an unsubscribe message
    pub fn unsubscribe(channel: &str) -> PusherMessage {
        PusherMessage {
            event: "pusher:unsubscribe".to_string(),
            channel: None,
            data: serde_json::json!({
                "channel": channel
            })
            .to_string(),
        }
    }

    /// Create a ping message
    pub fn ping() -> PusherMessage {
        PusherMessage {
            event: "pusher:ping".to_string(),
            channel: None,
            data: "{}".to_string(),
        }
    }

    /// Create a pong message
    pub fn pong() -> PusherMessage {
        PusherMessage {
            event: "pusher:pong".to_string(),
            channel: None,
            data: "{}".to_string(),
        }
    }

    /// Create a client event message
    pub fn client_event(channel: &str, event: &str, data: impl Serialize) -> PusherMessage {
        PusherMessage {
            event: format!("client-{}", event),
            channel: Some(channel.to_string()),
            data: serde_json::to_string(&data).unwrap_or_default(),
        }
    }
}

/// Pusher event types
pub mod events {
    pub const CONNECTION_ESTABLISHED: &str = "pusher:connection_established";
    pub const SUBSCRIPTION_SUCCEEDED: &str = "pusher:subscription_succeeded";
    pub const SUBSCRIPTION_ERROR: &str = "pusher:subscription_error";
    pub const MEMBER_ADDED: &str = "pusher:member_added";
    pub const MEMBER_REMOVED: &str = "pusher:member_removed";
    pub const PING: &str = "pusher:ping";
    pub const PONG: &str = "pusher:pong";
    pub const ERROR: &str = "pusher:error";
}

/// Connection established data
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionEstablished {
    pub socket_id: String,
    pub activity_timeout: Option<u32>,
}

/// Subscription succeeded data for presence channels
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionSucceeded {
    pub presence: Option<PresenceData>,
}

/// Presence data from subscription
#[derive(Debug, Clone, Deserialize)]
pub struct PresenceData {
    pub count: usize,
    pub ids: Vec<String>,
    pub hash: std::collections::HashMap<String, serde_json::Value>,
}

/// Subscription error data
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: String,
    pub status: Option<u16>,
}

/// Pusher error data
#[derive(Debug, Clone, Deserialize)]
pub struct PusherError {
    pub message: String,
    pub code: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_message() {
        let msg = PusherProtocol::subscribe("my-channel");
        assert_eq!(msg.event, "pusher:subscribe");
        assert!(msg.data.contains("my-channel"));
    }

    #[test]
    fn test_subscribe_private_message() {
        let msg = PusherProtocol::subscribe_private("private-channel", "auth-key");
        assert_eq!(msg.event, "pusher:subscribe");
        assert!(msg.data.contains("private-channel"));
        assert!(msg.data.contains("auth-key"));
    }

    #[test]
    fn test_client_event() {
        let msg = PusherProtocol::client_event("channel", "typing", serde_json::json!({}));
        assert_eq!(msg.event, "client-typing");
        assert_eq!(msg.channel, Some("channel".to_string()));
    }

    #[test]
    fn test_ping_pong() {
        let ping = PusherProtocol::ping();
        assert_eq!(ping.event, "pusher:ping");

        let pong = PusherProtocol::pong();
        assert_eq!(pong.event, "pusher:pong");
    }
}
