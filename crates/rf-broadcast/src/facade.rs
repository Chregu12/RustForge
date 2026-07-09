//! Process-global `Broadcast` facade.
//!
//! Every other RustForge backend (`Cache`, `Mail`, `Event`, `Storage`) is
//! callable statically from anywhere — including from inside a background job
//! body, which runs deserialized in a `Worker` with **no** captured application
//! state. Broadcasting was the odd one out: it required threading an
//! `Arc<MemoryBroadcaster>` to every call site, so a job that wanted to push a
//! real-time event had to stash the broadcaster in its own `static`.
//!
//! This module closes that gap by mirroring the other facades: a single
//! process-global default [`MemoryBroadcaster`] plus a static [`Broadcast`] API
//! over it. The same global default backs [`websocket_router_default`] so a
//! WebSocket server and a job dispatched anywhere in the process share one
//! broadcaster with **zero** wiring.
//!
//! ```no_run
//! use rf_broadcast::Broadcast;
//! use serde_json::json;
//!
//! # fn example() -> rf_broadcast::BroadcastResult<()> {
//! // One-liner, no Arc threaded — safe to call from inside a Job::handle().
//! Broadcast::event("orders", "order.created", json!({ "id": 42 }))?;
//!
//! // Or the fluent builder:
//! Broadcast::to("orders").event("order.created").with(json!({ "id": 43 }))?;
//! # Ok(())
//! # }
//! ```
//!
//! The explicit `Arc`-passing API (`BroadcastFacade`, the free `broadcast`
//! function, `websocket_router(broadcaster)`) is unchanged and keeps working for
//! callers that want to own their broadcaster.

use crate::{
    BroadcastResult, Channel, ConnectionId, Event, MemoryBroadcaster, PresenceInfo, SimpleEvent,
    UserId,
};
use serde_json::Value;
use std::sync::{Arc, OnceLock};

/// The process-global default in-memory broadcaster backing the [`Broadcast`]
/// facade and [`websocket_router_default`].
///
/// A single fixed instance for the whole process: because the `MemoryBroadcaster`
/// owns the underlying `tokio::sync::broadcast` channel that WebSocket handlers
/// receive on, a stable shared instance guarantees that events published through
/// the facade reach clients connected via [`websocket_router_default`] with no
/// configuration.
static DEFAULT_BROADCASTER: OnceLock<Arc<MemoryBroadcaster>> = OnceLock::new();

/// Get a handle to the process-global default broadcaster.
///
/// This is the same instance the [`Broadcast`] facade publishes through and that
/// [`websocket_router_default`] serves, so you can pass it to any of the explicit
/// `Arc`-taking APIs ([`crate::websocket_router`], [`BroadcastFacade::new`]) to
/// interoperate with the facade.
pub fn default_broadcaster() -> Arc<MemoryBroadcaster> {
    Arc::clone(DEFAULT_BROADCASTER.get_or_init(|| Arc::new(MemoryBroadcaster::new())))
}

/// Laravel-style process-global broadcasting facade.
///
/// All methods operate on the process-global default [`MemoryBroadcaster`]
/// (see [`default_broadcaster`]); no broadcaster handle is threaded through call
/// sites, so this works from inside a background [`Job`](../../rf_queue) body.
pub struct Broadcast;

impl Broadcast {
    /// Get the process-global default broadcaster (for wiring an explicit
    /// [`crate::websocket_router`] or [`BroadcastFacade`] to the same instance).
    pub fn broadcaster() -> Arc<MemoryBroadcaster> {
        default_broadcaster()
    }

    /// Build a [`Channel`] by name (public channel).
    pub fn channel(name: &str) -> Channel {
        Channel::new(name)
    }

    /// Broadcast a typed [`Event`] to a channel through the global default
    /// broadcaster. This is the handle-free counterpart to the free
    /// [`crate::broadcast`] function.
    ///
    /// Safe to call from inside **or** outside a Tokio runtime (it drives the
    /// broadcaster's synchronous in-memory core, so it never spins up or blocks
    /// on a nested runtime), which is what lets a background job body broadcast.
    pub fn broadcast(channel: &Channel, event: &dyn Event) -> BroadcastResult<()> {
        default_broadcaster().broadcast_now(channel, event)
    }

    /// Broadcast a named event with a JSON payload to a channel in one call.
    ///
    /// This is the ergonomic one-liner a background job reaches for: no event
    /// struct to define, no broadcaster to thread.
    ///
    /// ```no_run
    /// use rf_broadcast::Broadcast;
    /// use serde_json::json;
    ///
    /// # fn example() -> rf_broadcast::BroadcastResult<()> {
    /// Broadcast::event("orders", "order.created", json!({ "id": 42 }))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn event(
        channel: impl Into<String>,
        event: impl Into<String>,
        data: Value,
    ) -> BroadcastResult<()> {
        let channel = Channel::new(channel);
        let event = SimpleEvent::new(event, data, vec![channel.clone()]);
        Self::broadcast(&channel, &event)
    }

    /// Start a fluent broadcast to `channel`.
    ///
    /// ```no_run
    /// use rf_broadcast::Broadcast;
    /// use serde_json::json;
    ///
    /// # fn example() -> rf_broadcast::BroadcastResult<()> {
    /// Broadcast::to("orders").event("order.created").with(json!({ "id": 42 }))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to(channel: impl Into<String>) -> PendingBroadcast {
        PendingBroadcast {
            channel: Channel::new(channel),
            event_name: None,
        }
    }

    /// Subscribe a connection to a channel on the global default broadcaster.
    pub fn subscribe(
        channel: &Channel,
        connection_id: ConnectionId,
        user_id: Option<UserId>,
    ) -> BroadcastResult<()> {
        default_broadcaster().subscribe_now(channel, connection_id, user_id)
    }

    /// Unsubscribe a connection from a channel on the global default broadcaster.
    pub fn unsubscribe(channel: &Channel, connection_id: &ConnectionId) -> BroadcastResult<()> {
        default_broadcaster().unsubscribe_now(channel, connection_id)
    }

    /// List the connections currently subscribed to a channel.
    pub fn connections(channel: &Channel) -> BroadcastResult<Vec<ConnectionId>> {
        default_broadcaster().connections_now(channel)
    }

    /// List presence info for a presence channel.
    pub fn presence(channel: &Channel) -> BroadcastResult<Vec<PresenceInfo>> {
        default_broadcaster().presence_now(channel)
    }
}

/// A broadcast in progress, targeted at a channel, awaiting an event name and
/// payload. Created by [`Broadcast::to`].
pub struct PendingBroadcast {
    channel: Channel,
    event_name: Option<String>,
}

impl PendingBroadcast {
    /// Set the event name for this broadcast.
    pub fn event(mut self, name: impl Into<String>) -> Self {
        self.event_name = Some(name.into());
        self
    }

    /// Attach the JSON payload and dispatch the broadcast through the global
    /// default broadcaster. If no [`event`](Self::event) name was set, the
    /// channel name is used as the event name.
    pub fn with(self, data: Value) -> BroadcastResult<()> {
        let event_name = self
            .event_name
            .unwrap_or_else(|| self.channel.name().to_string());
        let event = SimpleEvent::new(event_name, data, vec![self.channel.clone()]);
        Broadcast::broadcast(&self.channel, &event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn global_default_broadcaster_is_stable() {
        // The facade and the router must share ONE instance, so repeated handles
        // point at the same broadcaster.
        let a = default_broadcaster();
        let b = Broadcast::broadcaster();
        assert!(Arc::ptr_eq(&a, &b), "global default broadcaster must be a single instance");
    }

    // Single sequential test: the facade publishes through ONE process-global
    // broadcaster whose `tokio::sync::broadcast` sender is shared across the whole
    // process. Running the scenarios in one test (rather than parallel `#[test]`s
    // that would race on that shared sender) keeps each `try_recv` deterministic:
    // the message published immediately before it is the one it reads.
    #[test]
    fn facade_publishes_through_global_broadcaster_no_handle() {
        // Subscribe fake connections on the SAME global broadcaster the facade
        // publishes through, then receive raw events off its broadcast channel to
        // prove the calls actually delivered — no Arc threaded anywhere.
        let mut rx = Broadcast::broadcaster().subscribe_to_events();

        let orders = Channel::new("facade-orders");
        Broadcast::subscribe(&orders, "conn-facade-1".to_string(), None).unwrap();

        // (1) The `event` one-liner.
        Broadcast::event("facade-orders", "order.created", json!({ "id": 7 })).unwrap();
        let msg = rx
            .try_recv()
            .expect("facade event should be published to the global broadcaster");
        assert_eq!(msg.event_name, "order.created");
        assert!(msg.connections.contains(&"conn-facade-1".to_string()));
        assert!(msg.data.contains("\"id\":7"));

        // (2) The fluent `to(..).event(..).with(..)` builder.
        let builder = Channel::new("facade-builder");
        Broadcast::subscribe(&builder, "conn-facade-2".to_string(), None).unwrap();
        Broadcast::to("facade-builder")
            .event("thing.done")
            .with(json!({ "ok": true }))
            .unwrap();
        let msg = rx.try_recv().expect("fluent builder should publish an event");
        assert_eq!(msg.event_name, "thing.done");
        assert!(msg.connections.contains(&"conn-facade-2".to_string()));
    }
}
