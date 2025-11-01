//! OAuth provider implementations

use async_trait::async_trait;
use crate::{OAuthProvider, OAuthUser, Result};

pub struct GoogleProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GoogleProvider {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self { client_id, client_secret, redirect_uri }
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &'static str { "google" }
    fn authorize_url(&self) -> String {
        format!("https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile",
            self.client_id, self.redirect_uri)
    }
    async fn exchange_code(&self, _code: &str) -> Result<String> {
        Ok(String::new())
    }
    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        Ok(OAuthUser {
            provider: "google".to_string(),
            provider_id: "".to_string(),
            email: None,
            name: None,
            avatar: None,
        })
    }
}

pub struct GithubProvider {
    client_id: String,
    client_secret: String,
}

impl GithubProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self { client_id, client_secret }
    }
}

#[async_trait]
impl OAuthProvider for GithubProvider {
    fn name(&self) -> &'static str { "github" }
    fn authorize_url(&self) -> String {
        format!("https://github.com/login/oauth/authorize?client_id={}&scope=user:email", self.client_id)
    }
    async fn exchange_code(&self, _code: &str) -> Result<String> {
        Ok(String::new())
    }
    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        Ok(OAuthUser {
            provider: "github".to_string(),
            provider_id: "".to_string(),
            email: None,
            name: None,
            avatar: None,
        })
    }
}

pub struct FacebookProvider {
    client_id: String,
    client_secret: String,
}

impl FacebookProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self { client_id, client_secret }
    }
}

#[async_trait]
impl OAuthProvider for FacebookProvider {
    fn name(&self) -> &'static str { "facebook" }
    fn authorize_url(&self) -> String {
        format!("https://www.facebook.com/v12.0/dialog/oauth?client_id={}&scope=email,public_profile", self.client_id)
    }
    async fn exchange_code(&self, _code: &str) -> Result<String> {
        Ok(String::new())
    }
    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        Ok(OAuthUser {
            provider: "facebook".to_string(),
            provider_id: "".to_string(),
            email: None,
            name: None,
            avatar: None,
        })
    }
}
