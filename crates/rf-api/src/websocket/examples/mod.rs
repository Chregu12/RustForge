//! WebSocket Examples
//!
//! Praktische Beispiele für WebSocket-Anwendungsfälle.

pub mod chat;
pub mod live_updates;

pub use chat::{ChatMessage, ChatService};
pub use live_updates::{
    DashboardMetrics, DashboardService, LiveUpdate, LiveUpdateService, UpdateAction,
};
