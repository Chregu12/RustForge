//! Personal Access Token model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Personal Access Token stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalAccessToken {
    pub id: i64,
    pub tokenable_type: String, // "User", "App", etc.
    pub tokenable_id: i64,
    pub name: String,
    pub token: String, // SHA256 hash
    pub abilities: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_agent: Option<String>,      // Device/browser user agent
    pub last_used_ip: Option<String>,    // Last IP address used
}

/// New token with plaintext value (only returned once)
#[derive(Debug, Clone)]
pub struct NewToken {
    pub access_token: String, // Plaintext token (show once)
    pub token: PersonalAccessToken,
}

impl PersonalAccessToken {
    /// Generate a random token
    pub fn generate_token() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..80)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Hash a token using SHA256
    pub fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Check if token has an ability
    pub fn can(&self, ability: &str) -> bool {
        self.abilities.contains(&ability.to_string()) || self.abilities.contains(&"*".to_string())
    }

    /// Check if token has any of the abilities
    pub fn can_any(&self, abilities: &[&str]) -> bool {
        abilities.iter().any(|&ability| self.can(ability))
    }

    /// Check if token has all abilities
    pub fn can_all(&self, abilities: &[&str]) -> bool {
        abilities.iter().all(|&ability| self.can(ability))
    }

    /// Create from database model
    pub fn from_model(model: crate::models::Model) -> Self {
        let abilities = if let Some(arr) = model.abilities.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };

        Self {
            id: model.id,
            tokenable_type: model.tokenable_type,
            tokenable_id: model.tokenable_id,
            name: model.name,
            token: model.token,
            abilities,
            last_used_at: model.last_used_at,
            expires_at: model.expires_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
            user_agent: model.user_agent,
            last_used_ip: model.last_used_ip,
        }
    }

    /// Get device name from user agent
    pub fn device_name(&self) -> Option<String> {
        self.user_agent.as_ref().and_then(|ua| {
            // Simple device detection from user agent
            if ua.contains("Mobile") || ua.contains("Android") || ua.contains("iPhone") {
                Some("Mobile Device".to_string())
            } else if ua.contains("iPad") || ua.contains("Tablet") {
                Some("Tablet".to_string())
            } else if ua.contains("Windows") || ua.contains("Macintosh") || ua.contains("Linux") {
                Some("Desktop".to_string())
            } else {
                Some("Unknown Device".to_string())
            }
        })
    }

    /// Get browser name from user agent
    pub fn browser_name(&self) -> Option<String> {
        self.user_agent.as_ref().and_then(|ua| {
            if ua.contains("Chrome") && !ua.contains("Edg") {
                Some("Chrome".to_string())
            } else if ua.contains("Safari") && !ua.contains("Chrome") {
                Some("Safari".to_string())
            } else if ua.contains("Firefox") {
                Some("Firefox".to_string())
            } else if ua.contains("Edg") {
                Some("Edge".to_string())
            } else {
                Some("Unknown Browser".to_string())
            }
        })
    }

    /// Check if token was recently used (within last N minutes)
    pub fn is_recently_used(&self, minutes: i64) -> bool {
        if let Some(last_used) = self.last_used_at {
            let now = Utc::now();
            let diff = now - last_used;
            diff.num_minutes() <= minutes
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = PersonalAccessToken::generate_token();
        assert_eq!(token.len(), 80);
    }

    #[test]
    fn test_hash_token() {
        let token = "my-secret-token";
        let hash = PersonalAccessToken::hash_token(token);
        assert_eq!(hash.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_token_abilities() {
        let token = PersonalAccessToken {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: vec!["read:posts".to_string(), "write:posts".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_agent: None,
            last_used_ip: None,
        };

        assert!(token.can("read:posts"));
        assert!(token.can("write:posts"));
        assert!(!token.can("delete:posts"));
        assert!(token.can_any(&["read:posts", "delete:posts"]));
        assert!(!token.can_all(&["read:posts", "delete:posts"]));
    }

    #[test]
    fn test_wildcard_ability() {
        let token = PersonalAccessToken {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: vec!["*".to_string()],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_agent: None,
            last_used_ip: None,
        };

        assert!(token.can("anything"));
        assert!(token.can("read:posts"));
        assert!(token.can("delete:everything"));
    }

    #[test]
    fn test_device_detection() {
        let token = PersonalAccessToken {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: vec![],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_agent: Some("Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)".to_string()),
            last_used_ip: Some("192.168.1.1".to_string()),
        };

        assert_eq!(token.device_name(), Some("Mobile Device".to_string()));
    }

    #[test]
    fn test_browser_detection() {
        let token = PersonalAccessToken {
            id: 1,
            tokenable_type: "User".to_string(),
            tokenable_id: 123,
            name: "test".to_string(),
            token: "hash".to_string(),
            abilities: vec![],
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_agent: Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string()),
            last_used_ip: None,
        };

        assert_eq!(token.browser_name(), Some("Chrome".to_string()));
    }
}
