//! Implicit Grant implementation (deprecated but included for compatibility)

use crate::{
    client::ClientRepository,
    config::PassportConfig,
    errors::{PassportError, PassportResult},
    scope::ScopeRepository,
    token::TokenRepository,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// Implicit grant authorization request
#[derive(Debug, Deserialize)]
pub struct ImplicitGrantRequest {
    pub response_type: String,
    pub client_id: i64,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

/// Implicit grant authorization response
#[derive(Debug, Serialize)]
pub struct ImplicitGrantResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: Option<String>,
    pub state: Option<String>,
}

/// Implicit Grant handler
///
/// WARNING: This grant type is deprecated and should not be used in new applications.
/// Use Authorization Code Grant with PKCE instead.
pub struct ImplicitGrant<'a> {
    db: &'a DatabaseConnection,
    config: &'a PassportConfig,
}

impl<'a> ImplicitGrant<'a> {
    pub fn new(db: &'a DatabaseConnection, config: &'a PassportConfig) -> Self {
        Self { db, config }
    }

    /// Issue access token directly (implicit flow)
    pub async fn authorize(
        &self,
        user_id: i64,
        request: ImplicitGrantRequest,
    ) -> PassportResult<ImplicitGrantResponse> {
        // Validate grant is enabled
        if !self.config.enable_implicit_grant {
            return Err(PassportError::UnsupportedGrantType(
                "Implicit grant is disabled".to_string(),
            ));
        }

        // Validate response type (token or id_token for OpenID Connect)
        if request.response_type != "token" {
            return Err(PassportError::InvalidRequest(
                "Invalid response_type, expected 'token'".to_string(),
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

        // Create access token
        let token_repo = TokenRepository::new(self.db);
        let access_expires_at = Utc::now() + self.config.access_token_duration();

        let access_token = token_repo
            .create_access_token(Some(user_id), client.id, scopes, access_expires_at, None)
            .await?;

        // Implicit grant does NOT issue refresh tokens (security limitation)
        let scopes = access_token.get_scopes().join(" ");
        let token_id = access_token.id;

        Ok(ImplicitGrantResponse {
            access_token: token_id,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_lifetime,
            scope: Some(scopes),
            state: request.state,
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
