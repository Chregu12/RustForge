//! State management for CSRF protection in OAuth2 flows
//!
//! The state parameter is used to prevent CSRF attacks by ensuring that
//! the authorization callback comes from the same browser session that
//! initiated the OAuth flow.

use base64::Engine;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// State storage entry
#[derive(Debug, Clone)]
struct StateEntry {
    #[allow(dead_code)] // reserved: stored state value for future validation
    value: String,
    created_at: SystemTime,
    expires_in: Duration,
}

impl StateEntry {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed().unwrap_or(Duration::from_secs(0)) > self.expires_in
    }
}

/// In-memory state manager for CSRF protection
///
/// In production, you should use a distributed cache (Redis) or session storage
#[derive(Clone)]
pub struct StateManager {
    states: Arc<Mutex<HashMap<String, StateEntry>>>,
    default_ttl: Duration,
}

impl StateManager {
    /// Create a new state manager
    ///
    /// # Example
    ///
    /// ```
    /// use rf_socialite::state::StateManager;
    ///
    /// let manager = StateManager::new();
    /// let state = manager.generate();
    /// assert!(manager.verify(&state));
    /// ```
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            default_ttl: Duration::from_secs(600), // 10 minutes
        }
    }

    /// Create a new state manager with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            default_ttl: ttl,
        }
    }

    /// Generate a new random state token
    pub fn generate(&self) -> String {
        let state = Self::generate_token();
        self.store(&state, self.default_ttl);
        state
    }

    /// Generate a state token with custom TTL
    pub fn generate_with_ttl(&self, ttl: Duration) -> String {
        let state = Self::generate_token();
        self.store(&state, ttl);
        state
    }

    /// Store a state token
    fn store(&self, state: &str, ttl: Duration) {
        let mut states = self.states.lock().unwrap();
        states.insert(
            state.to_string(),
            StateEntry {
                value: state.to_string(),
                created_at: SystemTime::now(),
                expires_in: ttl,
            },
        );
    }

    /// Verify and consume a state token
    ///
    /// Returns `true` if the state is valid and not expired.
    /// The state is removed after verification (one-time use).
    pub fn verify(&self, state: &str) -> bool {
        let mut states = self.states.lock().unwrap();

        // Clean up expired states
        states.retain(|_, entry| !entry.is_expired());

        // Verify and remove the state
        if let Some(entry) = states.remove(state) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Generate a random token
    fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&random_bytes)
    }

    /// Clean up expired states (should be called periodically)
    pub fn cleanup_expired(&self) {
        let mut states = self.states.lock().unwrap();
        states.retain(|_, entry| !entry.is_expired());
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_state() {
        let manager = StateManager::new();
        let state = manager.generate();
        assert!(!state.is_empty());
    }

    #[test]
    fn test_verify_valid_state() {
        let manager = StateManager::new();
        let state = manager.generate();
        assert!(manager.verify(&state));
    }

    #[test]
    fn test_verify_invalid_state() {
        let manager = StateManager::new();
        assert!(!manager.verify("invalid-state"));
    }

    #[test]
    fn test_state_one_time_use() {
        let manager = StateManager::new();
        let state = manager.generate();
        assert!(manager.verify(&state));
        // Second verification should fail (already consumed)
        assert!(!manager.verify(&state));
    }

    #[test]
    fn test_state_expiration() {
        let manager = StateManager::with_ttl(Duration::from_millis(100));
        let state = manager.generate();

        std::thread::sleep(Duration::from_millis(150));

        assert!(!manager.verify(&state));
    }

    #[test]
    fn test_different_states() {
        let manager = StateManager::new();
        let state1 = manager.generate();
        let state2 = manager.generate();
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_cleanup_expired() {
        let manager = StateManager::with_ttl(Duration::from_millis(100));
        let _state = manager.generate();

        std::thread::sleep(Duration::from_millis(150));
        manager.cleanup_expired();

        let states = manager.states.lock().unwrap();
        assert_eq!(states.len(), 0);
    }
}
