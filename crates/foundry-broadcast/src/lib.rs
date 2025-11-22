//! # Foundry Broadcasting
//!
//! WebSocket event broadcasting with presence channels.

pub mod broadcaster;
pub mod channels;
pub mod events;
pub mod presence;

pub use broadcaster::{BroadcastMessage, Broadcaster};
pub use channels::{Channel, PresenceChannel, PrivateChannel};
pub use events::BroadcastEvent;
pub use presence::PresenceTracker;

#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Send error: {0}")]
    SendError(String),
}

pub type Result<T> = std::result::Result<T, BroadcastError>;
