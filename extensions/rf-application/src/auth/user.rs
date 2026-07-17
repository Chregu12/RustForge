use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use sha2::{Digest, Sha256};

use super::guard::{AuthError, Authenticatable, Credentials, Provider};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: PasswordHash,
    pub is_active: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, name: String, password_hash: PasswordHash) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            email,
            name,
            password_hash,
            is_active: true,
            email_verified_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
}

impl Authenticatable for User {
    fn get_auth_id(&self) -> i64 {
        self.id
    }

    fn get_password_hash(&self) -> PasswordHash {
        self.password_hash.clone()
    }

    fn is_active(&self) -> bool {
        self.is_active && self.is_email_verified()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordHash {
    salt_hex: String,
    hash_hex: String,
}

impl PasswordHash {
    pub fn raw(value: impl Into<String>) -> Self {
        Self {
            salt_hex: String::new(),
            hash_hex: value.into(),
        }
    }

    pub fn hash(password: &str) -> Result<Self, AuthError> {
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        let salt_hex = hex::encode(salt);
        let mut hasher = Sha256::new();
        hasher.update(&salt_hex);
        hasher.update(password.as_bytes());
        let hash_hex = hex::encode(hasher.finalize());
        Ok(Self { salt_hex, hash_hex })
    }

    pub fn verify(&self, password: &str) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&self.salt_hex);
        hasher.update(password.as_bytes());
        hex::encode(hasher.finalize()) == self.hash_hex
    }
}

impl From<String> for PasswordHash {
    fn from(value: String) -> Self {
        Self {
            salt_hex: String::new(),
            hash_hex: value,
        }
    }
}

impl AsRef<str> for PasswordHash {
    fn as_ref(&self) -> &str {
        &self.hash_hex
    }
}

#[derive(Clone, Default)]
pub struct InMemoryUserProvider {
    users: Arc<RwLock<Vec<User>>>,
}

impl InMemoryUserProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_users(users: Vec<User>) -> Self {
        Self {
            users: Arc::new(RwLock::new(users)),
        }
    }

    pub fn push(&self, mut user: User) {
        let mut guard = self.users.write().unwrap();
        let next_id = guard.len() as i64 + 1;
        user.id = next_id;
        guard.push(user);
    }
}

#[async_trait]
impl Provider for InMemoryUserProvider {
    type User = User;

    async fn retrieve_by_id(&self, id: i64) -> Result<Option<Self::User>, AuthError> {
        Ok(self
            .users
            .read()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned())
    }

    async fn retrieve_by_credentials(
        &self,
        credentials: &Credentials,
    ) -> Result<Option<Self::User>, AuthError> {
        Ok(self
            .users
            .read()
            .unwrap()
            .iter()
            .find(|user| user.email == credentials.email)
            .cloned())
    }

    async fn validate_credentials(&self, user: &Self::User, password: &str) -> bool {
        user.password_hash.verify(password)
    }
}
