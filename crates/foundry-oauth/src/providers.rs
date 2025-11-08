//! OAuth provider implementations
//!
//! Note: These are stub implementations. In production, you should implement
//! actual HTTP requests to the OAuth providers' endpoints.

use async_trait::async_trait;
use crate::{OAuthProvider, OAuthUser, OAuthTokens, Result};

/// Google OAuth provider
pub struct GoogleProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GoogleProvider {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
             client_id={}&\
             redirect_uri={}&\
             response_type=code&\
             scope=openid%20email%20profile&\
             state={}&\
             access_type=offline",
            self.client_id, self.redirect_uri, state
        )
    }

    async fn exchange_code(&self, _code: &str) -> Result<OAuthTokens> {
        // TODO: Implement actual HTTP request to:
        // POST https://oauth2.googleapis.com/token
        // with code, client_id, client_secret, redirect_uri, grant_type=authorization_code

        // Stub implementation
        Ok(OAuthTokens {
            access_token: "google_access_token".to_string(),
            refresh_token: Some("google_refresh_token".to_string()),
            expires_in: Some(3600),
            token_type: "Bearer".to_string(),
        })
    }

    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        // TODO: Implement actual HTTP request to:
        // GET https://www.googleapis.com/oauth2/v2/userinfo
        // with Authorization: Bearer {token}

        // Stub implementation
        Ok(OAuthUser {
            provider: "google".to_string(),
            provider_id: "google_user_id".to_string(),
            email: Some("user@gmail.com".to_string()),
            name: Some("Google User".to_string()),
            avatar: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens> {
        // TODO: Implement actual HTTP request to:
        // POST https://oauth2.googleapis.com/token
        // with refresh_token, client_id, client_secret, grant_type=refresh_token

        Ok(OAuthTokens {
            access_token: "new_google_access_token".to_string(),
            refresh_token: Some("google_refresh_token".to_string()),
            expires_in: Some(3600),
            token_type: "Bearer".to_string(),
        })
    }
}

/// GitHub OAuth provider
pub struct GithubProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl GithubProvider {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[async_trait]
impl OAuthProvider for GithubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?\
             client_id={}&\
             redirect_uri={}&\
             scope=user:email&\
             state={}",
            self.client_id, self.redirect_uri, state
        )
    }

    async fn exchange_code(&self, _code: &str) -> Result<OAuthTokens> {
        // TODO: Implement actual HTTP request to:
        // POST https://github.com/login/oauth/access_token
        // with code, client_id, client_secret

        Ok(OAuthTokens {
            access_token: "github_access_token".to_string(),
            refresh_token: None, // GitHub doesn't provide refresh tokens
            expires_in: None,    // GitHub tokens don't expire
            token_type: "Bearer".to_string(),
        })
    }

    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        // TODO: Implement actual HTTP requests to:
        // GET https://api.github.com/user
        // GET https://api.github.com/user/emails

        Ok(OAuthUser {
            provider: "github".to_string(),
            provider_id: "github_user_id".to_string(),
            email: Some("user@github.com".to_string()),
            name: Some("GitHub User".to_string()),
            avatar: None,
        })
    }
}

/// Facebook OAuth provider
pub struct FacebookProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl FacebookProvider {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
        }
    }
}

#[async_trait]
impl OAuthProvider for FacebookProvider {
    fn name(&self) -> &'static str {
        "facebook"
    }

    fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://www.facebook.com/v12.0/dialog/oauth?\
             client_id={}&\
             redirect_uri={}&\
             scope=email,public_profile&\
             state={}",
            self.client_id, self.redirect_uri, state
        )
    }

    async fn exchange_code(&self, _code: &str) -> Result<OAuthTokens> {
        // TODO: Implement actual HTTP request to:
        // GET https://graph.facebook.com/v12.0/oauth/access_token
        // with code, client_id, client_secret, redirect_uri

        Ok(OAuthTokens {
            access_token: "facebook_access_token".to_string(),
            refresh_token: None,
            expires_in: Some(5184000), // Facebook tokens typically expire in 60 days
            token_type: "Bearer".to_string(),
        })
    }

    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        // TODO: Implement actual HTTP request to:
        // GET https://graph.facebook.com/me?fields=id,name,email,picture
        // with access_token parameter

        Ok(OAuthUser {
            provider: "facebook".to_string(),
            provider_id: "facebook_user_id".to_string(),
            email: Some("user@facebook.com".to_string()),
            name: Some("Facebook User".to_string()),
            avatar: None,
        })
    }
}
