//! # rf-broadcasting Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-broadcasting.
//!
//! ## Usage
//!
//! ```rust
//! use rf_broadcasting::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: auth::{ChannelAuthorization, ChannelType};
pub use crate:: drivers::redis::RedisBroadcastDriver;
pub use crate:: websocket::{WebSocketConfig, WebSocketServer};
