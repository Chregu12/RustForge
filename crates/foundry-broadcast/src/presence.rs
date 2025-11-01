//! Presence tracking

use std::collections::HashMap;

pub struct PresenceTracker {
    members: HashMap<i64, serde_json::Value>,
}

impl PresenceTracker {
    pub fn new() -> Self {
        Self {
            members: HashMap::new(),
        }
    }

    pub fn add(&mut self, user_id: i64, info: serde_json::Value) {
        self.members.insert(user_id, info);
    }

    pub fn remove(&mut self, user_id: i64) {
        self.members.remove(&user_id);
    }

    pub fn list(&self) -> Vec<(i64, serde_json::Value)> {
        self.members.iter().map(|(k, v)| (*k, v.clone())).collect()
    }

    pub fn count(&self) -> usize {
        self.members.len()
    }
}

impl Default for PresenceTracker {
    fn default() -> Self {
        Self::new()
    }
}
