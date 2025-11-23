//! OAuth State Management
//!
//! Provides CSRF protection for OAuth flows through state parameter validation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use rand::{distributions::Alphanumeric, Rng};

/// State record with expiration
#[derive(Debug, Clone)]
struct StateRecord {
    created_at: u128,
    expires_at: u128,
}

impl StateRecord {
    fn new(ttl_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        Self {
            created_at: now,
            expires_at: now + (ttl_secs as u128 * 1000), // Convert seconds to milliseconds
        }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        now > self.expires_at
    }
}

/// OAuth state manager for CSRF protection
pub struct StateManager {
    states: Arc<RwLock<HashMap<String, StateRecord>>>,
    ttl: Duration,
}

impl StateManager {
    /// Create a new state manager
    pub fn new(ttl: Duration) -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Generate a new state token
    pub async fn generate(&self) -> String {
        let state: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let record = StateRecord::new(self.ttl.as_secs());
        self.states.write().await.insert(state.clone(), record);

        // Clean up expired states
        self.cleanup_expired().await;

        state
    }

    /// Validate a state token
    pub async fn validate(&self, state: &str) -> bool {
        let mut states = self.states.write().await;

        match states.get(state) {
            Some(record) if !record.is_expired() => {
                // Remove after validation (one-time use)
                states.remove(state);
                true
            }
            Some(_) => {
                // Expired, remove it
                states.remove(state);
                false
            }
            None => false,
        }
    }

    /// Clean up expired state tokens
    async fn cleanup_expired(&self) {
        let mut states = self.states.write().await;
        states.retain(|_, record| !record.is_expired());
    }
}

impl Default for StateManager {
    fn default() -> Self {
        // Default TTL of 10 minutes
        Self::new(Duration::from_secs(600))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_generate_state() {
        let manager = StateManager::new(Duration::from_secs(300));
        let state = manager.generate().await;

        assert_eq!(state.len(), 32);
    }

    #[tokio::test]
    async fn test_validate_state() {
        let manager = StateManager::new(Duration::from_secs(300));
        let state = manager.generate().await;

        assert!(manager.validate(&state).await);

        // Cannot validate twice (one-time use)
        assert!(!manager.validate(&state).await);
    }

    #[tokio::test]
    async fn test_invalid_state() {
        let manager = StateManager::new(Duration::from_secs(300));

        assert!(!manager.validate("invalid-state").await);
    }

    #[tokio::test]
    async fn test_expired_state() {
        let manager = StateManager::new(Duration::from_millis(100));
        let state = manager.generate().await;

        // Wait for state to expire
        sleep(Duration::from_millis(150)).await;

        assert!(!manager.validate(&state).await);
    }
}
