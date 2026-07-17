//! CSRF protection

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// CSRF token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

impl CsrfToken {
    pub fn new(ttl: Duration) -> Self {
        Self {
            token: Uuid::new_v4().to_string(),
            expires_at: Utc::now() + ttl,
        }
    }

    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

/// CSRF protection manager
pub struct CsrfProtection {
    tokens: Arc<RwLock<HashMap<String, CsrfToken>>>,
    ttl: Duration,
}

impl CsrfProtection {
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::seconds(ttl_seconds),
        }
    }

    pub fn generate(&self, session_id: &str) -> String {
        let token = CsrfToken::new(self.ttl);
        let token_value = token.token.clone();

        let mut tokens = self.tokens.write().unwrap();
        tokens.insert(session_id.to_string(), token);

        token_value
    }

    pub fn validate(&self, session_id: &str, token: &str) -> bool {
        let tokens = self.tokens.read().unwrap();

        if let Some(stored_token) = tokens.get(session_id) {
            if !stored_token.is_valid() {
                return false;
            }
            // Constant-time comparison to prevent timing attacks
            let stored_bytes = stored_token.token.as_bytes();
            let provided_bytes = token.as_bytes();
            if stored_bytes.len() != provided_bytes.len() {
                return false;
            }
            let mut result = 0u8;
            for (a, b) in stored_bytes.iter().zip(provided_bytes.iter()) {
                result |= a ^ b;
            }
            result == 0
        } else {
            false
        }
    }

    pub fn cleanup_expired(&self) {
        let mut tokens = self.tokens.write().unwrap();
        tokens.retain(|_, token| token.is_valid());
    }
}

impl Default for CsrfProtection {
    fn default() -> Self {
        Self::new(3600) // 1 hour default TTL
    }
}
