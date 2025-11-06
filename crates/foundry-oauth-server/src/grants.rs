//! OAuth2 Grant Types
//!
//! Implements all OAuth2 flows:
//! - Authorization Code Grant (with PKCE)
//! - Client Credentials Grant
//! - Password Grant (Resource Owner Password Credentials)
//! - Refresh Token Grant

use crate::errors::{OAuth2Error, OAuth2Result};
use crate::models::{Client, AccessToken, RefreshToken, AuthorizationCode};
use crate::tokens::TokenGenerator;
use chrono::{Utc, Duration};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// OAuth2 Grant Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrantType {
    AuthorizationCode,
    ClientCredentials,
    Password,
    RefreshToken,
}

impl std::fmt::Display for GrantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrantType::AuthorizationCode => write!(f, "authorization_code"),
            GrantType::ClientCredentials => write!(f, "client_credentials"),
            GrantType::Password => write!(f, "password"),
            GrantType::RefreshToken => write!(f, "refresh_token"),
        }
    }
}

/// Token Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Parameters for creating an authorization code
#[derive(Debug, Clone)]
pub struct AuthCodeParams {
    pub user_id: Uuid,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub lifetime: i64,
}

/// Authorization Code Grant
pub struct AuthorizationCodeGrant {
    token_generator: TokenGenerator,
    access_token_lifetime: i64,
    refresh_token_lifetime: i64,
}

impl AuthorizationCodeGrant {
    pub fn new(
        token_generator: TokenGenerator,
        access_token_lifetime: i64,
        refresh_token_lifetime: i64,
    ) -> Self {
        Self {
            token_generator,
            access_token_lifetime,
            refresh_token_lifetime,
        }
    }

    /// Create authorization code
    pub fn create_authorization_code(
        &self,
        client: &Client,
        params: AuthCodeParams,
    ) -> OAuth2Result<AuthorizationCode> {
        let code = self.generate_code();
        let expires_at = Utc::now() + Duration::seconds(params.lifetime);

        Ok(AuthorizationCode {
            id: Uuid::new_v4(),
            client_id: client.id,
            user_id: params.user_id,
            code,
            redirect_uri: params.redirect_uri,
            scopes: params.scopes,
            code_challenge: params.code_challenge,
            code_challenge_method: params.code_challenge_method,
            revoked: false,
            expires_at,
            created_at: Utc::now(),
        })
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        client: &Client,
        code: &AuthorizationCode,
        code_verifier: Option<String>,
    ) -> OAuth2Result<TokenResponse> {
        // Validate code is not expired
        if code.expires_at < Utc::now() {
            return Err(OAuth2Error::InvalidGrant("Authorization code expired".to_string()));
        }

        // Validate code is not revoked
        if code.revoked {
            return Err(OAuth2Error::InvalidGrant("Authorization code revoked".to_string()));
        }

        // Validate PKCE if required
        if let Some(challenge) = &code.code_challenge {
            let verifier = code_verifier.ok_or_else(|| {
                OAuth2Error::InvalidRequest("code_verifier required".to_string())
            })?;

            if !self.verify_pkce(challenge, &verifier, code.code_challenge_method.as_deref())? {
                return Err(OAuth2Error::InvalidGrant("Invalid code_verifier".to_string()));
            }
        }

        // Generate access token
        let access_token = self.token_generator.generate_access_token(
            client.id,
            Some(code.user_id),
            code.scopes.clone(),
            self.access_token_lifetime,
        )?;

        // Generate refresh token
        let refresh_token = self.token_generator.generate_refresh_token(
            access_token.id,
            self.refresh_token_lifetime,
        )?;

        Ok(TokenResponse {
            access_token: access_token.token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_lifetime,
            refresh_token: Some(refresh_token.token),
            scope: Some(code.scopes.join(" ")),
        })
    }

    fn generate_code(&self) -> String {
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(32)
            .collect();
        STANDARD.encode(&random_bytes)
    }

    fn verify_pkce(
        &self,
        challenge: &str,
        verifier: &str,
        method: Option<&str>,
    ) -> OAuth2Result<bool> {
        use subtle::ConstantTimeEq;

        match method {
            Some("S256") => {
                use sha2::{Sha256, Digest};
                use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
                let mut hasher = Sha256::new();
                hasher.update(verifier.as_bytes());
                let hash = hasher.finalize();
                let computed_challenge = URL_SAFE_NO_PAD.encode(hash);

                // Use constant-time comparison to prevent timing attacks
                Ok(computed_challenge.as_bytes().ct_eq(challenge.as_bytes()).into())
            }
            Some("plain") | None => {
                // Use constant-time comparison to prevent timing attacks
                Ok(verifier.as_bytes().ct_eq(challenge.as_bytes()).into())
            }
            _ => Err(OAuth2Error::InvalidRequest(
                "Unsupported code_challenge_method".to_string(),
            )),
        }
    }
}

/// Client Credentials Grant
pub struct ClientCredentialsGrant {
    token_generator: TokenGenerator,
    access_token_lifetime: i64,
}

impl ClientCredentialsGrant {
    pub fn new(token_generator: TokenGenerator, access_token_lifetime: i64) -> Self {
        Self {
            token_generator,
            access_token_lifetime,
        }
    }

    pub async fn issue_token(
        &self,
        client: &Client,
        scopes: Vec<String>,
    ) -> OAuth2Result<TokenResponse> {
        // Validate client is confidential
        if !client.is_confidential() {
            return Err(OAuth2Error::UnauthorizedClient(
                "Client credentials grant requires confidential client".to_string(),
            ));
        }

        // Generate access token (no user context)
        let access_token = self.token_generator.generate_access_token(
            client.id,
            None, // No user_id for client credentials
            scopes.clone(),
            self.access_token_lifetime,
        )?;

        Ok(TokenResponse {
            access_token: access_token.token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_lifetime,
            refresh_token: None, // No refresh token for client credentials
            scope: Some(scopes.join(" ")),
        })
    }
}

/// Password Grant (Resource Owner Password Credentials)
pub struct PasswordGrant {
    token_generator: TokenGenerator,
    access_token_lifetime: i64,
    refresh_token_lifetime: i64,
}

impl PasswordGrant {
    pub fn new(
        token_generator: TokenGenerator,
        access_token_lifetime: i64,
        refresh_token_lifetime: i64,
    ) -> Self {
        Self {
            token_generator,
            access_token_lifetime,
            refresh_token_lifetime,
        }
    }

    pub async fn issue_token(
        &self,
        client: &Client,
        user_id: Uuid,
        scopes: Vec<String>,
    ) -> OAuth2Result<TokenResponse> {
        // Generate access token
        let access_token = self.token_generator.generate_access_token(
            client.id,
            Some(user_id),
            scopes.clone(),
            self.access_token_lifetime,
        )?;

        // Generate refresh token
        let refresh_token = self.token_generator.generate_refresh_token(
            access_token.id,
            self.refresh_token_lifetime,
        )?;

        Ok(TokenResponse {
            access_token: access_token.token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_lifetime,
            refresh_token: Some(refresh_token.token),
            scope: Some(scopes.join(" ")),
        })
    }
}

/// Refresh Token Grant
pub struct RefreshTokenGrant {
    token_generator: TokenGenerator,
    access_token_lifetime: i64,
    refresh_token_lifetime: i64,
}

impl RefreshTokenGrant {
    pub fn new(
        token_generator: TokenGenerator,
        access_token_lifetime: i64,
        refresh_token_lifetime: i64,
    ) -> Self {
        Self {
            token_generator,
            access_token_lifetime,
            refresh_token_lifetime,
        }
    }

    pub async fn refresh(
        &self,
        client: &Client,
        old_refresh_token: &RefreshToken,
        old_access_token: &AccessToken,
    ) -> OAuth2Result<TokenResponse> {
        // Validate refresh token not expired
        if old_refresh_token.expires_at < Utc::now() {
            return Err(OAuth2Error::InvalidGrant("Refresh token expired".to_string()));
        }

        // Validate not revoked
        if old_refresh_token.revoked {
            return Err(OAuth2Error::InvalidGrant("Refresh token revoked".to_string()));
        }

        // Generate new access token (preserve scopes and user_id)
        let access_token = self.token_generator.generate_access_token(
            client.id,
            old_access_token.user_id,
            old_access_token.scopes.clone(),
            self.access_token_lifetime,
        )?;

        // Generate new refresh token
        let refresh_token = self.token_generator.generate_refresh_token(
            access_token.id,
            self.refresh_token_lifetime,
        )?;

        Ok(TokenResponse {
            access_token: access_token.token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_lifetime,
            refresh_token: Some(refresh_token.token),
            scope: Some(old_access_token.scopes.join(" ")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OAuth2Config;

    #[test]
    fn test_pkce_s256() {
        let config = OAuth2Config::default();
        let token_gen = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());
        let grant = AuthorizationCodeGrant::new(token_gen, 3600, 2592000);

        let verifier = "test_verifier_123456789012345678901234567890";

        // Compute S256 challenge
        use sha2::{Sha256, Digest};
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(&hash);

        assert!(grant.verify_pkce(&challenge, verifier, Some("S256")).unwrap());
    }

    #[test]
    fn test_pkce_plain() {
        let config = OAuth2Config::default();
        let token_gen = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());
        let grant = AuthorizationCodeGrant::new(token_gen, 3600, 2592000);

        let verifier = "plain_challenge";
        assert!(grant.verify_pkce(verifier, verifier, Some("plain")).unwrap());
    }
}
