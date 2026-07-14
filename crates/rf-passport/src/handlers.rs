//! OAuth2 route handlers for Axum

use crate::{
    client::ClientRepository,
    config::PassportConfig,
    errors::PassportError,
    grants::{
        AuthorizationCodeGrant, AuthorizationCodeTokenRequest, ClientCredentialsGrant,
        ClientCredentialsRequest, PasswordGrant, PasswordGrantRequest, PasswordVerifier,
        RefreshTokenGrant, RefreshTokenRequest, TokenResponse,
    },
    middleware::PassportAuth,
    token::TokenRepository,
};
use axum::{extract::State, response::IntoResponse, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct PassportState {
    pub db: Arc<DatabaseConnection>,
    pub config: Arc<PassportConfig>,
}

/// Token endpoint request (supports multiple grant types)
#[derive(Debug, Deserialize)]
#[serde(tag = "grant_type")]
pub enum TokenRequest {
    #[serde(rename = "authorization_code")]
    AuthorizationCode(AuthorizationCodeTokenRequest),
    #[serde(rename = "password")]
    Password(PasswordGrantRequest),
    #[serde(rename = "client_credentials")]
    ClientCredentials(ClientCredentialsRequest),
    #[serde(rename = "refresh_token")]
    RefreshToken(RefreshTokenRequest),
}

/// Token endpoint - POST /oauth/token
///
/// Handles multiple grant types:
/// - authorization_code
/// - password
/// - client_credentials
/// - refresh_token
pub async fn token_endpoint<V: PasswordVerifier>(
    State(state): State<PassportState>,
    verifier: Option<V>,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<TokenResponse>, PassportError> {
    // Determine grant type
    let grant_type = request
        .get("grant_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| PassportError::InvalidRequest("Missing grant_type".to_string()))?;

    let response = match grant_type {
        "authorization_code" => {
            let req: AuthorizationCodeTokenRequest = serde_json::from_value(request)
                .map_err(|e| PassportError::InvalidRequest(e.to_string()))?;

            let grant = AuthorizationCodeGrant::new(&state.db, &state.config);
            grant.exchange_token(req).await?
        }

        "password" => {
            let req: PasswordGrantRequest = serde_json::from_value(request)
                .map_err(|e| PassportError::InvalidRequest(e.to_string()))?;

            let verifier = verifier.ok_or_else(|| {
                PassportError::InternalError("Password verifier not configured".to_string())
            })?;

            let grant = PasswordGrant::new(&state.db, &state.config);
            grant.issue_token(req, &verifier).await?
        }

        "client_credentials" => {
            let req: ClientCredentialsRequest = serde_json::from_value(request)
                .map_err(|e| PassportError::InvalidRequest(e.to_string()))?;

            let grant = ClientCredentialsGrant::new(&state.db, &state.config);
            grant.issue_token(req).await?
        }

        "refresh_token" => {
            let req: RefreshTokenRequest = serde_json::from_value(request)
                .map_err(|e| PassportError::InvalidRequest(e.to_string()))?;

            let grant = RefreshTokenGrant::new(&state.db, &state.config);
            grant.refresh(req).await?
        }

        _ => {
            return Err(PassportError::UnsupportedGrantType(format!(
                "Grant type '{}' is not supported",
                grant_type
            )))
        }
    };

    Ok(Json(response))
}

/// Revoke token endpoint - DELETE /oauth/tokens/{token_id}
pub async fn revoke_token(
    State(state): State<PassportState>,
    PassportAuth(user_id, requesting_token): PassportAuth,
    token_id: String,
) -> Result<impl IntoResponse, PassportError> {
    // Only allow users to revoke their own tokens
    let token_repo = TokenRepository::new(&state.db);
    let token = token_repo
        .find_access_token(&token_id)
        .await?
        .ok_or(PassportError::InvalidToken)?;

    // For client-credentials tokens (user_id is None), verify client_id matches
    // For user tokens, verify user_id matches
    if token.user_id.is_none() {
        // Client-credentials token: must be same client
        if token.client_id != requesting_token.client_id {
            return Err(PassportError::AccessDenied(
                "Cannot revoke another client's token".to_string(),
            ));
        }
    } else if token.user_id != user_id {
        // User token: must be same user
        return Err(PassportError::AccessDenied(
            "Cannot revoke another user's token".to_string(),
        ));
    }

    token_repo.revoke_access_token(&token_id).await?;

    Ok(Json(serde_json::json!({
        "message": "Token revoked successfully"
    })))
}

/// List user's tokens - GET /oauth/tokens
pub async fn list_tokens(
    State(state): State<PassportState>,
    PassportAuth(user_id, _): PassportAuth,
) -> Result<Json<Vec<TokenInfo>>, PassportError> {
    let user_id = user_id.ok_or(PassportError::AccessDenied(
        "Only user tokens can be listed".to_string(),
    ))?;

    let token_repo = TokenRepository::new(&state.db);
    let tokens = token_repo.find_tokens_by_user(user_id).await?;

    let token_infos: Vec<TokenInfo> = tokens
        .into_iter()
        .map(|t| {
            let scopes = t.get_scopes();
            TokenInfo {
                id: t.id,
                name: t.name,
                scopes,
                revoked: t.revoked,
                expires_at: t.expires_at,
                created_at: t.created_at,
            }
        })
        .collect();

    Ok(Json(token_infos))
}

/// Token information response
#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub id: String,
    pub name: Option<String>,
    pub scopes: Vec<String>,
    pub revoked: bool,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// List user's clients - GET /oauth/clients
pub async fn list_clients(
    State(state): State<PassportState>,
    PassportAuth(user_id, _): PassportAuth,
) -> Result<Json<Vec<ClientInfo>>, PassportError> {
    let user_id = user_id.ok_or(PassportError::AccessDenied(
        "Only user clients can be listed".to_string(),
    ))?;

    let client_repo = ClientRepository::new(&state.db);
    let clients = client_repo.find_by_user(user_id).await?;

    let client_infos: Vec<ClientInfo> = clients
        .into_iter()
        .map(|c| {
            let uris = c.redirect_uris();
            ClientInfo {
                id: c.id,
                name: c.name,
                redirect_uris: uris,
                personal_access_client: c.personal_access_client,
                password_client: c.password_client,
                revoked: c.revoked,
                created_at: c.created_at,
            }
        })
        .collect();

    Ok(Json(client_infos))
}

/// Client information response
#[derive(Debug, Serialize)]
pub struct ClientInfo {
    pub id: i64,
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub personal_access_client: bool,
    pub password_client: bool,
    pub revoked: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create client - POST /oauth/clients
#[derive(Debug, Deserialize)]
pub struct CreateClientRequest {
    pub name: String,
    pub redirect_uris: Vec<String>,
    pub confidential: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CreateClientResponse {
    pub client_id: i64,
    pub client_secret: Option<String>,
    pub name: String,
    pub redirect_uris: Vec<String>,
}

pub async fn create_client(
    State(state): State<PassportState>,
    PassportAuth(user_id, _): PassportAuth,
    Json(request): Json<CreateClientRequest>,
) -> Result<Json<CreateClientResponse>, PassportError> {
    let user_id = user_id.ok_or(PassportError::AccessDenied(
        "Only users can create clients".to_string(),
    ))?;

    let client_repo = ClientRepository::new(&state.db);
    let (client, secret) = client_repo
        .create(
            Some(user_id),
            &request.name,
            request.redirect_uris.clone(),
            false,
            false,
            request.confidential.unwrap_or(true),
        )
        .await?;

    Ok(Json(CreateClientResponse {
        client_id: client.id,
        client_secret: secret,
        name: client.name,
        redirect_uris: request.redirect_uris,
    }))
}

/// Delete client - DELETE /oauth/clients/{client_id}
pub async fn delete_client(
    State(state): State<PassportState>,
    PassportAuth(user_id, _): PassportAuth,
    client_id: i64,
) -> Result<impl IntoResponse, PassportError> {
    let user_id = user_id.ok_or(PassportError::AccessDenied(
        "Only users can delete clients".to_string(),
    ))?;

    let client_repo = ClientRepository::new(&state.db);
    let client = client_repo
        .find_by_id(client_id)
        .await?
        .ok_or(PassportError::ClientNotFound)?;

    // Verify ownership
    if client.user_id != Some(user_id) {
        return Err(PassportError::AccessDenied(
            "Cannot delete another user's client".to_string(),
        ));
    }

    client_repo.delete(client_id).await?;

    Ok(Json(serde_json::json!({
        "message": "Client deleted successfully"
    })))
}

// Handlers wire together the grant pipeline and require an HTTP context;
// covered by integration tests.
