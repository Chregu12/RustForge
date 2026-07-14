//! Client Credentials Grant implementation

use crate::{
    client::ClientRepository,
    config::PassportConfig,
    errors::{PassportError, PassportResult},
    grants::authorization_code::TokenResponse,
    scope::ScopeRepository,
    token::TokenRepository,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

/// Client credentials grant token request
#[derive(Debug, Deserialize)]
pub struct ClientCredentialsRequest {
    pub grant_type: String,
    pub client_id: i64,
    pub client_secret: String,
    pub scope: Option<String>,
}

/// Client Credentials Grant handler
pub struct ClientCredentialsGrant<'a> {
    db: &'a DatabaseConnection,
    config: &'a PassportConfig,
}

impl<'a> ClientCredentialsGrant<'a> {
    pub fn new(db: &'a DatabaseConnection, config: &'a PassportConfig) -> Self {
        Self { db, config }
    }

    /// Issue token using client credentials
    pub async fn issue_token(
        &self,
        request: ClientCredentialsRequest,
    ) -> PassportResult<TokenResponse> {
        // Validate grant is enabled
        if !self.config.enable_client_credentials_grant {
            return Err(PassportError::UnsupportedGrantType(
                "Client credentials grant is disabled".to_string(),
            ));
        }

        // Validate grant type
        if request.grant_type != "client_credentials" {
            return Err(PassportError::InvalidRequest(
                "Invalid grant_type".to_string(),
            ));
        }

        // Verify client credentials
        let client_repo = ClientRepository::new(self.db);
        let client = client_repo
            .verify_credentials(request.client_id, &request.client_secret)
            .await?;

        // Client credentials grant should use confidential clients
        if !client.is_confidential() {
            return Err(PassportError::UnauthorizedClient(
                "Public clients cannot use client credentials grant".to_string(),
            ));
        }

        // Parse and validate scopes
        let scopes = self.parse_scopes(&request.scope)?;

        // Create access token (no user_id for client credentials)
        let token_repo = TokenRepository::new(self.db);
        let access_expires_at = Utc::now() + self.config.access_token_duration();

        let access_token = token_repo
            .create_access_token(None, client.id, scopes, access_expires_at, None)
            .await?;

        // Client credentials grant typically does not issue refresh tokens
        let scopes = access_token.get_scopes().join(" ");
        let token_id = access_token.id;

        Ok(TokenResponse {
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_lifetime,
            access_token: token_id,
            refresh_token: None,
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

// Grant logic requires a live token store; covered by integration tests.
