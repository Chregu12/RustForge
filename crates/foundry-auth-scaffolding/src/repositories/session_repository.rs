//! Session Repository - PostgreSQL Implementation
//!
//! Provides database-backed session storage

use crate::models::{Session, PasswordReset, EmailVerification};
use crate::repositories::user_repository::{RepositoryError, RepositoryResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Session Repository Trait
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create_session(&self, session: Session) -> RepositoryResult<Session>;
    async fn find_session_by_token(&self, token: &str) -> RepositoryResult<Option<Session>>;
    async fn find_session_by_id(&self, id: Uuid) -> RepositoryResult<Option<Session>>;
    async fn find_sessions_by_user(&self, user_id: Uuid) -> RepositoryResult<Vec<Session>>;
    async fn update_session(&self, session: Session) -> RepositoryResult<Session>;
    async fn delete_session(&self, id: Uuid) -> RepositoryResult<()>;
    async fn delete_user_sessions(&self, user_id: Uuid) -> RepositoryResult<()>;
    async fn delete_expired_sessions(&self) -> RepositoryResult<usize>;
}

/// Password Reset Repository Trait
#[async_trait]
pub trait PasswordResetRepository: Send + Sync {
    async fn create_reset(&self, reset: PasswordReset) -> RepositoryResult<PasswordReset>;
    async fn find_reset_by_token(&self, token: &str) -> RepositoryResult<Option<PasswordReset>>;
    async fn find_reset_by_email(&self, email: &str) -> RepositoryResult<Option<PasswordReset>>;
    async fn delete_reset(&self, id: Uuid) -> RepositoryResult<()>;
    async fn delete_resets_by_email(&self, email: &str) -> RepositoryResult<()>;
    async fn delete_expired_resets(&self) -> RepositoryResult<usize>;
}

/// Email Verification Repository Trait
#[async_trait]
pub trait EmailVerificationRepository: Send + Sync {
    async fn create_verification(&self, verification: EmailVerification) -> RepositoryResult<EmailVerification>;
    async fn find_verification_by_token(&self, token: &str) -> RepositoryResult<Option<EmailVerification>>;
    async fn find_verification_by_user(&self, user_id: Uuid) -> RepositoryResult<Option<EmailVerification>>;
    async fn delete_verification(&self, id: Uuid) -> RepositoryResult<()>;
    async fn delete_verifications_by_user(&self, user_id: Uuid) -> RepositoryResult<()>;
    async fn delete_expired_verifications(&self) -> RepositoryResult<usize>;
}

/// PostgreSQL Session Repository
pub struct PostgresSessionRepository {
    pool: PgPool,
}

impl PostgresSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_session(&self, row: &sqlx::postgres::PgRow) -> RepositoryResult<Session> {
        Ok(Session {
            id: row.try_get("id").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            user_id: row.try_get("user_id").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            token: row.try_get("token").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            ip_address: row.try_get("ip_address").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            user_agent: row.try_get("user_agent").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            last_activity: row.try_get("last_activity").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
        })
    }
}

#[async_trait]
impl SessionRepository for PostgresSessionRepository {
    async fn create_session(&self, session: Session) -> RepositoryResult<Session> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, token, ip_address, user_agent, last_activity, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(session.id)
        .bind(session.user_id)
        .bind(&session.token)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.last_activity)
        .bind(session.expires_at)
        .bind(session.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to insert session: {}", e)))?;

        Ok(session)
    }

    async fn find_session_by_token(&self, token: &str) -> RepositoryResult<Option<Session>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, ip_address, user_agent, last_activity, expires_at, created_at
             FROM sessions
             WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_session(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_session_by_id(&self, id: Uuid) -> RepositoryResult<Option<Session>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, ip_address, user_agent, last_activity, expires_at, created_at
             FROM sessions
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_session(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_sessions_by_user(&self, user_id: Uuid) -> RepositoryResult<Vec<Session>> {
        let rows = sqlx::query(
            "SELECT id, user_id, token, ip_address, user_agent, last_activity, expires_at, created_at
             FROM sessions
             WHERE user_id = $1
             ORDER BY last_activity DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        rows.iter()
            .map(|row| self.row_to_session(row))
            .collect()
    }

    async fn update_session(&self, session: Session) -> RepositoryResult<Session> {
        let result = sqlx::query(
            "UPDATE sessions
             SET ip_address = $2, user_agent = $3, last_activity = $4, expires_at = $5
             WHERE id = $1"
        )
        .bind(session.id)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.last_activity)
        .bind(session.expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to update session: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(session)
    }

    async fn delete_session(&self, id: Uuid) -> RepositoryResult<()> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete session: {}", e)))?;

        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: Uuid) -> RepositoryResult<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete user sessions: {}", e)))?;

        Ok(())
    }

    async fn delete_expired_sessions(&self) -> RepositoryResult<usize> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < $1")
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete expired sessions: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }
}

/// PostgreSQL Password Reset Repository
pub struct PostgresPasswordResetRepository {
    pool: PgPool,
}

impl PostgresPasswordResetRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_password_reset(&self, row: &sqlx::postgres::PgRow) -> RepositoryResult<PasswordReset> {
        Ok(PasswordReset {
            id: row.try_get("id").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            email: row.try_get("email").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            token: row.try_get("token").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
        })
    }
}

#[async_trait]
impl PasswordResetRepository for PostgresPasswordResetRepository {
    async fn create_reset(&self, reset: PasswordReset) -> RepositoryResult<PasswordReset> {
        sqlx::query(
            "INSERT INTO password_resets (id, email, token, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(reset.id)
        .bind(&reset.email)
        .bind(&reset.token)
        .bind(reset.expires_at)
        .bind(reset.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to insert password reset: {}", e)))?;

        Ok(reset)
    }

    async fn find_reset_by_token(&self, token: &str) -> RepositoryResult<Option<PasswordReset>> {
        let row = sqlx::query(
            "SELECT id, email, token, expires_at, created_at
             FROM password_resets
             WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_password_reset(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_reset_by_email(&self, email: &str) -> RepositoryResult<Option<PasswordReset>> {
        let row = sqlx::query(
            "SELECT id, email, token, expires_at, created_at
             FROM password_resets
             WHERE email = $1
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_password_reset(&row)?)),
            None => Ok(None),
        }
    }

    async fn delete_reset(&self, id: Uuid) -> RepositoryResult<()> {
        sqlx::query("DELETE FROM password_resets WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete password reset: {}", e)))?;

        Ok(())
    }

    async fn delete_resets_by_email(&self, email: &str) -> RepositoryResult<()> {
        sqlx::query("DELETE FROM password_resets WHERE email = $1")
            .bind(email)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete password resets: {}", e)))?;

        Ok(())
    }

    async fn delete_expired_resets(&self) -> RepositoryResult<usize> {
        let result = sqlx::query("DELETE FROM password_resets WHERE expires_at < $1")
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete expired resets: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }
}

/// PostgreSQL Email Verification Repository
pub struct PostgresEmailVerificationRepository {
    pool: PgPool,
}

impl PostgresEmailVerificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_email_verification(&self, row: &sqlx::postgres::PgRow) -> RepositoryResult<EmailVerification> {
        Ok(EmailVerification {
            id: row.try_get("id").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            user_id: row.try_get("user_id").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            token: row.try_get("token").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| RepositoryError::DatabaseError(e.to_string()))?,
        })
    }
}

#[async_trait]
impl EmailVerificationRepository for PostgresEmailVerificationRepository {
    async fn create_verification(&self, verification: EmailVerification) -> RepositoryResult<EmailVerification> {
        sqlx::query(
            "INSERT INTO email_verifications (id, user_id, token, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(verification.id)
        .bind(verification.user_id)
        .bind(&verification.token)
        .bind(verification.expires_at)
        .bind(verification.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(format!("Failed to insert email verification: {}", e)))?;

        Ok(verification)
    }

    async fn find_verification_by_token(&self, token: &str) -> RepositoryResult<Option<EmailVerification>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, expires_at, created_at
             FROM email_verifications
             WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_email_verification(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_verification_by_user(&self, user_id: Uuid) -> RepositoryResult<Option<EmailVerification>> {
        let row = sqlx::query(
            "SELECT id, user_id, token, expires_at, created_at
             FROM email_verifications
             WHERE user_id = $1
             ORDER BY created_at DESC
             LIMIT 1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepositoryError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(Some(self.row_to_email_verification(&row)?)),
            None => Ok(None),
        }
    }

    async fn delete_verification(&self, id: Uuid) -> RepositoryResult<()> {
        sqlx::query("DELETE FROM email_verifications WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete email verification: {}", e)))?;

        Ok(())
    }

    async fn delete_verifications_by_user(&self, user_id: Uuid) -> RepositoryResult<()> {
        sqlx::query("DELETE FROM email_verifications WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete email verifications: {}", e)))?;

        Ok(())
    }

    async fn delete_expired_verifications(&self) -> RepositoryResult<usize> {
        let result = sqlx::query("DELETE FROM email_verifications WHERE expires_at < $1")
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .map_err(|e| RepositoryError::DatabaseError(format!("Failed to delete expired verifications: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would require database setup
    // Placeholder for integration tests
}
