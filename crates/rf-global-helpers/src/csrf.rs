//! CSRF (Cross-Site Request Forgery) protection helpers.
//!
//! This module provides functions for generating and validating CSRF tokens.

use uuid::Uuid;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// CSRF token with expiration
#[derive(Debug, Clone)]
struct CsrfToken {
    token: String,
    created_at: SystemTime,
}

impl CsrfToken {
    fn new() -> Self {
        Self {
            token: Uuid::new_v4().to_string(),
            created_at: SystemTime::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        match SystemTime::now().duration_since(self.created_at) {
            Ok(elapsed) => elapsed > ttl,
            Err(_) => true,
        }
    }
}

/// CSRF token storage (in production, this would use sessions)
static CSRF_TOKENS: Lazy<RwLock<HashMap<String, CsrfToken>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

/// Default token TTL (1 hour)
const TOKEN_TTL: Duration = Duration::from_secs(3600);

/// Generate a new CSRF token.
///
/// In production, this would be tied to a user's session.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::csrf_token;
///
/// let token = csrf_token();
/// assert!(!token.is_empty());
/// ```
pub fn csrf_token() -> String {
    let token = CsrfToken::new();
    let token_str = token.token.clone();

    // Store token (in production, would use session ID)
    let session_id = Uuid::new_v4().to_string();
    CSRF_TOKENS.write().insert(session_id, token);

    token_str
}

/// Generate a CSRF token for a specific session.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::csrf::csrf_token_for_session;
///
/// let session_id = "user-session-123";
/// let token = csrf_token_for_session(session_id);
/// ```
pub fn csrf_token_for_session(session_id: &str) -> String {
    let token = CsrfToken::new();
    let token_str = token.token.clone();

    CSRF_TOKENS
        .write()
        .insert(session_id.to_string(), token);

    token_str
}

/// Verify a CSRF token for a session.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::csrf::{csrf_token_for_session, verify_csrf_token};
///
/// let session_id = "test-session";
/// let token = csrf_token_for_session(session_id);
///
/// assert!(verify_csrf_token(session_id, &token));
/// assert!(!verify_csrf_token(session_id, "invalid-token"));
/// ```
pub fn verify_csrf_token(session_id: &str, token: &str) -> bool {
    let tokens = CSRF_TOKENS.read();

    if let Some(stored_token) = tokens.get(session_id) {
        if stored_token.is_expired(TOKEN_TTL) {
            return false;
        }
        return stored_token.token == token;
    }

    false
}

/// Regenerate a CSRF token for a session.
///
/// This should be called after successful login or privilege escalation.
pub fn regenerate_csrf_token(session_id: &str) -> String {
    let new_token = CsrfToken::new();
    let token_str = new_token.token.clone();

    CSRF_TOKENS
        .write()
        .insert(session_id.to_string(), new_token);

    token_str
}

/// Delete a CSRF token for a session.
pub fn delete_csrf_token(session_id: &str) {
    CSRF_TOKENS.write().remove(session_id);
}

/// Generate an HTML hidden input field with the CSRF token.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::csrf_field;
///
/// let html = csrf_field();
/// assert!(html.contains("name=\"_token\""));
/// ```
pub fn csrf_field() -> String {
    let token = csrf_token();
    format!("<input type=\"hidden\" name=\"_token\" value=\"{}\" />", token)
}

/// Generate an HTML meta tag with the CSRF token.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::csrf::csrf_meta;
///
/// let html = csrf_meta();
/// assert!(html.contains("name=\"csrf-token\""));
/// ```
pub fn csrf_meta() -> String {
    let token = csrf_token();
    format!("<meta name=\"csrf-token\" content=\"{}\" />", token)
}

/// Clean up expired CSRF tokens.
///
/// This should be called periodically (e.g., via a scheduled job).
pub fn cleanup_expired_tokens() {
    let mut tokens = CSRF_TOKENS.write();
    tokens.retain(|_, token| !token.is_expired(TOKEN_TTL));
}

/// Get the number of stored CSRF tokens (for testing/monitoring).
pub fn token_count() -> usize {
    CSRF_TOKENS.read().len()
}

/// Clear all CSRF tokens (for testing).
pub fn clear_all_tokens() {
    CSRF_TOKENS.write().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let token = csrf_token();
        assert!(!token.is_empty());
        assert_eq!(token.len(), 36); // UUID v4 length with hyphens
    }

    #[test]
    fn test_csrf_token_unique() {
        let token1 = csrf_token();
        let token2 = csrf_token();
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_csrf_token_for_session() {
        clear_all_tokens();

        let session_id = "test-session-1";
        let token = csrf_token_for_session(session_id);

        assert!(!token.is_empty());
        assert!(verify_csrf_token(session_id, &token));
    }

    #[test]
    fn test_verify_csrf_token() {
        clear_all_tokens();

        let session_id = "verify-test";
        let token = csrf_token_for_session(session_id);

        assert!(verify_csrf_token(session_id, &token));
        assert!(!verify_csrf_token(session_id, "wrong-token"));
        assert!(!verify_csrf_token("wrong-session", &token));
    }

    #[test]
    fn test_regenerate_csrf_token() {
        clear_all_tokens();

        let session_id = "regen-test";
        let token1 = csrf_token_for_session(session_id);
        let token2 = regenerate_csrf_token(session_id);

        assert_ne!(token1, token2);
        assert!(!verify_csrf_token(session_id, &token1));
        assert!(verify_csrf_token(session_id, &token2));
    }

    #[test]
    fn test_delete_csrf_token() {
        clear_all_tokens();

        let session_id = "delete-test";
        let token = csrf_token_for_session(session_id);

        assert!(verify_csrf_token(session_id, &token));

        delete_csrf_token(session_id);
        assert!(!verify_csrf_token(session_id, &token));
    }

    #[test]
    fn test_csrf_field() {
        let html = csrf_field();
        assert!(html.contains("<input"));
        assert!(html.contains("type=\"hidden\""));
        assert!(html.contains("name=\"_token\""));
        assert!(html.contains("value=\""));
    }

    #[test]
    fn test_csrf_meta() {
        let html = csrf_meta();
        assert!(html.contains("<meta"));
        assert!(html.contains("name=\"csrf-token\""));
        assert!(html.contains("content=\""));
    }

    #[test]
    fn test_token_count() {
        // Use unique session IDs to avoid interference
        let session1 = format!("test-token-count-{}", uuid::Uuid::new_v4());
        let session2 = format!("test-token-count-{}", uuid::Uuid::new_v4());

        let before = token_count();

        csrf_token_for_session(&session1);
        csrf_token_for_session(&session2);

        let after = token_count();

        // Should have at least 2 more tokens
        assert!(after >= before + 2);
    }

    #[test]
    fn test_clear_all_tokens() {
        clear_all_tokens();

        csrf_token_for_session("session1");
        csrf_token_for_session("session2");
        assert!(token_count() > 0);

        clear_all_tokens();
        assert_eq!(token_count(), 0);
    }
}
