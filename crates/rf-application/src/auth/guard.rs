use async_trait::async_trait;
use serde::Deserialize;
use thiserror::Error;

use super::user::PasswordHash;

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("unauthorized")]
    Unauthorized,
    #[error("session expired")]
    SessionExpired,
    #[error("internal authentication failure: {0}")]
    Internal(String),
}

#[async_trait]
pub trait Guard: Send + Sync {
    type User: Authenticatable;

    async fn check(&self) -> bool;
    async fn user(&self) -> Option<Self::User>;
    async fn attempt(&self, credentials: Credentials) -> Result<Self::User, AuthError>;
    async fn login(&self, user: Self::User) -> Result<(), AuthError>;
    async fn logout(&self) -> Result<(), AuthError>;
}

#[async_trait]
pub trait Provider: Send + Sync {
    type User: Authenticatable;

    async fn retrieve_by_id(&self, id: i64) -> Result<Option<Self::User>, AuthError>;
    async fn retrieve_by_credentials(
        &self,
        credentials: &Credentials,
    ) -> Result<Option<Self::User>, AuthError>;
    async fn validate_credentials(&self, user: &Self::User, password: &str) -> bool;
}

pub trait Authenticatable: Clone + Send + Sync + 'static {
    fn get_auth_id(&self) -> i64;
    fn get_password_hash(&self) -> PasswordHash;
    fn is_active(&self) -> bool {
        true
    }
}
