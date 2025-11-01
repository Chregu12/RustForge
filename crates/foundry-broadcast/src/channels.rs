//! Channel types

use std::sync::{Arc, RwLock};
use crate::{Result, PresenceTracker};

#[derive(Clone)]
pub struct Channel {
    pub name: String,
    subscribers: Arc<RwLock<Vec<String>>>,
}

impl Channel {
    pub fn new(name: String) -> Self {
        Self {
            name,
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn subscribe(&self, subscriber_id: String) {
        self.subscribers.write().unwrap().push(subscriber_id);
    }

    pub fn unsubscribe(&self, subscriber_id: &str) {
        self.subscribers.write().unwrap().retain(|id| id != subscriber_id);
    }

    pub async fn send(&self, event: String, data: serde_json::Value) -> Result<()> {
        // Broadcast to all subscribers
        Ok(())
    }
}

pub struct PrivateChannel {
    channel: Channel,
}

impl PrivateChannel {
    pub fn new(name: String) -> Self {
        Self {
            channel: Channel::new(name),
        }
    }

    pub fn authorize(&self, _user_id: i64) -> bool {
        // Implement authorization logic
        true
    }
}

pub struct PresenceChannel {
    channel: Channel,
    presence: Arc<RwLock<PresenceTracker>>,
}

impl PresenceChannel {
    pub fn new(name: String) -> Self {
        Self {
            channel: Channel::new(name),
            presence: Arc::new(RwLock::new(PresenceTracker::new())),
        }
    }

    pub fn join(&self, user_id: i64, user_info: serde_json::Value) {
        self.presence.write().unwrap().add(user_id, user_info);
    }

    pub fn leave(&self, user_id: i64) {
        self.presence.write().unwrap().remove(user_id);
    }

    pub fn get_members(&self) -> Vec<(i64, serde_json::Value)> {
        self.presence.read().unwrap().list()
    }
}
