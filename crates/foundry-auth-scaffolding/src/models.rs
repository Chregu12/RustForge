//! Authentication Data Models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub two_factor_secret: Option<String>,
    pub two_factor_recovery_codes: Option<Vec<String>>,
    pub remember_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(name: String, email: String, password_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            password_hash,
            email_verified_at: None,
            two_factor_secret: None,
            two_factor_recovery_codes: None,
            remember_token: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if email is verified
    pub fn has_verified_email(&self) -> bool {
        self.email_verified_at.is_some()
    }

    /// Check if two-factor is enabled
    pub fn has_two_factor(&self) -> bool {
        self.two_factor_secret.is_some()
    }

    /// Mark email as verified
    pub fn mark_email_as_verified(&mut self) {
        self.email_verified_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Enable two-factor authentication
    pub fn enable_two_factor(&mut self, secret: String, recovery_codes: Vec<String>) {
        self.two_factor_secret = Some(secret);
        self.two_factor_recovery_codes = Some(recovery_codes);
        self.updated_at = Utc::now();
    }

    /// Disable two-factor authentication
    pub fn disable_two_factor(&mut self) {
        self.two_factor_secret = None;
        self.two_factor_recovery_codes = None;
        self.updated_at = Utc::now();
    }
}

/// Session Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(user_id: Uuid, token: String, lifetime: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            token,
            ip_address: None,
            user_agent: None,
            last_activity: now,
            expires_at: now + chrono::Duration::seconds(lifetime),
            created_at: now,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Update last activity
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Password Reset Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordReset {
    pub id: Uuid,
    pub email: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl PasswordReset {
    pub fn new(email: String, token: String, lifetime: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            token,
            expires_at: now + chrono::Duration::seconds(lifetime),
            created_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Email Verification Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl EmailVerification {
    pub fn new(user_id: Uuid, token: String, lifetime: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            token,
            expires_at: now + chrono::Duration::seconds(lifetime),
            created_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "hashed_password".to_string(),
        );

        assert!(!user.has_verified_email());
        assert!(!user.has_two_factor());
    }

    #[test]
    fn test_email_verification() {
        let mut user = User::new(
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "hashed_password".to_string(),
        );

        user.mark_email_as_verified();
        assert!(user.has_verified_email());
    }

    #[test]
    fn test_session_expiry() {
        let session = Session::new(Uuid::new_v4(), "token123".to_string(), -60);
        assert!(session.is_expired());

        let session = Session::new(Uuid::new_v4(), "token123".to_string(), 3600);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_password_reset_expiry() {
        let reset = PasswordReset::new("user@example.com".to_string(), "token".to_string(), -60);
        assert!(reset.is_expired());
    }
}
