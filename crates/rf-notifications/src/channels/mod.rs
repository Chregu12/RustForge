//! Notification channel implementations

use crate::{Notifiable, Notification, NotificationResult};
use async_trait::async_trait;

#[cfg(feature = "mail")]
pub mod mail;

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "sms")]
pub mod sms;

#[cfg(feature = "slack")]
pub mod slack;

// Re-exports
#[cfg(feature = "mail")]
pub use mail::MailChannel;

#[cfg(feature = "database")]
pub use database::DatabaseChannel;

#[cfg(feature = "sms")]
pub use sms::{MockSmsProvider, SmsChannel, SmsProvider, TwilioProvider};

#[cfg(feature = "slack")]
pub use slack::{MockSlackChannel, SlackChannel};

/// Channel handler trait
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// Send notification via this channel
    async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()>;
}
