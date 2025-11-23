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

    async fn exchange_code(&self, code: &str) -> Result<OAuthTokens> {
        let client = reqwest::Client::new();

        let params = [
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("redirect_uri", &self.redirect_uri),
            ("grant_type", "authorization_code"),
        ];

        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct GoogleTokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<u64>,
            token_type: String,
        }

        let token_response: GoogleTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: token_response.expires_in,
            token_type: token_response.token_type,
        })
    }

    async fn get_user(&self, token: &str) -> Result<OAuthUser> {
        let client = reqwest::Client::new();

        let response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct GoogleUserInfo {
            id: String,
            email: Option<String>,
            name: Option<String>,
            picture: Option<String>,
        }

        let user_info: GoogleUserInfo = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        Ok(OAuthUser {
            provider: "google".to_string(),
            provider_id: user_info.id,
            email: user_info.email,
            name: user_info.name,
            avatar: user_info.picture,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens> {
        let client = reqwest::Client::new();

        let params = [
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("grant_type", "refresh_token"),
        ];

        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct GoogleTokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: Option<u64>,
            token_type: String,
        }

        let token_response: GoogleTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token.or(Some(refresh_token.to_string())),
            expires_in: token_response.expires_in,
            token_type: token_response.token_type,
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

    async fn exchange_code(&self, code: &str) -> Result<OAuthTokens> {
        let client = reqwest::Client::new();

        let params = [
            ("code", code),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];

        let response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct GithubTokenResponse {
            access_token: String,
            token_type: String,
        }

        let token_response: GithubTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: None,
            expires_in: None,
            token_type: token_response.token_type,
        })
    }

    async fn get_user(&self, token: &str) -> Result<OAuthUser> {
        let client = reqwest::Client::new();

        let response = client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "RustForge-OAuth")
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct GithubUserInfo {
            id: u64,
            login: String,
            name: Option<String>,
            email: Option<String>,
            avatar_url: Option<String>,
        }

        let mut user_info: GithubUserInfo = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        // If email is not public, fetch from emails endpoint
        if user_info.email.is_none() {
            #[derive(serde::Deserialize)]
            struct GithubEmail {
                email: String,
                primary: bool,
            }

            let email_response = client
                .get("https://api.github.com/user/emails")
                .header("Authorization", format!("Bearer {}", token))
                .header("User-Agent", "RustForge-OAuth")
                .send()
                .await
                .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

            let emails: Vec<GithubEmail> = email_response
                .json()
                .await
                .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

            user_info.email = emails
                .into_iter()
                .find(|e| e.primary)
                .map(|e| e.email);
        }

        Ok(OAuthUser {
            provider: "github".to_string(),
            provider_id: user_info.id.to_string(),
            email: user_info.email,
            name: user_info.name.or(Some(user_info.login)),
            avatar: user_info.avatar_url,
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

    async fn exchange_code(&self, code: &str) -> Result<OAuthTokens> {
        let client = reqwest::Client::new();

        let url = format!(
            "https://graph.facebook.com/v12.0/oauth/access_token?\
             client_id={}&\
             client_secret={}&\
             code={}&\
             redirect_uri={}",
            self.client_id, self.client_secret, code, self.redirect_uri
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct FacebookTokenResponse {
            access_token: String,
            token_type: String,
            expires_in: Option<u64>,
        }

        let token_response: FacebookTokenResponse = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: None,
            expires_in: token_response.expires_in,
            token_type: token_response.token_type,
        })
    }

    async fn get_user(&self, token: &str) -> Result<OAuthUser> {
        let client = reqwest::Client::new();

        let url = format!(
            "https://graph.facebook.com/me?\
             fields=id,name,email,picture&\
             access_token={}",
            token
        );

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::OAuthError::RequestFailed(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct FacebookPicture {
            data: FacebookPictureData,
        }

        #[derive(serde::Deserialize)]
        struct FacebookPictureData {
            url: String,
        }

        #[derive(serde::Deserialize)]
        struct FacebookUserInfo {
            id: String,
            name: Option<String>,
            email: Option<String>,
            picture: Option<FacebookPicture>,
        }

        let user_info: FacebookUserInfo = response
            .json()
            .await
            .map_err(|e| crate::OAuthError::InvalidResponse(e.to_string()))?;

        Ok(OAuthUser {
            provider: "facebook".to_string(),
            provider_id: user_info.id,
            email: user_info.email,
            name: user_info.name,
            avatar: user_info.picture.map(|p| p.data.url),
        })
    }
}
