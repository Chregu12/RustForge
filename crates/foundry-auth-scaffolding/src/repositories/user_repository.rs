//! User Repository - PostgreSQL Implementation
//!
//! Provides database-backed user storage

use crate::models::User;
use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

/// Repository Error
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("User not found")]
    NotFound,

    #[error("User already exists")]
    AlreadyExists,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// User Repository Trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> RepositoryResult<Option<User>>;
    async fn find_by_email(&self, email: &str) -> RepositoryResult<Option<User>>;
    async fn create(&self, user: User) -> RepositoryResult<User>;
    async fn update(&self, user: User) -> RepositoryResult<User>;
    async fn delete(&self, id: Uuid) -> RepositoryResult<()>;
    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<User>>;
    async fn count(&self) -> RepositoryResult<i64>;
}

/// PostgreSQL User Repository
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_user(&self, row: &sqlx::postgres::PgRow) -> RepositoryResult<User> {
        let two_factor_recovery_codes_json: Option<String> = row.try_get("two_factor_recovery_codes")
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        let two_factor_recovery_codes = if let Some(json) = two_factor_recovery_codes_json {
            Some(serde_json::from_str(&json)
                .map_err(|e| RepositoryError::SerializationError(format!("Failed to parse recovery codes: {}", e)))?)
        } else {
            None
        };

        Ok(User {
            id: row.try_get("id").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            name: row.try_get("name").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            email: row.try_get("email").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            password_hash: row.try_get("password_hash").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            email_verified_at: row.try_get("email_verified_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            two_factor_secret: row.try_get("two_factor_secret").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            two_factor_recovery_codes,
            remember_token: row.try_get("remember_token").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            updated_at: row.try_get("updated_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
        })
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: Uuid) -> RepositoryResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, name, email, password_hash, email_verified_at, two_factor_secret, two_factor_recovery_codes, remember_token, created_at, updated_at
             FROM users
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> RepositoryResult<Option<User>> {
        let row = sqlx::query(
            "SELECT id, name, email, password_hash, email_verified_at, two_factor_secret, two_factor_recovery_codes, remember_token, created_at, updated_at
             FROM users
             WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn create(&self, user: User) -> RepositoryResult<User> {
        // Check if email already exists
        if let Some(_) = self.find_by_email(&user.email).await? {
            return Err(RepositoryError::AlreadyExists);
        }

        let two_factor_recovery_codes_json = if let Some(codes) = &user.two_factor_recovery_codes {
            Some(serde_json::to_string(codes)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?)
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO users (id, name, email, password_hash, email_verified_at, two_factor_secret, two_factor_recovery_codes, remember_token, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.email_verified_at)
        .bind(&user.two_factor_secret)
        .bind(two_factor_recovery_codes_json)
        .bind(&user.remember_token)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to insert user: {}", e)))?;

        Ok(user)
    }

    async fn update(&self, user: User) -> RepositoryResult<User> {
        let two_factor_recovery_codes_json = if let Some(codes) = &user.two_factor_recovery_codes {
            Some(serde_json::to_string(codes)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?)
        } else {
            None
        };

        let result = sqlx::query(
            "UPDATE users
             SET name = $2, email = $3, password_hash = $4, email_verified_at = $5, two_factor_secret = $6, two_factor_recovery_codes = $7, remember_token = $8, updated_at = $9
             WHERE id = $1"
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.email_verified_at)
        .bind(&user.two_factor_secret)
        .bind(two_factor_recovery_codes_json)
        .bind(&user.remember_token)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to update user: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(user)
    }

    async fn delete(&self, id: Uuid) -> RepositoryResult<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete user: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, name, email, password_hash, email_verified_at, two_factor_secret, two_factor_recovery_codes, remember_token, created_at, updated_at
             FROM users
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to list users: {}", e)))?;

        rows.iter()
            .map(|row| self.row_to_user(row))
            .collect()
    }

    async fn count(&self) -> RepositoryResult<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to count users: {}", e)))?;

        let count: i64 = row.try_get("count")
            .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        Ok(count)
    }
}

/// In-Memory User Repository (for testing/development)
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct InMemoryUserRepository {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    emails: Arc<RwLock<HashMap<String, Uuid>>>, // email -> user_id mapping
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            emails: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(&self, id: Uuid) -> RepositoryResult<Option<User>> {
        let users = self.users.read()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;
        Ok(users.get(&id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> RepositoryResult<Option<User>> {
        let emails = self.emails.read()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;

        if let Some(user_id) = emails.get(email) {
            let users = self.users.read()
                .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;
            Ok(users.get(user_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn create(&self, user: User) -> RepositoryResult<User> {
        // Check if email already exists
        if let Some(_) = self.find_by_email(&user.email).await? {
            return Err(RepositoryError::AlreadyExists);
        }

        let mut users = self.users.write()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;
        let mut emails = self.emails.write()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;

        emails.insert(user.email.clone(), user.id);
        users.insert(user.id, user.clone());

        Ok(user)
    }

    async fn update(&self, user: User) -> RepositoryResult<User> {
        let mut users = self.users.write()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;

        if !users.contains_key(&user.id) {
            return Err(RepositoryError::NotFound);
        }

        // Update email mapping if email changed
        if let Some(existing) = users.get(&user.id) {
            if existing.email != user.email {
                let mut emails = self.emails.write()
                    .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;
                emails.remove(&existing.email);
                emails.insert(user.email.clone(), user.id);
            }
        }

        users.insert(user.id, user.clone());
        Ok(user)
    }

    async fn delete(&self, id: Uuid) -> RepositoryResult<()> {
        let mut users = self.users.write()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;
        let mut emails = self.emails.write()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;

        if let Some(user) = users.remove(&id) {
            emails.remove(&user.email);
            Ok(())
        } else {
            Err(RepositoryError::NotFound)
        }
    }

    async fn list(&self, limit: i64, offset: i64) -> RepositoryResult<Vec<User>> {
        let users = self.users.read()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;

        let mut user_list: Vec<User> = users.values().cloned().collect();
        user_list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let start = offset as usize;
        let end = std::cmp::min(start + limit as usize, user_list.len());

        Ok(user_list[start..end].to_vec())
    }

    async fn count(&self) -> RepositoryResult<i64> {
        let users = self.users.read()
            .map_err(|e| RepositoryError::DatabaseError(format!("Lock poisoned: {}", e)))?;
        Ok(users.len() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_user_repository() {
        let repo = InMemoryUserRepository::new();

        let user = User::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "hashed_password".to_string(),
        );
        let user_id = user.id;

        // Create
        let created = repo.create(user.clone()).await.unwrap();
        assert_eq!(created.email, "john@example.com");

        // Find by ID
        let found = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(found.name, "John Doe");

        // Find by email
        let found = repo.find_by_email("john@example.com").await.unwrap().unwrap();
        assert_eq!(found.id, user_id);

        // Update
        let mut updated_user = found.clone();
        updated_user.name = "Jane Doe".to_string();
        repo.update(updated_user).await.unwrap();

        let found = repo.find_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(found.name, "Jane Doe");

        // Count
        let count = repo.count().await.unwrap();
        assert_eq!(count, 1);

        // Delete
        repo.delete(user_id).await.unwrap();
        let found = repo.find_by_id(user_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_email() {
        let repo = InMemoryUserRepository::new();

        let user1 = User::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "hash1".to_string(),
        );

        let user2 = User::new(
            "Jane Doe".to_string(),
            "john@example.com".to_string(),
            "hash2".to_string(),
        );

        repo.create(user1).await.unwrap();
        let result = repo.create(user2).await;
        assert!(matches!(result, Err(RepositoryError::AlreadyExists)));
    }
}
