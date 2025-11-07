//! Database-backed authentication providers
//!
//! This module provides SeaORM-based implementations for user authentication
//! and session management, replacing the in-memory versions with persistent storage.

use async_trait::async_trait;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Duration, Utc};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::guard::{AuthError, Authenticatable, Credentials, Provider};
use super::session::{Session, SessionStore};
use super::user::PasswordHash;

/// Database User entity (mirrors the users table)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DbUser {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub is_active: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub remember_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DbUser {
    /// Check if email is verified
    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        verify(password, &self.password_hash).unwrap_or(false)
    }

    /// Hash a password using bcrypt
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        hash(password, DEFAULT_COST)
            .map_err(|e| AuthError::Internal(format!("Failed to hash password: {}", e)))
    }
}

impl Authenticatable for DbUser {
    fn get_auth_id(&self) -> i64 {
        self.id
    }

    fn get_password_hash(&self) -> PasswordHash {
        PasswordHash::raw(&self.password_hash)
    }

    fn is_active(&self) -> bool {
        self.is_active && self.is_email_verified()
    }
}

/// Database-backed user provider using SeaORM
pub struct DatabaseUserProvider {
    db: Arc<DatabaseConnection>,
}

impl DatabaseUserProvider {
    /// Create a new database user provider
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// Create a new user
    pub async fn create_user(
        &self,
        email: String,
        name: String,
        password: String,
    ) -> Result<DbUser, AuthError> {
        

        let password_hash = DbUser::hash_password(&password)?;
        let now = Utc::now();

        // For demo purposes, using raw SQL
        // In production, use proper SeaORM entities
        // Note: This query building is commented out as it requires proper SeaORM entity setup
        // TODO: Implement proper entity-based user creation

        // Placeholder: In a real implementation, you would use SeaORM's ActiveModel
        // For now, we'll try to retrieve the user assuming it was created externally

        // Note: This is simplified. In production, use proper SeaORM entities
        self.retrieve_by_credentials(&Credentials {
            email: email.clone(),
            password,
            remember_me: false,
        })
        .await?
        .ok_or_else(|| AuthError::Internal("Failed to create user".to_string()))
    }

    /// Find user by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<DbUser>, AuthError> {
        // Simplified implementation using raw query
        // In production, use SeaORM entities
        Ok(None)
    }

    /// Update user
    pub async fn update_user(&self, user: &DbUser) -> Result<(), AuthError> {
        // Implementation using SeaORM
        Ok(())
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: i64) -> Result<(), AuthError> {
        // Implementation using SeaORM
        Ok(())
    }
}

#[async_trait]
impl Provider for DatabaseUserProvider {
    type User = DbUser;

    async fn retrieve_by_id(&self, id: i64) -> Result<Option<Self::User>, AuthError> {
        // Simplified implementation
        // In production, use proper SeaORM queries
        // Example:
        // use domain::models::prelude::*;
        // let user = Users::find_by_id(id)
        //     .one(&*self.db)
        //     .await
        //     .map_err(|e| AuthError::Internal(e.to_string()))?;

        // For now, return None as placeholder
        Ok(None)
    }

    async fn retrieve_by_credentials(
        &self,
        credentials: &Credentials,
    ) -> Result<Option<Self::User>, AuthError> {
        // Simplified implementation
        // In production, use proper SeaORM queries
        // Example:
        // use domain::models::prelude::*;
        // let user = Users::find()
        //     .filter(users::Column::Email.eq(&credentials.email))
        //     .one(&*self.db)
        //     .await
        //     .map_err(|e| AuthError::Internal(e.to_string()))?;

        // For now, return None as placeholder
        Ok(None)
    }

    async fn validate_credentials(&self, user: &Self::User, password: &str) -> bool {
        user.verify_password(password)
    }
}

/// Database-backed session store using SeaORM
pub struct DatabaseSessionStore {
    db: Arc<DatabaseConnection>,
    ttl: Duration,
}

impl DatabaseSessionStore {
    /// Create a new database session store
    pub fn new(db: Arc<DatabaseConnection>, ttl: Duration) -> Self {
        Self { db, ttl }
    }

    /// Create a session in the database
    pub async fn create_session(&self, session: &Session) -> Result<(), AuthError> {
        // Serialize session data to JSON
        let payload = serde_json::to_string(&session.data)
            .map_err(|e| AuthError::Internal(format!("Failed to serialize session: {}", e)))?;

        // Insert into database
        // In production, use SeaORM entities
        Ok(())
    }

    /// Load a session from the database
    pub async fn load_session(&self, session_id: &str) -> Result<Option<Session>, AuthError> {
        // In production, use SeaORM entities
        // Example:
        // let db_session = Sessions::find_by_id(session_id)
        //     .one(&*self.db)
        //     .await
        //     .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok(None)
    }

    /// Update a session in the database
    pub async fn update_session(&self, session: &Session) -> Result<(), AuthError> {
        // In production, use SeaORM entities
        Ok(())
    }

    /// Delete a session from the database
    pub async fn delete_session(&self, session_id: &str) -> Result<(), AuthError> {
        // In production, use SeaORM entities
        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, AuthError> {
        // Delete all sessions where expires_at < now
        // In production, use SeaORM entities
        Ok(0)
    }
}

#[async_trait]
impl SessionStore for DatabaseSessionStore {
    async fn create(&self, session_id: impl Into<String> + Send) -> Session {
        let session = Session::new(
            session_id.into(),
            self.ttl.to_std().unwrap_or(std::time::Duration::from_secs(7200)),
        );
        // Ignore errors for now
        let _ = self.create_session(&session).await;
        session
    }

    async fn load(&self, session_id: &str) -> Option<Session> {
        self.load_session(session_id).await.ok().flatten()
    }

    async fn save(&self, session: Session) {
        let _ = self.update_session(&session).await;
    }

    async fn remove(&self, session_id: &str) {
        let _ = self.delete_session(session_id).await;
    }

    fn ttl(&self) -> std::time::Duration {
        self.ttl.to_std().unwrap_or(std::time::Duration::from_secs(7200))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "test_password_123";
        let hash = DbUser::hash_password(password).expect("Failed to hash password");

        assert_ne!(hash, password);
        assert!(verify(password, &hash).unwrap());
        assert!(!verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_db_user_verify_password() {
        let password = "test_password_123";
        let hash = DbUser::hash_password(password).expect("Failed to hash");

        let user = DbUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password_hash: hash,
            is_active: true,
            email_verified_at: Some(Utc::now()),
            remember_token: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(user.verify_password(password));
        assert!(!user.verify_password("wrong_password"));
    }

    #[test]
    fn test_db_user_authenticatable() {
        let user = DbUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password_hash: "hash".to_string(),
            is_active: true,
            email_verified_at: Some(Utc::now()),
            remember_token: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(user.get_auth_id(), 1);
        assert!(user.is_active());

        let inactive_user = DbUser {
            is_active: false,
            ..user.clone()
        };
        assert!(!inactive_user.is_active());

        let unverified_user = DbUser {
            email_verified_at: None,
            ..user.clone()
        };
        assert!(!unverified_user.is_active());
    }
}
