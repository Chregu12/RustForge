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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use tokio::sync::mpsc;

    fn text_of(msg: Message) -> String {
        match msg {
            Message::Text(t) => t.to_string(),
            other => panic!("expected text message, got {other:?}"),
        }
    }

    #[test]
    fn join_tracks_room_size_and_active_rooms() {
        let registry = RoomRegistry::new();
        assert_eq!(registry.active_rooms(), 0);

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let id1 = registry.join("room-a", tx1);
        let id2 = registry.join("room-a", tx2);

        assert_ne!(id1, id2, "each connection gets a unique id");
        assert_eq!(registry.room_size("room-a"), 2);
        assert_eq!(registry.active_rooms(), 1);
        assert_eq!(registry.room_size("missing"), 0);
    }

    #[test]
    fn leave_prunes_empty_rooms() {
        let registry = RoomRegistry::new();
        let (tx1, _rx1) = mpsc::unbounded_channel();
        let (tx2, _rx2) = mpsc::unbounded_channel();
        let id1 = registry.join("room-a", tx1);
        let id2 = registry.join("room-a", tx2);

        registry.leave("room-a", id1);
        assert_eq!(registry.room_size("room-a"), 1);
        assert_eq!(registry.active_rooms(), 1);

        registry.leave("room-a", id2);
        assert_eq!(registry.room_size("room-a"), 0);
        assert_eq!(registry.active_rooms(), 0, "empty room is removed");
    }

    #[test]
    fn broadcast_delivers_to_every_connection() {
        let registry = RoomRegistry::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        registry.join("room-a", tx1);
        registry.join("room-a", tx2);

        registry.broadcast("room-a", "hello");

        assert_eq!(text_of(rx1.try_recv().unwrap()), "hello");
        assert_eq!(text_of(rx2.try_recv().unwrap()), "hello");
        // Broadcasting to an unknown room is a no-op (must not panic).
        registry.broadcast("nope", "ignored");
    }

    #[test]
    fn broadcast_prunes_dead_connections() {
        let registry = RoomRegistry::new();
        let (tx_live, mut rx_live) = mpsc::unbounded_channel();
        let (tx_dead, rx_dead) = mpsc::unbounded_channel();
        registry.join("room-a", tx_live);
        registry.join("room-a", tx_dead);
        assert_eq!(registry.room_size("room-a"), 2);

        drop(rx_dead); // closing the receiver makes that sender fail.
        registry.broadcast("room-a", "ping");

        // The live connection still receives, the dead one is pruned.
        assert_eq!(text_of(rx_live.try_recv().unwrap()), "ping");
        assert_eq!(registry.room_size("room-a"), 1);
    }

    #[test]
    fn broadcast_json_serializes_payload() {
        #[derive(Serialize)]
        struct Payload<'a> {
            kind: &'a str,
            round: u32,
        }

        let registry = RoomRegistry::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        registry.join("session-1", tx);

        registry
            .broadcast_json("session-1", &Payload { kind: "ROUND_STARTED", round: 3 })
            .expect("serialisation succeeds");

        let received = text_of(rx.try_recv().unwrap());
        assert_eq!(received, r#"{"kind":"ROUND_STARTED","round":3}"#);
    }

    #[test]
    fn send_to_targets_a_single_connection() {
        let registry = RoomRegistry::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let id1 = registry.join("room-a", tx1);
        registry.join("room-a", tx2);

        registry.send_to("room-a", id1, Message::Text("just-you".to_string().into()));

        assert_eq!(text_of(rx1.try_recv().unwrap()), "just-you");
        assert!(rx2.try_recv().is_err(), "other connection receives nothing");
    }
}
