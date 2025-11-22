// Authentication Demo - Complete Auth Example with rf-auth
//
// Demonstrates:
// - Password hashing with bcrypt
// - JWT token generation and validation
// - User registration and login flows
// - Protected routes with authentication
// - Role-based access control
// - Token refresh mechanism

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use rf_auth::{middleware::require_role, Claims, JwtManager, PasswordHasher};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// Simple in-memory user store (in production, use a database)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i32,
    email: String,
    name: String,
    #[serde(skip_serializing)]
    password_hash: String,
    roles: Vec<String>,
}

// Request/Response DTOs
#[derive(Debug, Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
    refresh_token: String,
    expires_in: i64,
    user: UserResponse,
}

#[derive(Debug, Serialize)]
struct UserResponse {
    id: i32,
    email: String,
    name: String,
    roles: Vec<String>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            roles: user.roles,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct RefreshResponse {
    token: String,
    expires_in: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Authentication Demo - rf-auth with Axum");
    info!("==========================================\n");

    // Setup authentication components
    info!("🔧 Setting up authentication...");
    let password_hasher = Arc::new(PasswordHasher::bcrypt(12)?);
    let jwt_manager = Arc::new(JwtManager::new("demo-secret-key-min-32-characters-long")?);
    info!("✅ Authentication configured\n");

    // Build application router
    info!("🌐 Building application routes...");

    let app = Router::new()
        // Public routes
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/register", post(register_handler))
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        // Protected routes (manually check auth in handlers)
        .route("/profile", get(profile_handler))
        .route("/admin", get(admin_handler))
        // Shared state for all routes
        .layer(Extension(password_hasher))
        .layer(Extension(jwt_manager));

    info!("✅ Routes configured\n");

    // Demo server info
    info!("📡 Server Configuration:");
    info!("   Address: http://localhost:3000");
    info!("   Endpoints:");
    info!("     GET  /            - Root endpoint");
    info!("     GET  /health      - Health check");
    info!("     POST /register    - User registration");
    info!("     POST /login       - User login");
    info!("     POST /refresh     - Refresh access token");
    info!("     GET  /profile     - User profile (protected)");
    info!("     GET  /admin       - Admin dashboard (protected, admin role required)");
    info!("\n📝 Example Requests:");
    info!("   Register: curl -X POST http://localhost:3000/register -H 'Content-Type: application/json' -d '{{\"email\":\"user@example.com\",\"password\":\"SecurePass123\",\"name\":\"John Doe\"}}'");
    info!("   Login: curl -X POST http://localhost:3000/login -H 'Content-Type: application/json' -d '{{\"email\":\"user@example.com\",\"password\":\"SecurePass123\"}}'");
    info!("   Profile: curl http://localhost:3000/profile -H 'Authorization: Bearer YOUR_TOKEN'\n");

    // Start server
    info!("🚀 Starting server on http://localhost:3000...\n");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// === Handlers ===

async fn root_handler() -> &'static str {
    "🔐 Authentication Demo API\n\nEndpoints:\n  GET  /health - Health check\n  POST /register - Register new user\n  POST /login - Login\n  POST /refresh - Refresh token\n  GET  /profile - User profile (requires auth)\n  GET  /admin - Admin dashboard (requires auth + admin role)\n"
}

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "auth-demo",
    }))
}

async fn register_handler(
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    info!("📝 Registration request for: {}", req.email);

    // Validate password strength
    if req.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Hash password
    let password_hash = hasher
        .hash(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create user (in production, save to database)
    let user = User {
        id: 1, // In production, get from database
        email: req.email.clone(),
        name: req.name,
        password_hash,
        roles: vec!["user".to_string()],
    };

    info!("✅ User registered: {}", user.email);

    Ok(Json(UserResponse::from(user)))
}

async fn login_handler(
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Extension(jwt): Extension<Arc<JwtManager>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    info!("🔑 Login attempt for: {}", req.email);

    // In production, fetch user from database
    // For demo, create a mock user
    let mock_password_hash = hasher
        .hash("SecurePass123")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = User {
        id: 1,
        email: "user@example.com".to_string(),
        name: "John Doe".to_string(),
        password_hash: mock_password_hash,
        roles: vec!["user".to_string(), "admin".to_string()],
    };

    // Verify email
    if req.email != user.email {
        info!("❌ Login failed: user not found");
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Verify password
    let is_valid = hasher
        .verify_timing_safe(&req.password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !is_valid {
        info!("❌ Login failed: invalid password");
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    // Generate JWT tokens
    let claims = Claims::new(user.id, user.email.clone(), user.roles.clone(), 24);

    let token = jwt
        .generate_token(&claims)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let refresh_token = jwt
        .generate_refresh_token(&claims)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!("✅ Login successful for: {}", user.email);

    Ok(Json(LoginResponse {
        token,
        refresh_token,
        expires_in: 24 * 3600,
        user: UserResponse::from(user),
    }))
}

async fn refresh_handler(
    Extension(jwt): Extension<Arc<JwtManager>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, (StatusCode, String)> {
    info!("🔄 Token refresh requested");

    // Validate refresh token
    let claims = jwt
        .validate_refresh_token(&req.refresh_token)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid refresh token".to_string(),
            )
        })?;

    // Generate new access token
    let new_claims = Claims::new(claims.user_id, claims.sub.clone(), claims.roles.clone(), 24);

    let token = jwt
        .generate_token(&new_claims)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    info!("✅ Token refreshed for user: {}", claims.user_id);

    Ok(Json(RefreshResponse {
        token,
        expires_in: 24 * 3600,
    }))
}

async fn profile_handler(
    Extension(jwt): Extension<Arc<JwtManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Extract and validate JWT token
    let claims = extract_claims_from_headers(&jwt, &headers)?;

    info!("👤 Profile accessed by user: {}", claims.user_id);

    Ok(Json(serde_json::json!({
        "user_id": claims.user_id,
        "email": claims.sub,
        "roles": claims.roles,
        "message": "Welcome to your profile!",
    })))
}

async fn admin_handler(
    Extension(jwt): Extension<Arc<JwtManager>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Extract and validate JWT token
    let claims = extract_claims_from_headers(&jwt, &headers)?;

    // Check admin role
    require_role(&claims, "admin")?;

    info!("👑 Admin dashboard accessed by user: {}", claims.user_id);

    Ok(Json(serde_json::json!({
        "message": "Welcome to the admin dashboard!",
        "user_id": claims.user_id,
        "email": claims.sub,
    })))
}

/// Helper function to extract claims from Authorization header
fn extract_claims_from_headers(
    jwt: &JwtManager,
    headers: &HeaderMap,
) -> Result<Claims, (StatusCode, String)> {
    // Get Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing Authorization header".to_string(),
        ))?;

    // Extract token from "Bearer <token>"
    let token = auth_header.strip_prefix("Bearer ").ok_or((
        StatusCode::UNAUTHORIZED,
        "Invalid Authorization header format".to_string(),
    ))?;

    // Validate token
    let claims = jwt.validate_token(token).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired token".to_string(),
        )
    })?;

    Ok(claims)
}
