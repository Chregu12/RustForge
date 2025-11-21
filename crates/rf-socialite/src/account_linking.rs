//! Social account linking functionality
//!
//! This module provides utilities for linking OAuth social accounts
//! to user accounts in your application.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Social account representation
///
/// This structure represents a linked social account in your database.
/// You should create a corresponding database table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialAccount {
    /// Primary key
    pub id: i64,

    /// Associated user ID
    pub user_id: i64,

    /// OAuth provider name (google, github, facebook, etc.)
    pub provider: String,

    /// User ID from the OAuth provider
    pub provider_user_id: String,

    /// OAuth access token (should be encrypted in production)
    pub access_token: String,

    /// OAuth refresh token (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// Token expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    /// When the account was linked
    pub created_at: DateTime<Utc>,

    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl SocialAccount {
    /// Create a new social account
    pub fn new(
        user_id: i64,
        provider: impl Into<String>,
        provider_user_id: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Will be set by database
            user_id,
            provider: provider.into(),
            provider_user_id: provider_user_id.into(),
            access_token: access_token.into(),
            refresh_token: None,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set refresh token
    pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }

    /// Set expiration time
    pub fn with_expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if token needs refresh (expires in less than 5 minutes)
    pub fn needs_refresh(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let threshold = Utc::now() + chrono::Duration::minutes(5);
            threshold > expires_at
        } else {
            false
        }
    }
}

/// Database migration SQL for social_accounts table
///
/// This is a reference implementation. Adapt to your database and ORM.
pub const MIGRATION_SQL: &str = r#"
CREATE TABLE social_accounts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_provider_user UNIQUE (provider, provider_user_id)
);

CREATE INDEX idx_social_accounts_user_id ON social_accounts(user_id);
CREATE INDEX idx_social_accounts_provider ON social_accounts(provider);
"#;

/// Account linking strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkingStrategy {
    /// Automatically link to existing user by email
    AutoLinkByEmail,

    /// Always create a new user
    AlwaysCreateNew,

    /// Ask user before linking
    AskUser,
}

/// Account linker helper
#[derive(Debug)]
pub struct AccountLinker {
    strategy: LinkingStrategy,
}

impl AccountLinker {
    /// Create a new account linker with strategy
    pub fn new(strategy: LinkingStrategy) -> Self {
        Self { strategy }
    }

    /// Get the linking strategy
    pub fn strategy(&self) -> LinkingStrategy {
        self.strategy
    }

    /// Should auto-link by email?
    pub fn should_auto_link(&self) -> bool {
        self.strategy == LinkingStrategy::AutoLinkByEmail
    }

    /// Should always create new user?
    pub fn should_create_new(&self) -> bool {
        self.strategy == LinkingStrategy::AlwaysCreateNew
    }

    /// Should ask user?
    pub fn should_ask_user(&self) -> bool {
        self.strategy == LinkingStrategy::AskUser
    }
}

impl Default for AccountLinker {
    fn default() -> Self {
        Self::new(LinkingStrategy::AutoLinkByEmail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_social_account_creation() {
        let account = SocialAccount::new(1, "github", "12345", "token123");
        assert_eq!(account.user_id, 1);
        assert_eq!(account.provider, "github");
        assert_eq!(account.provider_user_id, "12345");
        assert_eq!(account.access_token, "token123");
    }

    #[test]
    fn test_social_account_with_refresh_token() {
        let account = SocialAccount::new(1, "google", "67890", "access")
            .with_refresh_token("refresh");
        assert_eq!(account.refresh_token, Some("refresh".to_string()));
    }

    #[test]
    fn test_is_expired() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let account = SocialAccount::new(1, "github", "123", "token")
            .with_expires_at(past);
        assert!(account.is_expired());
    }

    #[test]
    fn test_needs_refresh() {
        let soon = Utc::now() + chrono::Duration::minutes(3);
        let account = SocialAccount::new(1, "github", "123", "token")
            .with_expires_at(soon);
        assert!(account.needs_refresh());
    }

    #[test]
    fn test_not_expired() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let account = SocialAccount::new(1, "github", "123", "token")
            .with_expires_at(future);
        assert!(!account.is_expired());
    }

    #[test]
    fn test_linking_strategy() {
        let linker = AccountLinker::new(LinkingStrategy::AutoLinkByEmail);
        assert!(linker.should_auto_link());
        assert!(!linker.should_create_new());
        assert!(!linker.should_ask_user());
    }

    #[test]
    fn test_default_linker() {
        let linker = AccountLinker::default();
        assert_eq!(linker.strategy(), LinkingStrategy::AutoLinkByEmail);
    }
}
