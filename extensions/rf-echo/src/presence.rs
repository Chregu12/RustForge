//! Presence channel member tracking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A member in a presence channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceMember {
    /// Unique user ID
    pub user_id: String,
    /// User info (custom data)
    pub user_info: serde_json::Value,
}

impl PresenceMember {
    pub fn new(user_id: impl Into<String>, user_info: serde_json::Value) -> Self {
        Self {
            user_id: user_id.into(),
            user_info,
        }
    }

    /// Get a field from user_info
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.user_info.get(key)
    }

    /// Get a string field from user_info
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.user_info.get(key).and_then(|v| v.as_str())
    }
}

/// State of presence channel members
#[derive(Debug, Clone, Default)]
pub struct PresenceState {
    /// Map of user_id to member info
    members: HashMap<String, PresenceMember>,
    /// Own user ID (if present in channel)
    me: Option<String>,
}

impl PresenceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current user
    pub fn set_me(&mut self, user_id: String) {
        self.me = Some(user_id);
    }

    /// Get the current user's ID
    pub fn me(&self) -> Option<&String> {
        self.me.as_ref()
    }

    /// Get the current user's member info
    pub fn me_member(&self) -> Option<&PresenceMember> {
        self.me.as_ref().and_then(|id| self.members.get(id))
    }

    /// Add a member
    pub fn add_member(&mut self, member: PresenceMember) {
        self.members.insert(member.user_id.clone(), member);
    }

    /// Remove a member
    pub fn remove_member(&mut self, user_id: &str) -> Option<PresenceMember> {
        self.members.remove(user_id)
    }

    /// Get a member by ID
    pub fn get(&self, user_id: &str) -> Option<&PresenceMember> {
        self.members.get(user_id)
    }

    /// Get all members
    pub fn all(&self) -> Vec<&PresenceMember> {
        self.members.values().collect()
    }

    /// Get member count
    pub fn count(&self) -> usize {
        self.members.len()
    }

    /// Check if user is present
    pub fn contains(&self, user_id: &str) -> bool {
        self.members.contains_key(user_id)
    }

    /// Clear all members
    pub fn clear(&mut self) {
        self.members.clear();
        self.me = None;
    }

    /// Set members from subscription succeeded event
    pub fn set_members(&mut self, members: Vec<PresenceMember>) {
        self.members.clear();
        for member in members {
            self.members.insert(member.user_id.clone(), member);
        }
    }
}

/// Parse members from Pusher subscription_succeeded event
pub fn parse_presence_members(data: &serde_json::Value) -> Vec<PresenceMember> {
    let mut members = Vec::new();

    if let Some(presence) = data.get("presence") {
        if let Some(hash) = presence.get("hash").and_then(|h| h.as_object()) {
            for (user_id, user_info) in hash {
                members.push(PresenceMember {
                    user_id: user_id.clone(),
                    user_info: user_info.clone(),
                });
            }
        }
    }

    members
}

/// Parse member_added event
pub fn parse_member_added(data: &serde_json::Value) -> Option<PresenceMember> {
    let user_id = data.get("user_id")?.as_str()?.to_string();
    let user_info = data.get("user_info").cloned().unwrap_or(serde_json::Value::Null);

    Some(PresenceMember { user_id, user_info })
}

/// Parse member_removed event
pub fn parse_member_removed(data: &serde_json::Value) -> Option<String> {
    data.get("user_id")?.as_str().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_member() {
        let member = PresenceMember::new(
            "123",
            serde_json::json!({
                "name": "John Doe",
                "email": "john@example.com"
            }),
        );

        assert_eq!(member.user_id, "123");
        assert_eq!(member.get_string("name"), Some("John Doe"));
    }

    #[test]
    fn test_presence_state() {
        let mut state = PresenceState::new();

        state.add_member(PresenceMember::new("1", serde_json::json!({"name": "Alice"})));
        state.add_member(PresenceMember::new("2", serde_json::json!({"name": "Bob"})));

        assert_eq!(state.count(), 2);
        assert!(state.contains("1"));
        assert!(!state.contains("3"));

        let removed = state.remove_member("1");
        assert!(removed.is_some());
        assert_eq!(state.count(), 1);
    }

    #[test]
    fn test_parse_presence_members() {
        let data = serde_json::json!({
            "presence": {
                "count": 2,
                "ids": ["1", "2"],
                "hash": {
                    "1": {"name": "Alice"},
                    "2": {"name": "Bob"}
                }
            }
        });

        let members = parse_presence_members(&data);
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn test_parse_member_added() {
        let data = serde_json::json!({
            "user_id": "123",
            "user_info": {"name": "Charlie"}
        });

        let member = parse_member_added(&data);
        assert!(member.is_some());
        assert_eq!(member.unwrap().user_id, "123");
    }
}
