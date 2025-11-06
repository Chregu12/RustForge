//! OAuth2 HTTP Routes
//!
//! Axum HTTP endpoints for OAuth2 server

use crate::clients::ClientRepository;
use crate::errors::OAuth2Error;
use crate::server::OAuth2Server;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// OAuth2 Error Response
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    error_description: String,
}

impl IntoResponse for OAuth2Error {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code()).unwrap();
        let body = Json(ErrorResponse {
            error: self.error_code().to_string(),
            error_description: self.to_string(),
        });

        (status, body).into_response()
    }
}

/// Authorization Request (Authorization Code Flow)
#[derive(Debug, Deserialize)]
pub struct AuthorizationRequest {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    scope: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

/// Authorization Response
#[derive(Debug, Serialize)]
pub struct AuthorizationResponse {
    code: String,
    state: Option<String>,
}

/// Token Request
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    grant_type: String,

    // Authorization Code Grant
    #[allow(dead_code)]
    code: Option<String>,
    #[allow(dead_code)]
    redirect_uri: Option<String>,
    #[allow(dead_code)]
    code_verifier: Option<String>,

    // Client Credentials Grant
    scope: Option<String>,

    // Password Grant
    username: Option<String>,
    password: Option<String>,

    // Refresh Token Grant
    refresh_token: Option<String>,

    // Client Authentication
    client_id: Option<String>,
    client_secret: Option<String>,
}

/// Token Introspection Request
#[derive(Debug, Deserialize)]
pub struct IntrospectionRequest {
    token: String,
    #[allow(dead_code)]
    token_type_hint: Option<String>,
}

/// Token Revocation Request
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RevocationRequest {
    token: String,
    token_type_hint: Option<String>,
}

/// Create OAuth2 router
pub fn oauth2_routes<R: ClientRepository + 'static>() -> Router<Arc<OAuth2Server<R>>> {
    Router::new()
        .route("/authorize", get(authorize_endpoint::<R>))
        .route("/token", post(token_endpoint::<R>))
        .route("/introspect", post(introspect_endpoint::<R>))
        .route("/revoke", post(revoke_endpoint::<R>))
        .route("/.well-known/oauth-authorization-server", get(metadata_endpoint::<R>))
}

/// Authorization endpoint (GET /oauth/authorize)
async fn authorize_endpoint<R: ClientRepository>(
    State(server): State<Arc<OAuth2Server<R>>>,
    Query(params): Query<AuthorizationRequest>,
) -> Result<Json<AuthorizationResponse>, OAuth2Error> {
    // Validate response_type
    if params.response_type != "code" {
        return Err(OAuth2Error::UnsupportedGrantType(
            "Only 'code' response_type supported".to_string(),
        ));
    }

    // Parse client_id
    let client_id = Uuid::parse_str(&params.client_id)
        .map_err(|_| OAuth2Error::InvalidClient("Invalid client_id format".to_string()))?;

    // Validate client
    let client = server.validate_client(client_id, None).await?;

    // Parse scopes
    let scopes = params
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_else(|| vec!["*".to_string()]);

    // For demonstration: simulate authenticated user
    // In production, this would come from session/authentication
    let user_id = Uuid::new_v4();

    // Create authorization code
    let auth_code = server.create_authorization_code(
        &client,
        user_id,
        params.redirect_uri,
        scopes,
        params.code_challenge,
        params.code_challenge_method,
    )?;

    Ok(Json(AuthorizationResponse {
        code: auth_code.code,
        state: params.state,
    }))
}

/// Token endpoint (POST /oauth/token)
async fn token_endpoint<R: ClientRepository>(
    State(_server): State<Arc<OAuth2Server<R>>>,
    Json(params): Json<TokenRequest>,
) -> Result<Json<crate::grants::TokenResponse>, OAuth2Error> {
    match params.grant_type.as_str() {
        "authorization_code" => {
            // This is a simplified example
            // In production, you would:
            // 1. Validate client credentials
            // 2. Look up the authorization code from storage
            // 3. Exchange it for tokens
            Err(OAuth2Error::ServerError(
                "Not implemented in example".to_string(),
            ))
        }
        "client_credentials" => {
            // Validate client credentials
            let _client_id = params
                .client_id
                .ok_or_else(|| OAuth2Error::InvalidRequest("client_id required".to_string()))?;
            let _client_secret = params
                .client_secret
                .ok_or_else(|| OAuth2Error::InvalidRequest("client_secret required".to_string()))?;

            // Parse scopes
            let _scopes = params
                .scope
                .map(|s| s.split_whitespace().map(String::from).collect())
                .unwrap_or_else(|| vec!["*".to_string()]);

            // Example implementation - needs storage integration
            Err(OAuth2Error::ServerError(
                "Not implemented in example".to_string(),
            ))
        }
        "password" => {
            // Validate username and password
            let _username = params
                .username
                .ok_or_else(|| OAuth2Error::InvalidRequest("username required".to_string()))?;
            let _password = params
                .password
                .ok_or_else(|| OAuth2Error::InvalidRequest("password required".to_string()))?;

            // Example implementation - needs user authentication
            Err(OAuth2Error::ServerError(
                "Not implemented in example".to_string(),
            ))
        }
        "refresh_token" => {
            // Validate refresh token
            let _refresh_token = params
                .refresh_token
                .ok_or_else(|| OAuth2Error::InvalidRequest("refresh_token required".to_string()))?;

            // Example implementation - needs token storage
            Err(OAuth2Error::ServerError(
                "Not implemented in example".to_string(),
            ))
        }
        _ => Err(OAuth2Error::UnsupportedGrantType(format!(
            "Grant type '{}' not supported",
            params.grant_type
        ))),
    }
}

/// Token introspection endpoint (POST /oauth/introspect)
async fn introspect_endpoint<R: ClientRepository>(
    State(server): State<Arc<OAuth2Server<R>>>,
    Json(params): Json<IntrospectionRequest>,
) -> Result<Json<crate::tokens::TokenIntrospection>, OAuth2Error> {
    let introspection = server.introspect_token(&params.token);
    Ok(Json(introspection))
}

/// Token revocation endpoint (POST /oauth/revoke)
async fn revoke_endpoint<R: ClientRepository>(
    State(_server): State<Arc<OAuth2Server<R>>>,
    Json(_params): Json<RevocationRequest>,
) -> Result<StatusCode, OAuth2Error> {
    // Token revocation implementation would go here
    // This requires token storage integration
    Ok(StatusCode::OK)
}

/// Authorization Server Metadata (RFC 8414)
#[derive(Debug, Serialize)]
pub struct AuthorizationServerMetadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    introspection_endpoint: String,
    revocation_endpoint: String,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    code_challenge_methods_supported: Vec<String>,
    scopes_supported: Vec<String>,
}

/// Authorization Server Metadata endpoint
async fn metadata_endpoint<R: ClientRepository>(
    State(server): State<Arc<OAuth2Server<R>>>,
) -> Json<AuthorizationServerMetadata> {
    let scopes: Vec<String> = server
        .scope_manager()
        .all()
        .iter()
        .map(|s| s.id.clone())
        .collect();

    Json(AuthorizationServerMetadata {
        issuer: "foundry-oauth-server".to_string(),
        authorization_endpoint: "/oauth/authorize".to_string(),
        token_endpoint: "/oauth/token".to_string(),
        introspection_endpoint: "/oauth/introspect".to_string(),
        revocation_endpoint: "/oauth/revoke".to_string(),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "client_credentials".to_string(),
            "password".to_string(),
            "refresh_token".to_string(),
        ],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_post".to_string(),
            "client_secret_basic".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string(), "plain".to_string()],
        scopes_supported: scopes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::InMemoryClientRepository;
    use crate::OAuth2Config;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_metadata_endpoint() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();
        let server = Arc::new(OAuth2Server::new(config, repo));

        let app = oauth2_routes().with_state(server);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/oauth-authorization-server")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
