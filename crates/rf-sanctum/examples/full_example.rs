//! Complete example of Laravel Sanctum authentication
//!
//! This example demonstrates:
//! - Creating API tokens for users
//! - Token-based authentication
//! - Ability/scope checking
//! - Token revocation
//! - SPA CSRF protection

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use rf_sanctum::{
    spa::sanctum_csrf_cookie, LoadFromToken, NewToken, PersonalAccessToken, SanctumAuth,
    SanctumError, Tokenable, TokenRepository,
};
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Example User model
#[derive(Clone, Debug, Serialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

// Implement Tokenable trait
#[async_trait::async_trait]
impl Tokenable for User {
    fn tokenable_type() -> &'static str {
        "User"
    }

    fn tokenable_id(&self) -> i64 {
        self.id
    }
}

// Implement LoadFromToken trait
#[async_trait::async_trait]
impl LoadFromToken for User {
    async fn load_from_token(
        tokenable_id: i64,
        _db: &DatabaseConnection,
    ) -> Result<Self, SanctumError> {
        // In a real app, load from database
        Ok(User {
            id: tokenable_id,
            name: format!("User {}", tokenable_id),
            email: format!("user{}@example.com", tokenable_id),
        })
    }
}

// Application state
#[derive(Clone)]
struct AppState {
    db: Arc<DatabaseConnection>,
}

// Request/Response types
#[derive(Deserialize)]
struct CreateTokenRequest {
    name: String,
    abilities: Vec<String>,
}

#[derive(Serialize)]
struct CreateTokenResponse {
    token: String,
    abilities: Vec<String>,
}

#[derive(Serialize)]
struct TokenInfo {
    id: i64,
    name: String,
    abilities: Vec<String>,
    created_at: String,
}

// Handlers
async fn create_token(
    State(state): State<AppState>,
    SanctumAuth(user, _): SanctumAuth<User>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, SanctumError> {
    // Create token with specified abilities
    let new_token = user
        .create_token(
            &req.name,
            req.abilities.iter().map(|s| s.as_str()).collect(),
            None, // No expiration
            &state.db,
        )
        .await?;

    Ok(Json(CreateTokenResponse {
        token: new_token.access_token,
        abilities: new_token.token.abilities,
    }))
}

async fn list_tokens(
    State(state): State<AppState>,
    SanctumAuth(user, _): SanctumAuth<User>,
) -> Result<Json<Vec<TokenInfo>>, SanctumError> {
    let tokens = user.tokens(&state.db).await?;

    let token_infos = tokens
        .into_iter()
        .map(|t| TokenInfo {
            id: t.id,
            name: t.name,
            abilities: t.abilities,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(token_infos))
}

async fn revoke_token(
    State(state): State<AppState>,
    SanctumAuth(user, _): SanctumAuth<User>,
    axum::extract::Path(token_id): axum::extract::Path<i64>,
) -> Result<StatusCode, SanctumError> {
    user.revoke_token(token_id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current_user(
    SanctumAuth(user, token): SanctumAuth<User>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "user": user,
        "token_abilities": token.abilities,
    }))
}

// Protected route requiring specific abilities
async fn admin_only(
    SanctumAuth(user, token): SanctumAuth<User>,
) -> Result<String, SanctumError> {
    if !token.can("admin") {
        return Err(SanctumError::InsufficientPermissions(
            "admin ability required".to_string(),
        ));
    }

    Ok(format!("Welcome, admin {}!", user.name))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup database
    let db = Database::connect("sqlite::memory:").await?;
    let state = AppState { db: Arc::new(db) };

    // Build router
    let app = Router::new()
        // SPA CSRF protection
        .route("/sanctum/csrf-cookie", get(sanctum_csrf_cookie))
        // Token management
        .route("/api/tokens", post(create_token))
        .route("/api/tokens", get(list_tokens))
        .route("/api/tokens/:id", delete(revoke_token))
        // Protected routes
        .route("/api/user", get(current_user))
        .route("/api/admin", get(admin_only))
        .with_state(state);

    println!("Sanctum API example running on http://localhost:3000");
    println!("\nExample usage:");
    println!("1. Create a token: POST /api/tokens");
    println!("   {{\"name\": \"mobile-app\", \"abilities\": [\"read:posts\", \"write:posts\"]}}");
    println!("\n2. Use token in requests:");
    println!("   Authorization: Bearer <token>");
    println!("\n3. List tokens: GET /api/tokens");
    println!("4. Revoke token: DELETE /api/tokens/:id");

    // Run server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
