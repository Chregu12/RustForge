//! Foundry Notifications - Multi-channel notification system
//!
//! # Features
//!
//! - **Multiple Channels**: Database, Email, SMS, Slack, Push
//! - **Notifiable Trait**: Apply to any model
//! - **Batch Notifications**: Send to multiple recipients
//! - **Channel Routing**: Configure per-notification
//!
//! # Example
//!
//! ```no_run
//! use foundry_notifications::prelude::*;
//!
//! # async fn example() -> Result<(), NotificationError> {
//! let notification = SimpleNotification::new(
//!     "Welcome!",
//!     "Thanks for signing up.",
//! );
//!
//! let mut manager = NotificationManager::new();
//! manager.send(&notification, &recipient).await?;
//! # Ok(())
//! # }
//! ```

pub mod notification;
pub mod channels;
pub mod notifiable;
pub mod manager;

pub use notification::{Notification, NotificationData, NotificationError};
pub use channels::{Channel, ChannelResult, DatabaseChannel, SlackChannel};
pub use notifiable::Notifiable;
pub use manager::NotificationManager;

pub mod prelude {
    pub use crate::notification::{Notification, NotificationData, SimpleNotification};
    pub use crate::channels::Channel;
    pub use crate::notifiable::Notifiable;
    pub use crate::manager::NotificationManager;
    pub use crate::NotificationError;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert!(true);
    }
}
