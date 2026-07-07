#![allow(dead_code)] // fields/methods retained for planned functionality, not read internally yet
//! Channel implementations

use crate::{Event, EventHandler, MemberHandler, PresenceHandler, PresenceMember};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Channel type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Public,
    Private,
    Presence,
}

/// Public channel
pub struct Channel {
    pub name: String,
    pub channel_type: ChannelType,
    event_rx: broadcast::Receiver<Event>,
    handlers: Arc<DashMap<String, Vec<EventHandler>>>,
}

impl Channel {
    pub fn new(name: String, channel_type: ChannelType, event_rx: broadcast::Receiver<Event>) -> Self {
        Self {
            name,
            channel_type,
            event_rx,
            handlers: Arc::new(DashMap::new()),
        }
    }

    /// Listen to an event on this channel
    pub fn listen<F>(&self, event: &str, handler: F) -> &Self
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let event_name = self.format_event_name(event);
        let handler = Arc::new(handler) as EventHandler;

        self.handlers
            .entry(event_name)
            .or_insert_with(Vec::new)
            .push(handler);

        self
    }

    /// Stop listening to an event
    pub fn stop_listening(&self, event: &str) -> &Self {
        let event_name = self.format_event_name(event);
        self.handlers.remove(&event_name);
        self
    }

    /// Format event name (add namespace if configured)
    fn format_event_name(&self, event: &str) -> String {
        if event.starts_with("pusher:") || event.starts_with('.') {
            event.to_string()
        } else {
            event.to_string()
        }
    }

    /// Start processing events
    pub async fn start_listening(&mut self) {
        while let Ok(event) = self.event_rx.recv().await {
            // Check if event is for this channel
            if let Some(ref channel) = event.channel {
                if channel != &self.name {
                    continue;
                }
            }

            // Call handlers for this event
            if let Some(handlers) = self.handlers.get(&event.event) {
                for handler in handlers.iter() {
                    handler(event.clone());
                }
            }
        }
    }
}

/// Private channel
pub struct PrivateChannel {
    pub name: String,
    pub auth: String,
    event_rx: broadcast::Receiver<Event>,
    handlers: Arc<DashMap<String, Vec<EventHandler>>>,
}

impl PrivateChannel {
    pub fn new(name: String, event_rx: broadcast::Receiver<Event>, auth: String) -> Self {
        Self {
            name,
            auth,
            event_rx,
            handlers: Arc::new(DashMap::new()),
        }
    }

    /// Listen to an event on this channel
    pub fn listen<F>(&self, event: &str, handler: F) -> &Self
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler) as EventHandler;

        self.handlers
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(handler);

        self
    }

    /// Stop listening to an event
    pub fn stop_listening(&self, event: &str) -> &Self {
        self.handlers.remove(event);
        self
    }

    /// Whisper an event to other users (client event)
    pub fn whisper(&self, event: &str, _data: serde_json::Value) -> &Self {
        // Client events are prefixed with "client-"
        let _event_name = format!("client-{}", event);
        // Would send via WebSocket
        self
    }
}

/// Presence channel with member tracking
pub struct PresenceChannel {
    pub name: String,
    pub auth: String,
    pub channel_data: String,
    event_rx: broadcast::Receiver<Event>,
    handlers: Arc<DashMap<String, Vec<EventHandler>>>,
    members: Arc<DashMap<String, PresenceMember>>,
    here_handler: Arc<tokio::sync::RwLock<Option<PresenceHandler>>>,
    joining_handler: Arc<tokio::sync::RwLock<Option<MemberHandler>>>,
    leaving_handler: Arc<tokio::sync::RwLock<Option<MemberHandler>>>,
}

impl PresenceChannel {
    pub fn new(name: String, event_rx: broadcast::Receiver<Event>, auth: String) -> Self {
        Self {
            name,
            auth,
            channel_data: String::new(),
            event_rx,
            handlers: Arc::new(DashMap::new()),
            members: Arc::new(DashMap::new()),
            here_handler: Arc::new(tokio::sync::RwLock::new(None)),
            joining_handler: Arc::new(tokio::sync::RwLock::new(None)),
            leaving_handler: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Listen to an event on this channel
    pub fn listen<F>(&self, event: &str, handler: F) -> &Self
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler) as EventHandler;

        self.handlers
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(handler);

        self
    }

    /// Set handler for when subscription succeeds and we get initial member list
    pub fn here<F>(&self, handler: F) -> &Self
    where
        F: Fn(Vec<PresenceMember>) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler) as PresenceHandler;
        let here_handler = self.here_handler.clone();

        tokio::spawn(async move {
            let mut guard = here_handler.write().await;
            *guard = Some(handler);
        });

        self
    }

    /// Set handler for when a member joins
    pub fn joining<F>(&self, handler: F) -> &Self
    where
        F: Fn(PresenceMember) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler) as MemberHandler;
        let joining_handler = self.joining_handler.clone();

        tokio::spawn(async move {
            let mut guard = joining_handler.write().await;
            *guard = Some(handler);
        });

        self
    }

    /// Set handler for when a member leaves
    pub fn leaving<F>(&self, handler: F) -> &Self
    where
        F: Fn(PresenceMember) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler) as MemberHandler;
        let leaving_handler = self.leaving_handler.clone();

        tokio::spawn(async move {
            let mut guard = leaving_handler.write().await;
            *guard = Some(handler);
        });

        self
    }

    /// Stop listening to an event
    pub fn stop_listening(&self, event: &str) -> &Self {
        self.handlers.remove(event);
        self
    }

    /// Get current members
    pub fn members(&self) -> Vec<PresenceMember> {
        self.members
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Whisper an event to other users (client event)
    pub fn whisper(&self, event: &str, _data: serde_json::Value) -> &Self {
        let _event_name = format!("client-{}", event);
        // Would send via WebSocket
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type() {
        assert_eq!(ChannelType::Public, ChannelType::Public);
        assert_ne!(ChannelType::Public, ChannelType::Private);
    }
}
