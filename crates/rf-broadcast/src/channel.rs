//! Channel types for broadcasting

use serde::{Deserialize, Serialize};

/// Channel type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Public channel - anyone can subscribe
    Public(String),

    /// Private channel - requires authentication
    Private(String),

    /// Presence channel - tracks who's subscribed
    Presence(String),
}

impl Channel {
    /// Create public channel
    pub fn public(name: impl Into<String>) -> Self {
        Self::Public(name.into())
    }

    /// Create private channel
    pub fn private(name: impl Into<String>) -> Self {
        Self::Private(name.into())
    }

    /// Create presence channel
    pub fn presence(name: impl Into<String>) -> Self {
        Self::Presence(name.into())
    }

    /// Get channel name
    pub fn name(&self) -> &str {
        match self {
            Channel::Public(name) => name,
            Channel::Private(name) => name,
            Channel::Presence(name) => name,
        }
    }

    /// Check if channel requires authentication
    pub fn requires_auth(&self) -> bool {
        matches!(self, Channel::Private(_) | Channel::Presence(_))
    }

    /// Check if channel is a presence channel
    pub fn is_presence(&self) -> bool {
        matches!(self, Channel::Presence(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let public = Channel::public("users");
        assert_eq!(public.name(), "users");
        assert!(!public.requires_auth());
        assert!(!public.is_presence());

        let private = Channel::private("orders");
        assert_eq!(private.name(), "orders");
        assert!(private.requires_auth());
        assert!(!private.is_presence());

        let presence = Channel::presence("chat");
        assert_eq!(presence.name(), "chat");
        assert!(presence.requires_auth());
        assert!(presence.is_presence());
    }
}
