//! In-memory WebSocket room registry for server-side broadcasting.
//!
//! A *room* is a named group of WebSocket connections (e.g. one per game session).
//! Use [`RoomRegistry`] to broadcast messages to every client in a room without
//! holding locks across `await` points.
//!
//! # Architecture
//!
//! Each connection receives a [`tokio::sync::mpsc::UnboundedSender`]. The registry
//! stores senders in a [`DashMap`]-based concurrent map, so `broadcast` never
//! blocks other callers.
//!
//! # Example
//!
//! ```rust,no_run
//! use rf_broadcast::room_registry::RoomRegistry;
//! use axum::extract::ws::Message;
//! use std::sync::Arc;
//! use tokio::sync::mpsc;
//!
//! # async fn example() {
//! let registry = Arc::new(RoomRegistry::new());
//!
//! // Inside a WebSocket handler:
//! let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
//! let conn_id = registry.join("session-abc", tx);
//!
//! // Broadcast from any task:
//! registry.broadcast("session-abc", r#"{"type":"ROUND_STARTED"}"#);
//!
//! // Cleanup on disconnect:
//! registry.leave("session-abc", conn_id);
//! # }
//! ```

use axum::extract::ws::Message;
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

type RoomId = String;
type ConnectionId = Uuid;

/// Concurrent, in-memory registry of WebSocket connections grouped by room.
///
/// Cheap to clone — all state lives behind an internal `Arc` inside each `DashMap`.
#[derive(Default)]
pub struct RoomRegistry {
    /// room_id → { connection_id → sender }
    rooms: DashMap<RoomId, DashMap<ConnectionId, UnboundedSender<Message>>>,
}

impl RoomRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a connection to a room. Returns the assigned [`ConnectionId`].
    ///
    /// The room is created automatically if it does not exist yet.
    pub fn join(&self, room_id: &str, sender: UnboundedSender<Message>) -> ConnectionId {
        let conn_id = Uuid::new_v4();
        self.rooms
            .entry(room_id.to_string())
            .or_default()
            .insert(conn_id, sender);
        tracing::debug!(room = %room_id, conn = %conn_id, size = %self.room_size(room_id), "connection joined");
        conn_id
    }

    /// Remove a connection from its room. Empty rooms are cleaned up.
    pub fn leave(&self, room_id: &str, conn_id: ConnectionId) {
        if let Some(room) = self.rooms.get(room_id) {
            room.remove(&conn_id);
            tracing::debug!(room = %room_id, conn = %conn_id, size = %room.len(), "connection left");
        }
        self.rooms.remove_if(room_id, |_, room| room.is_empty());
    }

    /// Broadcast a raw text message to every connection in `room_id`.
    ///
    /// Dead connections (closed senders) are removed automatically.
    pub fn broadcast(&self, room_id: &str, text: &str) {
        let msg = Message::Text(text.to_string().into());
        self.broadcast_message(room_id, msg);
    }

    /// Serialize `payload` as JSON and broadcast it to every connection in `room_id`.
    ///
    /// Returns `Err` only if JSON serialisation fails; individual send errors are
    /// silently ignored (dead connections are pruned).
    pub fn broadcast_json<T: Serialize>(
        &self,
        room_id: &str,
        payload: &T,
    ) -> Result<(), serde_json::Error> {
        let text = serde_json::to_string(payload)?;
        self.broadcast(room_id, &text);
        Ok(())
    }

    /// Send a message to a single connection inside `room_id`.
    pub fn send_to(&self, room_id: &str, conn_id: ConnectionId, message: Message) {
        if let Some(room) = self.rooms.get(room_id) {
            if let Some(sender) = room.get(&conn_id) {
                let _ = sender.send(message);
            }
        }
    }

    /// Return the number of active connections in `room_id`.
    pub fn room_size(&self, room_id: &str) -> usize {
        self.rooms.get(room_id).map(|r| r.len()).unwrap_or(0)
    }

    /// Return the total number of non-empty rooms.
    pub fn active_rooms(&self) -> usize {
        self.rooms.len()
    }

    // ── private helpers ──────────────────────────────────────────────────────

    fn broadcast_message(&self, room_id: &str, msg: Message) {
        if let Some(room) = self.rooms.get(room_id) {
            let dead: Vec<ConnectionId> = room
                .iter()
                .filter_map(|e| {
                    if e.value().send(msg.clone()).is_err() {
                        Some(*e.key())
                    } else {
                        None
                    }
                })
                .collect();
            for id in dead {
                room.remove(&id);
                tracing::debug!(room = %room_id, conn = %id, "pruned dead connection");
            }
        }
    }
}
