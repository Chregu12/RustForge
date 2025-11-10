//! OAuth2 types

use serde::{Deserialize, Serialize};

/// OAuth2 grant types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantType {
    AuthorizationCode,
    ClientCredentials,
    RefreshToken,
    Password, // Deprecated but sometimes needed
}

/// OAuth2 scope
pub type Scope = String;

/// OAuth2 server configuration
#[derive(Debug, Clone)]
pub struct OAuth2Config {
    pub issuer: String,
    pub access_token_ttl: u64,  // seconds
    pub refresh_token_ttl: u64, // seconds
}

impl Default for OAuth2Config {
    fn default() -> Self {
        Self {
            issuer: "https://auth.example.com".to_string(),
            access_token_ttl: 3600,      // 1 hour
            refresh_token_ttl: 86400 * 7, // 7 days
        }
    }
}
