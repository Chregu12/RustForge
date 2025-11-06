//! Session Management

use crate::models::Session;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Session Manager
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a session
    pub fn store(&self, session: Session) {
        let mut sessions = self.sessions.write()
            .expect("Session lock poisoned - unrecoverable state");
        sessions.insert(session.token.clone(), session);
    }

    /// Find session by token
    pub fn find(&self, token: &str) -> Option<Session> {
        let sessions = self.sessions.read()
            .expect("Session lock poisoned - unrecoverable state");
        sessions.get(token).cloned()
    }

    /// Delete session
    pub fn delete(&self, token: &str) {
        let mut sessions = self.sessions.write()
            .expect("Session lock poisoned - unrecoverable state");
        sessions.remove(token);
    }

    /// Delete all sessions for a user
    pub fn delete_for_user(&self, user_id: Uuid) {
        let mut sessions = self.sessions.write()
            .expect("Session lock poisoned - unrecoverable state");
        sessions.retain(|_, session| session.user_id != user_id);
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write()
            .expect("Session lock poisoned - unrecoverable state");
        sessions.retain(|_, session| !session.is_expired());
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_storage() {
        let manager = SessionManager::new();
        let session = Session::new(Uuid::new_v4(), "token123".to_string(), 3600);
        let token = session.token.clone();

        manager.store(session);

        let found = manager.find(&token);
        assert!(found.is_some());
    }

    #[test]
    fn test_session_deletion() {
        let manager = SessionManager::new();
        let session = Session::new(Uuid::new_v4(), "token123".to_string(), 3600);
        let token = session.token.clone();

        manager.store(session);
        manager.delete(&token);

        let found = manager.find(&token);
        assert!(found.is_none());
    }
}
