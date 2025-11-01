use crate::notification::{Notification, NotificationError};
use async_trait::async_trait;

pub type ChannelResult = Result<(), NotificationError>;

/// Notification channel trait
#[async_trait]
pub trait Channel: Send + Sync {
    /// Get channel name
    fn name(&self) -> &str;

    /// Send notification through this channel
    async fn send(&self, notification: &dyn Notification, recipient: &dyn std::any::Any) -> ChannelResult;

    /// Test if channel is configured and ready
    async fn is_ready(&self) -> bool {
        true
    }
}
