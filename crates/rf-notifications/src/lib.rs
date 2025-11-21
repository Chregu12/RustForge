//! Multi-Channel Notifications System for RustForge
//!
//! This crate provides Laravel-style notification delivery across multiple channels:
//! - Email (via rf-mail)
//! - Database (via SeaORM)
//! - SMS (via Twilio or custom providers)
//! - Slack (via webhooks)
//!
//! # Quick Start
//!
//! ```rust
//! use rf_notifications::*;
//!
//! struct InvoicePaid {
//!     invoice_id: u64,
//!     amount: f64,
//! }
//!
//! #[async_trait::async_trait]
//! impl Notification for InvoicePaid {
//!     fn via(&self) -> Vec<Channel> {
//!         vec![Channel::Mail, Channel::Database]
//!     }
//!
//!     async fn to_mail(&self) -> Option<MailMessage> {
//!         Some(
//!             MailMessage::new()
//!                 .subject("Invoice Paid")
//!                 .greeting("Hello!")
//!                 .line(format!("Your invoice #{} has been paid.", self.invoice_id))
//!                 .line(format!("Amount: ${:.2}", self.amount))
//!                 .action("View Invoice", format!("/invoices/{}", self.invoice_id))
//!         )
//!     }
//!
//!     async fn to_database(&self) -> Option<DatabaseNotification> {
//!         Some(DatabaseNotification {
//!             title: "Invoice Paid".to_string(),
//!             message: format!("Invoice #{} has been paid", self.invoice_id),
//!             data: serde_json::json!({
//!                 "invoice_id": self.invoice_id,
//!                 "amount": self.amount,
//!             }),
//!         })
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Setup notifier
//! let mut notifier = Notifier::new();
//!
//! // Send notification
//! // user.notify(InvoicePaid { invoice_id: 1, amount: 99.99 }, &notifier).await?;
//! # Ok(())
//! # }
//! ```

pub mod channels;
pub mod messages;
pub mod notifier;

#[cfg(feature = "database")]
pub mod entities;

pub mod examples;

use async_trait::async_trait;
pub use channels::*;
pub use messages::*;
pub use notifier::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Notification errors
#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Routing error: {0}")]
    RoutingError(String),

    #[error("Send error: {0}")]
    SendError(String),

    #[cfg(feature = "mail")]
    #[error("Mail error: {0}")]
    MailError(#[from] rf_mail::MailError),

    #[cfg(feature = "database")]
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[cfg(any(feature = "sms", feature = "slack"))]
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type NotificationResult<T> = Result<T, NotificationError>;

/// Notification channels
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    Mail,
    Database,
    Sms,
    Slack,
    Custom(String),
}

/// Core notification trait - implement this for your notifications
#[async_trait]
pub trait Notification: Send + Sync {
    /// Determine which channels to use
    fn via(&self) -> Vec<Channel>;

    /// Convert to mail message (Laravel-style)
    async fn to_mail(&self) -> Option<MailMessage> {
        None
    }

    /// Convert to database notification
    async fn to_database(&self) -> Option<DatabaseNotification> {
        None
    }

    /// Convert to SMS message
    async fn to_sms(&self) -> Option<SmsMessage> {
        None
    }

    /// Convert to Slack message
    async fn to_slack(&self) -> Option<SlackMessage> {
        None
    }

    /// Should this notification be queued?
    fn should_queue(&self) -> bool {
        false
    }
}

/// Notifiable entity trait - implement this for models like User
pub trait Notifiable: Send + Sync {
    /// Email routing
    fn route_notification_for_mail(&self) -> Option<String> {
        None
    }

    /// Database routing (returns user ID)
    fn route_notification_for_database(&self) -> Option<i64> {
        None
    }

    /// SMS routing (returns phone number)
    fn route_notification_for_sms(&self) -> Option<String> {
        None
    }

    /// Slack routing (returns webhook URL or channel)
    fn route_notification_for_slack(&self) -> Option<String> {
        None
    }
}

/// Extension trait to add notify method to Notifiable types
#[async_trait]
pub trait NotifiableExt: Notifiable + Sized {
    /// Send a notification to this notifiable entity
    async fn notify<N: Notification>(
        &self,
        notification: N,
        notifier: &Notifier,
    ) -> NotificationResult<()>
    where
        Self: Notifiable,
    {
        notifier.send(&notification, self as &dyn Notifiable).await
    }
}

// Blanket implementation for all Notifiable types
impl<T: Notifiable> NotifiableExt for T {}

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::channels::*;
    pub use crate::messages::*;
    pub use crate::notifier::*;
    pub use crate::{
        Channel, Notifiable, NotifiableExt, Notification, NotificationError, NotificationResult,
    };

    #[cfg(feature = "database")]
    pub use crate::entities;
}
