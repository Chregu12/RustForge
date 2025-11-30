//! Authorization Code Grant implementation

use crate::{
    auth_code::{verify_code_challenge, AuthCodeRepository, CodeChallengeMethod},
    client::ClientRepository,
    config::PassportConfig,
    errors::{PassportError, PassportResult},
    scope::ScopeRepository,
    token::TokenRepository,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// Authorization request parameters
#[derive(Debug, Deserialize)]
pub struct AuthorizationRequest {
    pub response_type: String,
    pub client_id: i64,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Authorization response
#[derive(Debug, Serialize)]
pub struct AuthorizationResponse {
    pub code: String,
    pub state: Option<String>,
}

/// Token request for authorization code grant
#[derive(Debug, Deserialize)]
pub struct AuthorizationCodeTokenRequest {
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
    pub client_id: i64,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
}

/// Token response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token_type: String,
    pub expires_in: i64,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Authorization Code Grant handler
pub struct AuthorizationCodeGrant<'a> {
    db: &'a DatabaseConnection,
    config: &'a PassportConfig,
}

impl<'a> AuthorizationCodeGrant<'a> {
    pub fn new(db: &'a DatabaseConnection, config: &'a PassportConfig) -> Self {
        Self { db, config }
    }

    /// Handle authorization request
    pub async fn authorize(
        &self,
        user_id: i64,
        request: AuthorizationRequest,
    ) -> PassportResult<AuthorizationResponse> {
        // Validate grant is enabled
        if !self.config.enable_authorization_code_grant {
            return Err(PassportError::UnsupportedGrantType(
                "Authorization code grant is disabled".to_string(),
            ));
        }

        // Validate response type
        if request.response_type != "code" {
            return Err(PassportError::InvalidRequest(
                "Invalid response_type, expected 'code'".to_string(),
            ));
        }

        // Find and validate client
        let client_repo = ClientRepository::new(self.db);
        let client = client_repo.find_active(request.client_id).await?;

        // Validate redirect URI
        if !client.is_redirect_uri_valid(&request.redirect_uri) {
            return Err(PassportError::InvalidRedirectUri);
        }

        // Parse and validate scopes
        let scopes = self.parse_scopes(&request.scope)?;

        // Validate PKCE if enforced
        let (code_challenge, code_challenge_method) = if self.config.enforce_pkce {
            let challenge = request.code_challenge.ok_or_else(|| {
                PassportError::InvalidRequest("PKCE code_challenge is required".to_string())
            })?;

            let method_str = request
                .code_challenge_method
                .as_deref()
                .unwrap_or("plain");

            // Check if plain method is allowed
            if method_str == "plain" && !self.config.allow_plain_pkce {
                return Err(PassportError::InvalidRequest(
                    "Plain PKCE method is not allowed".to_string(),
                ));
            }

            let method = CodeChallengeMethod::from_str(method_str)?;
            (Some(challenge), Some(method))
        } else {
            // PKCE is optional
            if let Some(challenge) = request.code_challenge {
                let method_str = request
                    .code_challenge_method
                    .as_deref()
                    .unwrap_or("plain");
                let method = CodeChallengeMethod::from_str(method_str)?;
                (Some(challenge), Some(method))
            } else {
                (None, None)
            }
        };

        // Create authorization code
        let auth_code_repo = AuthCodeRepository::new(self.db);
        let expires_at = Utc::now() + self.config.auth_code_duration();

        let auth_code = auth_code_repo
            .create(
                user_id,
                client.id,
                scopes,
                request.redirect_uri.clone(),
                expires_at,
                code_challenge,
                code_challenge_method,
            )
            .await?;

        Ok(AuthorizationResponse {
            code: auth_code.id,
            state: request.state,
        })
    }

    /// Exchange authorization code for access token
    pub async fn exchange_token(
        &self,
        request: AuthorizationCodeTokenRequest,
    ) -> PassportResult<TokenResponse> {
        // Validate grant type
        if request.grant_type != "authorization_code" {
            return Err(PassportError::InvalidRequest(
                "Invalid grant_type".to_string(),
            ));
        }

        // Find and validate client
        let client_repo = ClientRepository::new(self.db);
        let client = if let Some(secret) = &request.client_secret {
            // Confidential client - verify credentials
            client_repo
                .verify_credentials(request.client_id, secret)
                .await?
        } else {
            // Public client - just verify it exists and is active
            client_repo.find_active(request.client_id).await?
        };

        // Find and validate authorization code
        let auth_code_repo = AuthCodeRepository::new(self.db);
        let auth_code = auth_code_repo.find_valid(&request.code).await?;

        // Verify client ID matches
        if auth_code.client_id != client.id {
            return Err(PassportError::InvalidGrant(
                "Client ID mismatch".to_string(),
            ));
        }

        // Verify redirect URI matches
        if auth_code.redirect_uri != request.redirect_uri {
            return Err(PassportError::InvalidGrant(
                "Redirect URI mismatch".to_string(),
            ));
        }

        // Verify PKCE if present
        if let Some(code_challenge) = &auth_code.code_challenge {
            let code_verifier = request.code_verifier.ok_or_else(|| {
                PassportError::InvalidRequest("code_verifier is required".to_string())
            })?;

            let method_str = auth_code
                .code_challenge_method
                .as_deref()
                .unwrap_or("plain");
            let method = CodeChallengeMethod::from_str(method_str)?;

            let valid = verify_code_challenge(&code_verifier, code_challenge, &method)?;
            if !valid {
                return Err(PassportError::PkceVerificationFailed(
                    "Code verifier does not match challenge".to_string(),
                ));
            }
        }

        // Revoke the authorization code (single use)
        auth_code_repo.revoke(&request.code).await?;

        // Create access token
        let token_repo = TokenRepository::new(self.db);
        let access_expires_at = Utc::now() + self.config.access_token_duration();

        let access_token = token_repo
            .create_access_token(
                Some(auth_code.user_id),
                client.id,
                auth_code.get_scopes(),
                access_expires_at,
                None,
            )
            .await?;

        // Create refresh token if enabled
        let refresh_token = if self.config.enable_refresh_token_grant {
            let refresh_expires_at = Utc::now() + self.config.refresh_token_duration();
            let refresh = token_repo
                .create_refresh_token(access_token.id.clone(), refresh_expires_at)
                .await?;
            Some(refresh.id)
        } else {
            None
        };

        let scopes = access_token.get_scopes().join(" ");
        let token_id = access_token.id;

        Ok(TokenResponse {
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_lifetime,
            access_token: token_id,
            refresh_token,
            scope: Some(scopes),
        })
    }

    /// Parse scope string into Vec<String>
    fn parse_scopes(&self, scope_str: &Option<String>) -> PassportResult<Vec<String>> {
        let scopes = if let Some(s) = scope_str {
            s.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            self.config.default_scopes.clone()
        };

        // Validate scopes if scope repository is configured
        if ScopeRepository::count() > 0 {
            if let Err(invalid) = ScopeRepository::validate(&scopes) {
                return Err(PassportError::InvalidScope(format!(
                    "Invalid scopes: {}",
                    invalid.join(", ")
                )));
            }
        }

        Ok(scopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_compiles() {
        // Compilation test
        assert!(true);
    }
}
