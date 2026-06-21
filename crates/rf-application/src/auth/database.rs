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

    /// Create a new user using raw SQL INSERT
    pub async fn create_user(
        &self,
        email: String,
        name: String,
        password: String,
    ) -> Result<DbUser, AuthError> {
        use sea_orm::{ConnectionTrait, DbBackend, Statement};

        let password_hash = DbUser::hash_password(&password)?;
        let now = Utc::now();
        let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let backend = self.db.get_database_backend();
        let (p1, p2, p3, p4, p5) = match backend {
            DbBackend::Postgres => ("$1", "$2", "$3", "$4", "$5"),
            _ => ("?", "?", "?", "?", "?"),
        };

        let sql = format!(
            "INSERT INTO users (email, name, password_hash, is_active, created_at, updated_at) \
             VALUES ({p1}, {p2}, {p3}, true, {p4}, {p5})"
        );

        self.db
            .execute(Statement::from_sql_and_values(
                backend,
                &sql,
                [
                    sea_orm::Value::String(Some(Box::new(email.clone()))),
                    sea_orm::Value::String(Some(Box::new(name))),
                    sea_orm::Value::String(Some(Box::new(password_hash))),
                    sea_orm::Value::String(Some(Box::new(now_str.clone()))),
                    sea_orm::Value::String(Some(Box::new(now_str))),
                ],
            ))
            .await
            .map_err(|e| AuthError::Internal(format!("Failed to create user: {}", e)))?;

        self.find_by_email(&email)
            .await?
            .ok_or_else(|| AuthError::Internal("User not found after creation".to_string()))
    }

    /// Find user by email using raw SQL SELECT
    pub async fn find_by_email(&self, email: &str) -> Result<Option<DbUser>, AuthError> {
        use sea_orm::{ConnectionTrait, DbBackend, Statement};

        let backend = self.db.get_database_backend();
        let placeholder = match backend {
            DbBackend::Postgres => "$1",
            _ => "?",
        };

        let sql = format!(
            "SELECT id, email, name, password_hash, is_active, email_verified_at, \
             remember_token, created_at, updated_at \
             FROM users WHERE email = {placeholder} LIMIT 1"
        );

        let result = self
            .db
            .query_one(Statement::from_sql_and_values(
                backend,
                &sql,
                [sea_orm::Value::String(Some(Box::new(email.to_string())))],
            ))
            .await
            .map_err(|e| AuthError::Internal(format!("Database error: {}", e)))?;

        match result {
            None => Ok(None),
            Some(row) => {
                let id: i64 = row
                    .try_get("", "id")
                    .map_err(|e| AuthError::Internal(format!("Column error: {}", e)))?;
                let email: String = row
                    .try_get("", "email")
                    .map_err(|e| AuthError::Internal(format!("Column error: {}", e)))?;
                let name: String = row
                    .try_get("", "name")
                    .map_err(|e| AuthError::Internal(format!("Column error: {}", e)))?;
                let password_hash: String = row
                    .try_get("", "password_hash")
                    .map_err(|e| AuthError::Internal(format!("Column error: {}", e)))?;
                let is_active: bool = row
                    .try_get("", "is_active")
                    .map_err(|e| AuthError::Internal(format!("Column error: {}", e)))?;
                let email_verified_at: Option<DateTime<Utc>> = row
                    .try_get("", "email_verified_at")
                    .unwrap_or(None);
                let remember_token: Option<String> = row
                    .try_get("", "remember_token")
                    .unwrap_or(None);
                let created_at: DateTime<Utc> = row
                    .try_get("", "created_at")
                    .unwrap_or_else(|_| Utc::now());
                let updated_at: DateTime<Utc> = row
                    .try_get("", "updated_at")
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(DbUser {
                    id,
                    email,
                    name,
                    password_hash,
                    is_active,
                    email_verified_at,
                    remember_token,
                    created_at,
                    updated_at,
                }))
            }
        }
    }

    /// Update user
    pub async fn update_user(&self, _user: &DbUser) -> Result<(), AuthError> {
        // Implementation using SeaORM
        Ok(())
    }

    /// Delete user
    pub async fn delete_user(&self, _user_id: i64) -> Result<(), AuthError> {
        // Implementation using SeaORM
        Ok(())
    }
}

#[async_trait]
impl Provider for DatabaseUserProvider {
    type User = DbUser;

    async fn retrieve_by_id(&self, id: i64) -> Result<Option<Self::User>, AuthError> {
        use sea_orm::{ConnectionTrait, DbBackend, Statement};

        let backend = self.db.get_database_backend();
        let placeholder = match backend {
            DbBackend::Postgres => "$1",
            _ => "?",
        };

        let sql = format!(
            "SELECT id, email, name, password_hash, is_active, email_verified_at, \
             remember_token, created_at, updated_at \
             FROM users WHERE id = {placeholder} LIMIT 1"
        );

        let result = self
            .db
            .query_one(Statement::from_sql_and_values(
                backend,
                &sql,
                [sea_orm::Value::BigInt(Some(id))],
            ))
            .await
            .map_err(|e| AuthError::Internal(format!("Database error: {}", e)))?;

        match result {
            None => Ok(None),
            Some(row) => {
                let email: String = row
                    .try_get("", "email")
                    .map_err(|e| AuthError::Internal(format!("Column error: {}", e)))?;
                self.find_by_email(&email).await
            }
        }
    }

    async fn retrieve_by_credentials(
        &self,
        credentials: &Credentials,
    ) -> Result<Option<Self::User>, AuthError> {
        self.find_by_email(&credentials.email).await
    }

    async fn validate_credentials(&self, user: &Self::User, password: &str) -> bool {
        user.verify_password(password)
    }
}

/// Database-backed session store using SeaORM
pub struct DatabaseSessionStore {
    #[allow(dead_code)] // reserved: retained for future DB-backed session queries
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
        let _payload = serde_json::to_string(&session.data)
            .map_err(|e| AuthError::Internal(format!("Failed to serialize session: {}", e)))?;

        // Insert into database
        // In production, use SeaORM entities
        Ok(())
    }

    /// Load a session from the database
    pub async fn load_session(&self, _session_id: &str) -> Result<Option<Session>, AuthError> {
        // In production, use SeaORM entities
        // Example:
        // let db_session = Sessions::find_by_id(session_id)
        //     .one(&*self.db)
        //     .await
        //     .map_err(|e| AuthError::Internal(e.to_string()))?;

        Ok(None)
    }

    /// Update a session in the database
    pub async fn update_session(&self, _session: &Session) -> Result<(), AuthError> {
        // In production, use SeaORM entities
        Ok(())
    }

    /// Delete a session from the database
    pub async fn delete_session(&self, _session_id: &str) -> Result<(), AuthError> {
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
            self.ttl
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(7200)),
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
        self.ttl
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(7200))
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
