//! Transient (non-persistent) tokens
//!
//! Transient tokens are useful for:
//! - Testing
//! - Temporary access
//! - In-memory token management
//! - Scenarios where database persistence is not needed

use crate::{PersonalAccessToken, SanctumError};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory storage for transient tokens
#[derive(Debug, Clone)]
pub struct TransientTokenStore {
    tokens: Arc<RwLock<HashMap<String, PersonalAccessToken>>>,
}

impl Default for TransientTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TransientTokenStore {
    /// Create a new transient token store
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a token (using its hashed value as key)
    pub fn store(&self, token: PersonalAccessToken) -> Result<(), SanctumError> {
        let mut store = self.tokens.write().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire write lock".to_string(),
            ))
        })?;

        store.insert(token.token.clone(), token);
        Ok(())
    }

    /// Find a token by its hashed value
    pub fn find(&self, hashed_token: &str) -> Result<Option<PersonalAccessToken>, SanctumError> {
        let store = self.tokens.read().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire read lock".to_string(),
            ))
        })?;

        Ok(store.get(hashed_token).cloned())
    }

    /// Remove a token
    pub fn remove(&self, hashed_token: &str) -> Result<(), SanctumError> {
        let mut store = self.tokens.write().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire write lock".to_string(),
            ))
        })?;

        store.remove(hashed_token);
        Ok(())
    }

    /// Remove all tokens for a tokenable
    pub fn remove_all_for_tokenable(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
    ) -> Result<(), SanctumError> {
        let mut store = self.tokens.write().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire write lock".to_string(),
            ))
        })?;

        store.retain(|_, token| {
            token.tokenable_type != tokenable_type || token.tokenable_id != tokenable_id
        });

        Ok(())
    }

    /// Get all tokens for a tokenable
    pub fn find_by_tokenable(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
    ) -> Result<Vec<PersonalAccessToken>, SanctumError> {
        let store = self.tokens.read().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire read lock".to_string(),
            ))
        })?;

        let tokens = store
            .values()
            .filter(|token| {
                token.tokenable_type == tokenable_type && token.tokenable_id == tokenable_id
            })
            .cloned()
            .collect();

        Ok(tokens)
    }

    /// Clean up expired tokens
    pub fn cleanup_expired(&self) -> Result<usize, SanctumError> {
        let mut store = self.tokens.write().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire write lock".to_string(),
            ))
        })?;

        let now = Utc::now();
        let before_count = store.len();

        store.retain(|_, token| {
            if let Some(expires_at) = token.expires_at {
                expires_at > now
            } else {
                true // Keep tokens without expiration
            }
        });

        Ok(before_count - store.len())
    }

    /// Update last_used_at for a token
    pub fn touch(&self, hashed_token: &str) -> Result<(), SanctumError> {
        let mut store = self.tokens.write().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire write lock".to_string(),
            ))
        })?;

        if let Some(token) = store.get_mut(hashed_token) {
            token.last_used_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Clear all tokens (useful for testing)
    pub fn clear(&self) -> Result<(), SanctumError> {
        let mut store = self.tokens.write().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire write lock".to_string(),
            ))
        })?;

        store.clear();
        Ok(())
    }

    /// Get count of stored tokens
    pub fn count(&self) -> Result<usize, SanctumError> {
        let store = self.tokens.read().map_err(|_| {
            SanctumError::DatabaseError(sea_orm::DbErr::Custom(
                "Failed to acquire read lock".to_string(),
            ))
        })?;

        Ok(store.len())
    }
}

/// Builder for creating transient tokens
pub struct TransientTokenBuilder {
    tokenable_type: String,
    tokenable_id: i64,
    name: String,
    abilities: Vec<String>,
    expires_at: Option<DateTime<Utc>>,
}

impl TransientTokenBuilder {
    /// Create a new transient token builder
    pub fn new(tokenable_type: impl Into<String>, tokenable_id: i64, name: impl Into<String>) -> Self {
        Self {
            tokenable_type: tokenable_type.into(),
            tokenable_id,
            name: name.into(),
            abilities: Vec::new(),
            expires_at: None,
        }
    }

    /// Add abilities to the token
    pub fn with_abilities(mut self, abilities: Vec<String>) -> Self {
        self.abilities = abilities;
        self
    }

    /// Set expiration for the token
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Build the transient token
    pub fn build(self) -> (String, PersonalAccessToken) {
        let plain_token = PersonalAccessToken::generate_token();
        let hashed_token = PersonalAccessToken::hash_token(&plain_token);

        let token = PersonalAccessToken {
            id: 0, // Transient tokens don't have IDs
            tokenable_type: self.tokenable_type,
            tokenable_id: self.tokenable_id,
            name: self.name,
            token: hashed_token,
            abilities: self.abilities,
            last_used_at: None,
            expires_at: self.expires_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_agent: None,
            last_used_ip: None,
        };

        (plain_token, token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_transient_store() {
        let store = TransientTokenStore::new();

        let (_plain, token) = TransientTokenBuilder::new("User", 1, "test-token")
            .with_abilities(vec!["read:posts".to_string()])
            .build();

        // Store token
        store.store(token.clone()).unwrap();

        // Find token
        let found = store.find(&token.token).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-token");

        // Count
        assert_eq!(store.count().unwrap(), 1);

        // Remove token
        store.remove(&token.token).unwrap();
        assert!(store.find(&token.token).unwrap().is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let store = TransientTokenStore::new();

        // Add expired token
        let (_, expired_token) = TransientTokenBuilder::new("User", 1, "expired")
            .with_expiration(Utc::now() - Duration::hours(1))
            .build();
        store.store(expired_token).unwrap();

        // Add valid token
        let (_, valid_token) = TransientTokenBuilder::new("User", 1, "valid")
            .with_expiration(Utc::now() + Duration::hours(1))
            .build();
        store.store(valid_token.clone()).unwrap();

        // Cleanup
        let removed = store.cleanup_expired().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(store.count().unwrap(), 1);

        // Valid token should still exist
        let found = store.find(&valid_token.token).unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn test_find_by_tokenable() {
        let store = TransientTokenStore::new();

        // Add tokens for different users
        let (_, token1) = TransientTokenBuilder::new("User", 1, "user1-token1").build();
        let (_, token2) = TransientTokenBuilder::new("User", 1, "user1-token2").build();
        let (_, token3) = TransientTokenBuilder::new("User", 2, "user2-token1").build();

        store.store(token1).unwrap();
        store.store(token2).unwrap();
        store.store(token3).unwrap();

        // Find tokens for user 1
        let user1_tokens = store.find_by_tokenable("User", 1).unwrap();
        assert_eq!(user1_tokens.len(), 2);

        // Find tokens for user 2
        let user2_tokens = store.find_by_tokenable("User", 2).unwrap();
        assert_eq!(user2_tokens.len(), 1);
    }

    #[test]
    fn test_remove_all_for_tokenable() {
        let store = TransientTokenStore::new();

        let (_, token1) = TransientTokenBuilder::new("User", 1, "token1").build();
        let (_, token2) = TransientTokenBuilder::new("User", 1, "token2").build();
        let (_, token3) = TransientTokenBuilder::new("User", 2, "token3").build();

        store.store(token1).unwrap();
        store.store(token2).unwrap();
        store.store(token3).unwrap();

        assert_eq!(store.count().unwrap(), 3);

        // Remove all tokens for user 1
        store.remove_all_for_tokenable("User", 1).unwrap();
        assert_eq!(store.count().unwrap(), 1);

        // User 2's token should still exist
        let user2_tokens = store.find_by_tokenable("User", 2).unwrap();
        assert_eq!(user2_tokens.len(), 1);
    }

    #[test]
    fn test_touch() {
        let store = TransientTokenStore::new();

        let (_, token) = TransientTokenBuilder::new("User", 1, "test").build();
        let hashed = token.token.clone();

        store.store(token).unwrap();

        // Initial last_used_at should be None
        let found = store.find(&hashed).unwrap().unwrap();
        assert!(found.last_used_at.is_none());

        // Touch the token
        store.touch(&hashed).unwrap();

        // last_used_at should now be set
        let found = store.find(&hashed).unwrap().unwrap();
        assert!(found.last_used_at.is_some());
    }
}
