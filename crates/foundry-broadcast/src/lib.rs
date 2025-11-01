//! # Foundry Broadcasting
//!
//! WebSocket event broadcasting with presence channels.

pub mod broadcaster;
pub mod channels;
pub mod presence;
pub mod events;

pub use broadcaster::{Broadcaster, BroadcastMessage};
pub use channels::{Channel, PrivateChannel, PresenceChannel};
pub use presence::PresenceTracker;
pub use events::BroadcastEvent;

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
