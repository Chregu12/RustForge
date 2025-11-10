//! OAuth2 token management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Access token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub client_id: String,
    pub user_id: Option<String>,
    pub scopes: Vec<String>,
}

impl AccessToken {
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if token has scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Refresh token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub client_id: String,
    pub user_id: Option<String>,
    pub scopes: Vec<String>,
}

impl RefreshToken {
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Token response (RFC 6749)
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl TokenResponse {
    /// Create bearer token response
    pub fn bearer(access_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
            refresh_token: None,
            scope: None,
        }
    }

    /// Add refresh token
    pub fn with_refresh_token(mut self, refresh_token: String) -> Self {
        self.refresh_token = Some(refresh_token);
        self
    }

    /// Add scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scope = Some(scopes.join(" "));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_access_token_expiry() {
        let token = AccessToken {
            token: "test".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            client_id: "client".to_string(),
            user_id: None,
            scopes: vec![],
        };

        assert!(!token.is_expired());

        let expired_token = AccessToken {
            token: "test".to_string(),
            expires_at: Utc::now() - Duration::hours(1),
            client_id: "client".to_string(),
            user_id: None,
            scopes: vec![],
        };

        assert!(expired_token.is_expired());
    }

    #[test]
    fn test_token_scopes() {
        let token = AccessToken {
            token: "test".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            client_id: "client".to_string(),
            user_id: None,
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        assert!(token.has_scope("read"));
        assert!(token.has_scope("write"));
        assert!(!token.has_scope("admin"));
    }

    #[test]
    fn test_token_response() {
        let response = TokenResponse::bearer("token123".to_string(), 3600)
            .with_refresh_token("refresh123".to_string())
            .with_scopes(vec!["read".to_string(), "write".to_string()]);

        assert_eq!(response.access_token, "token123");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
        assert_eq!(response.refresh_token, Some("refresh123".to_string()));
        assert_eq!(response.scope, Some("read write".to_string()));
    }
}
