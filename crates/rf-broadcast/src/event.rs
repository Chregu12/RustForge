//! Event types for broadcasting

use crate::{BroadcastError, Channel};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for broadcastable events
#[async_trait]
pub trait Event: Send + Sync {
    /// Event name (e.g., "user.created", "order.shipped")
    fn event_name(&self) -> &str;

    /// Serialize event to JSON
    fn to_json(&self) -> Result<String, BroadcastError>;

    /// Get channels to broadcast on
    fn broadcast_on(&self) -> Vec<Channel>;
}

/// Simple event implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleEvent {
    pub name: String,
    pub data: serde_json::Value,
    pub channels: Vec<Channel>,
}

impl SimpleEvent {
    /// Create new simple event
    pub fn new(
        name: impl Into<String>,
        data: serde_json::Value,
        channels: Vec<Channel>,
    ) -> Self {
        Self {
            name: name.into(),
            data,
            channels,
        }
    }
}

#[async_trait]
impl Event for SimpleEvent {
    fn event_name(&self) -> &str {
        &self.name
    }

    fn to_json(&self) -> Result<String, BroadcastError> {
        serde_json::to_string(&self.data)
            .map_err(|e| BroadcastError::SerializationError(e.to_string()))
    }

    fn broadcast_on(&self) -> Vec<Channel> {
        self.channels.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_event() {
        let event = SimpleEvent::new(
            "user.created",
            serde_json::json!({"id": 123}),
            vec![Channel::public("users")],
        );

        assert_eq!(event.event_name(), "user.created");
        assert_eq!(event.broadcast_on().len(), 1);
        assert!(event.to_json().is_ok());
    }
}
