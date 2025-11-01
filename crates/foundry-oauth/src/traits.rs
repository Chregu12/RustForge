//! OAuth traits

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUser {
    pub provider: String,
    pub provider_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn authorize_url(&self) -> String;
    async fn exchange_code(&self, code: &str) -> crate::Result<String>;
    async fn get_user(&self, token: &str) -> crate::Result<OAuthUser>;
}
