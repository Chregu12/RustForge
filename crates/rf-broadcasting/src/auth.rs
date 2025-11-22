//! Channel authorization for private and presence channels

use crate::{BroadcastError, BroadcastResult};
use async_trait::async_trait;

/// Channel types based on naming convention
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    /// Public channel (no authorization required)
    Public,
    /// Private channel (requires user authorization)
    Private,
    /// Presence channel (requires authorization + tracks online users)
    Presence,
}

impl ChannelType {
    /// Determine channel type from channel name
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("private-") {
            ChannelType::Private
        } else if name.starts_with("presence-") {
            ChannelType::Presence
        } else {
            ChannelType::Public
        }
    }

    /// Check if this channel type requires authorization
    pub fn requires_auth(&self) -> bool {
        matches!(self, ChannelType::Private | ChannelType::Presence)
    }
}

/// User information for channel authorization
pub trait User: Send + Sync {
    /// Get user ID
    fn id(&self) -> u64;

    /// Get user data for presence channels
    fn to_presence_data(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id(),
        })
    }
}

/// Simple user implementation for testing
#[derive(Debug, Clone)]
pub struct SimpleUser {
    pub id: u64,
    pub name: String,
}

impl User for SimpleUser {
    fn id(&self) -> u64 {
        self.id
    }

    fn to_presence_data(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
        })
    }
}

/// Trait for authorizing channel access
#[async_trait]
pub trait ChannelAuthorization: Send + Sync {
    /// Check if a user can access a channel
    async fn authorize(&self, user: &dyn User, channel: &str) -> bool;

    /// Get additional data for presence channels
    async fn presence_data(&self, user: &dyn User, channel: &str) -> Option<serde_json::Value> {
        if ChannelType::from_name(channel) == ChannelType::Presence {
            Some(user.to_presence_data())
        } else {
            None
        }
    }
}

/// Default channel authorization implementation
pub struct DefaultChannelAuth;

#[async_trait]
impl ChannelAuthorization for DefaultChannelAuth {
    async fn authorize(&self, user: &dyn User, channel: &str) -> bool {
        let channel_type = ChannelType::from_name(channel);

        match channel_type {
            ChannelType::Public => true,
            ChannelType::Private => {
                // Check if channel name matches user
                // Format: private-user.{id}
                if let Some(suffix) = channel.strip_prefix("private-user.") {
                    if let Ok(channel_user_id) = suffix.parse::<u64>() {
                        return user.id() == channel_user_id;
                    }
                }

                // Format: private-{anything} - allow by default
                // In production, you'd implement custom logic here
                true
            }
            ChannelType::Presence => {
                // Same logic as private channels
                // In production, you'd check room membership, etc.
                true
            }
        }
    }
}

/// Authorization token for channel access
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub user_id: u64,
    pub channel: String,
    pub timestamp: i64,
}

impl AuthToken {
    /// Create a new auth token
    pub fn new(user_id: u64, channel: String) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            user_id,
            channel,
            timestamp,
        }
    }

    /// Check if token is expired (1 hour validity)
    pub fn is_expired(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        now - self.timestamp > 3600 // 1 hour
    }

    /// Generate a signed token (simplified - use HMAC in production)
    pub fn sign(&self, secret: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.user_id.hash(&mut hasher);
        self.channel.hash(&mut hasher);
        self.timestamp.hash(&mut hasher);
        secret.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Verify a signed token
    pub fn verify(&self, signature: &str, secret: &str) -> bool {
        !self.is_expired() && self.sign(secret) == signature
    }
}

/// Helper to authorize channel subscriptions
pub async fn authorize_channel<A: ChannelAuthorization>(
    authorizer: &A,
    user: &dyn User,
    channel: &str,
    auth_token: Option<&str>,
) -> BroadcastResult<()> {
    let channel_type = ChannelType::from_name(channel);

    if !channel_type.requires_auth() {
        return Ok(());
    }

    // Verify auth token if provided
    if let Some(token) = auth_token {
        // In production, verify JWT or HMAC signature
        tracing::debug!(channel = %channel, "Verifying auth token");
        // For now, just check if token is not empty
        if token.is_empty() {
            return Err(BroadcastError::Unauthorized);
        }
    } else {
        return Err(BroadcastError::Unauthorized);
    }

    // Check authorization
    if !authorizer.authorize(user, channel).await {
        tracing::warn!(
            user_id = user.id(),
            channel = %channel,
            "Channel authorization failed"
        );
        return Err(BroadcastError::Unauthorized);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_detection() {
        assert_eq!(
            ChannelType::from_name("public-channel"),
            ChannelType::Public
        );
        assert_eq!(
            ChannelType::from_name("private-user.123"),
            ChannelType::Private
        );
        assert_eq!(
            ChannelType::from_name("presence-room.456"),
            ChannelType::Presence
        );
    }

    #[test]
    fn test_channel_requires_auth() {
        assert!(!ChannelType::Public.requires_auth());
        assert!(ChannelType::Private.requires_auth());
        assert!(ChannelType::Presence.requires_auth());
    }

    #[tokio::test]
    async fn test_default_authorization() {
        let auth = DefaultChannelAuth;
        let user = SimpleUser {
            id: 123,
            name: "Test User".to_string(),
        };

        // Public channel
        assert!(auth.authorize(&user, "public-channel").await);

        // Private channel with matching user ID
        assert!(auth.authorize(&user, "private-user.123").await);

        // Private channel with different user ID
        assert!(!auth.authorize(&user, "private-user.456").await);
    }

    #[test]
    fn test_auth_token() {
        let token = AuthToken::new(123, "private-channel".to_string());
        let secret = "secret-key";

        let signature = token.sign(secret);
        assert!(token.verify(&signature, secret));
        assert!(!token.verify("invalid", secret));
    }

    #[test]
    fn test_user_presence_data() {
        let user = SimpleUser {
            id: 123,
            name: "John Doe".to_string(),
        };

        let data = user.to_presence_data();
        assert_eq!(data["id"], 123);
        assert_eq!(data["name"], "John Doe");
    }
}
