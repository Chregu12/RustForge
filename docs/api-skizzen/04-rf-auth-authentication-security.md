# API Sketch: rf-auth - Authentication & Security

**Component**: rf-auth
**Version**: 0.1.0
**Status**: Draft
**Date**: 2025-01-09

## Overview

Production-ready authentication and security layer providing:
- Password hashing with bcrypt/argon2
- JWT token generation and validation
- Session management
- Authentication middleware for Axum
- User authentication flow
- Token refresh mechanism
- CSRF protection (optional)

## Goals

1. **Secure Password Storage**: Bcrypt/Argon2 hashing with salt
2. **JWT Token Management**: Generate, validate, refresh tokens
3. **Session Support**: Optional session-based authentication
4. **Middleware Integration**: Axum middleware for protected routes
5. **User Authentication**: Register, login, logout flows
6. **Token Refresh**: Refresh tokens for long-lived sessions
7. **Security Best Practices**: Timing-safe comparisons, secure defaults
8. **Integration**: Works with rf-core, rf-orm, rf-web

## Architecture

```
┌─────────────────────────────────────────┐
│          Application Layer              │
│  (Registration, Login, Protected Routes)│
└─────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│         rf-auth (Facade)                │
│  • PasswordHasher                       │
│  • JwtManager                           │
│  • AuthMiddleware                       │
│  • SessionManager (optional)            │
└─────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│    bcrypt    │ │ jsonwebtoken │ │ tower-sessions│
│   (hashing)  │ │ (JWT tokens) │ │  (sessions)  │
└──────────────┘ └──────────────┘ └──────────────┘
```

## Core Components

### 1. PasswordHasher

Secure password hashing with bcrypt or argon2.

```rust
use rf_auth::PasswordHasher;

// Create hasher with bcrypt (default)
let hasher = PasswordHasher::bcrypt(12)?; // cost = 12

// Or use argon2
let hasher = PasswordHasher::argon2()?;

// Hash password
let password = "secure_password_123";
let hash = hasher.hash(password)?;

// Verify password
let is_valid = hasher.verify(password, &hash)?;
assert!(is_valid);

// Timing-safe comparison
let is_same = hasher.verify_timing_safe("wrong_password", &hash)?;
assert!(!is_same);
```

### 2. JWT Token Manager

Generate and validate JWT tokens with claims.

```rust
use rf_auth::{JwtManager, Claims};
use chrono::{Utc, Duration};

// Create JWT manager with secret
let jwt = JwtManager::new("your-secret-key-min-32-chars")?;

// Create claims
let claims = Claims {
    sub: "user@example.com".to_string(),      // Subject (user ID)
    exp: (Utc::now() + Duration::hours(24)).timestamp(), // Expiry
    iat: Utc::now().timestamp(),              // Issued at
    user_id: 123,                             // Custom claim
    roles: vec!["user".to_string()],          // Custom claim
};

// Generate token
let token = jwt.generate_token(&claims)?;

// Validate and decode token
let decoded_claims = jwt.validate_token(&token)?;
assert_eq!(decoded_claims.user_id, 123);

// Check if token is expired
if jwt.is_expired(&decoded_claims) {
    println!("Token expired");
}
```

### 3. Authentication Middleware

Axum middleware for protecting routes with JWT authentication.

```rust
use rf_auth::middleware::require_auth;
use axum::{Router, routing::get};

// Protected route handler
async fn protected_handler(
    claims: Claims,  // Automatically extracted from JWT
) -> String {
    format!("Hello, user {}!", claims.user_id)
}

// Apply middleware to routes
let app = Router::new()
    .route("/protected", get(protected_handler))
    .layer(require_auth(jwt_manager));

// Or protect multiple routes
let protected_routes = Router::new()
    .route("/profile", get(profile_handler))
    .route("/settings", get(settings_handler))
    .layer(require_auth(jwt_manager));

let app = Router::new()
    .route("/public", get(public_handler))
    .nest("/api", protected_routes);
```

### 4. User Registration

Complete user registration with password hashing.

```rust
use rf_auth::{PasswordHasher, RegisterRequest};
use rf_orm::prelude::*;

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    email: String,
    password: String,
    name: String,
}

async fn register(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<UserResponse>> {
    // Validate password strength
    if req.password.len() < 8 {
        return Err(AppError::BadRequest {
            message: "Password must be at least 8 characters".to_string(),
        });
    }

    // Hash password
    let password_hash = hasher.hash(&req.password)?;

    // Create user
    let user = user::ActiveModel {
        email: Set(req.email.clone()),
        name: Set(req.name),
        password_hash: Set(password_hash),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };

    // Save to database
    let result = User::insert(user)
        .exec(db.connection())
        .await?;

    let user = User::find_by_id(result.last_insert_id)
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::Internal(anyhow!("User not found")))?;

    Ok(Json(UserResponse::from(user)))
}
```

### 5. User Login

Login endpoint with JWT token generation.

```rust
use rf_auth::{PasswordHasher, JwtManager, Claims, LoginRequest};

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

async fn login(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Extension(jwt): Extension<Arc<JwtManager>>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    // Find user by email
    let user = User::find()
        .filter(user::Column::Email.eq(&req.email))
        .filter(user::Column::DeletedAt.is_null())
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::Unauthorized)?;

    // Verify password (timing-safe)
    if !hasher.verify_timing_safe(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    // Generate JWT token
    let claims = Claims {
        sub: user.email.clone(),
        exp: (Utc::now() + Duration::hours(24)).timestamp(),
        iat: Utc::now().timestamp(),
        user_id: user.id,
        roles: vec!["user".to_string()],
    };

    let token = jwt.generate_token(&claims)?;
    let refresh_token = jwt.generate_refresh_token(&claims)?;

    Ok(Json(LoginResponse {
        token,
        refresh_token,
        expires_in: 24 * 3600, // 24 hours in seconds
        user: UserResponse::from(user),
    }))
}
```

### 6. Token Refresh

Refresh access token using refresh token.

```rust
#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
struct RefreshResponse {
    token: String,
    expires_in: i64,
}

async fn refresh_token(
    Extension(jwt): Extension<Arc<JwtManager>>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<RefreshResponse>> {
    // Validate refresh token
    let claims = jwt.validate_refresh_token(&req.refresh_token)?;

    // Generate new access token
    let new_claims = Claims {
        sub: claims.sub.clone(),
        exp: (Utc::now() + Duration::hours(24)).timestamp(),
        iat: Utc::now().timestamp(),
        user_id: claims.user_id,
        roles: claims.roles.clone(),
    };

    let token = jwt.generate_token(&new_claims)?;

    Ok(Json(RefreshResponse {
        token,
        expires_in: 24 * 3600,
    }))
}
```

### 7. Logout

Logout endpoint (token invalidation via blacklist or short TTL).

```rust
async fn logout(
    claims: Claims,  // Extracted from JWT
) -> AppResult<Json<serde_json::Value>> {
    // Option 1: Add token to blacklist (requires Redis/database)
    // blacklist.add(&claims.jti, claims.exp).await?;

    // Option 2: Client-side token deletion (simpler)
    // Just return success, client deletes token

    Ok(Json(serde_json::json!({
        "message": "Logged out successfully"
    })))
}
```

### 8. Protected Route with Role Check

Extract claims and check user roles.

```rust
use rf_auth::middleware::{require_auth, require_role};

async fn admin_handler(
    claims: Claims,
) -> AppResult<String> {
    // Check if user has admin role
    if !claims.roles.contains(&"admin".to_string()) {
        return Err(AppError::Forbidden {
            reason: "Admin access required".to_string(),
        });
    }

    Ok(format!("Admin dashboard for user {}", claims.user_id))
}

// Or use role middleware
let admin_routes = Router::new()
    .route("/dashboard", get(admin_handler))
    .layer(require_role("admin"));
```

### 9. Claims Extractor

Custom extractor for JWT claims in Axum handlers.

```rust
use rf_auth::Claims;
use axum::{extract::FromRequestParts, http::request::Parts};
use async_trait::async_trait;

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        // Get JWT manager from extensions
        let jwt = parts
            .extensions
            .get::<Arc<JwtManager>>()
            .ok_or(AppError::Internal(anyhow!("JWT manager not found")))?;

        // Validate and decode token
        let claims = jwt.validate_token(token)?;

        Ok(claims)
    }
}
```

### 10. Password Reset Flow

Password reset with token generation.

```rust
use uuid::Uuid;

// Step 1: Request password reset
async fn request_password_reset(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Json(req): Json<PasswordResetRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Find user
    let user = User::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::NotFound { resource: "User".into() })?;

    // Generate reset token
    let reset_token = Uuid::new_v4().to_string();
    let reset_expires = Utc::now() + Duration::hours(1);

    // Save token to database
    // ... update user with reset_token and reset_expires ...

    // Send email with reset link
    // send_email(&user.email, reset_token).await?;

    Ok(Json(serde_json::json!({
        "message": "Password reset email sent"
    })))
}

// Step 2: Reset password with token
async fn reset_password(
    Extension(db): Extension<Arc<DatabaseManager>>,
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Json(req): Json<ResetPasswordRequest>,
) -> AppResult<Json<serde_json::Value>> {
    // Find user by reset token
    let user = User::find()
        .filter(user::Column::ResetToken.eq(&req.token))
        .one(db.connection())
        .await?
        .ok_or_else(|| AppError::BadRequest {
            message: "Invalid reset token".to_string(),
        })?;

    // Check if token expired
    if user.reset_expires < Some(Utc::now()) {
        return Err(AppError::BadRequest {
            message: "Reset token expired".to_string(),
        });
    }

    // Hash new password
    let password_hash = hasher.hash(&req.new_password)?;

    // Update user
    let mut user_active: user::ActiveModel = user.into();
    user_active.password_hash = Set(password_hash);
    user_active.reset_token = Set(None);
    user_active.reset_expires = Set(None);
    user_active.updated_at = Set(Utc::now());
    user_active.update(db.connection()).await?;

    Ok(Json(serde_json::json!({
        "message": "Password reset successfully"
    })))
}
```

## Configuration

### Auth Config Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key (min 32 characters)
    pub jwt_secret: String,

    /// JWT access token expiry in hours
    #[serde(default = "default_token_expiry")]
    pub token_expiry_hours: u64,

    /// JWT refresh token expiry in days
    #[serde(default = "default_refresh_expiry")]
    pub refresh_expiry_days: u64,

    /// Password hashing algorithm (bcrypt or argon2)
    #[serde(default = "default_hash_algorithm")]
    pub hash_algorithm: String,

    /// Bcrypt cost (4-31, default 12)
    #[serde(default = "default_bcrypt_cost")]
    pub bcrypt_cost: u32,

    /// Minimum password length
    #[serde(default = "default_min_password_length")]
    pub min_password_length: usize,
}

fn default_token_expiry() -> u64 { 24 }
fn default_refresh_expiry() -> u64 { 7 }
fn default_hash_algorithm() -> String { "bcrypt".to_string() }
fn default_bcrypt_cost() -> u32 { 12 }
fn default_min_password_length() -> usize { 8 }
```

### TOML Configuration

```toml
[auth]
jwt_secret = "your-secret-key-change-in-production-min-32-chars"
token_expiry_hours = 24
refresh_expiry_days = 7
hash_algorithm = "bcrypt"
bcrypt_cost = 12
min_password_length = 8
```

### Environment Variables

```bash
APP__AUTH__JWT_SECRET="production-secret-key-very-long-and-random"
APP__AUTH__TOKEN_EXPIRY_HOURS=24
APP__AUTH__BCRYPT_COST=12
```

## JWT Claims Structure

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user identifier, usually email)
    pub sub: String,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// JWT ID (unique identifier for token)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,

    /// Custom: User ID
    pub user_id: i32,

    /// Custom: User roles
    pub roles: Vec<String>,
}

impl Claims {
    pub fn new(user_id: i32, email: String, roles: Vec<String>, expiry_hours: u64) -> Self {
        Self {
            sub: email,
            exp: (Utc::now() + Duration::hours(expiry_hours as i64)).timestamp(),
            iat: Utc::now().timestamp(),
            jti: Some(Uuid::new_v4().to_string()),
            user_id,
            roles,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.exp < Utc::now().timestamp()
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}
```

## Error Handling

```rust
use rf_auth::AuthError;

pub enum AuthError {
    /// Invalid credentials
    InvalidCredentials,

    /// Token expired
    TokenExpired,

    /// Invalid token format
    InvalidToken,

    /// Password too weak
    WeakPassword { reason: String },

    /// Hashing error
    HashingFailed { source: anyhow::Error },

    /// JWT error
    JwtError { source: jsonwebtoken::errors::Error },

    /// User not found
    UserNotFound,

    /// Email already exists
    EmailExists,
}

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidCredentials => AppError::Unauthorized,
            AuthError::TokenExpired => AppError::Unauthorized,
            AuthError::InvalidToken => AppError::Unauthorized,
            AuthError::WeakPassword { reason } => AppError::BadRequest { message: reason },
            AuthError::UserNotFound => AppError::NotFound { resource: "User".into() },
            AuthError::EmailExists => AppError::Conflict { message: "Email already exists".into() },
            _ => AppError::Internal(err.into()),
        }
    }
}
```

## Security Best Practices

### 1. Password Strength Validation

```rust
use rf_auth::PasswordValidator;

let validator = PasswordValidator::new()
    .min_length(8)
    .require_uppercase(true)
    .require_lowercase(true)
    .require_digit(true)
    .require_special_char(true);

let result = validator.validate("weak")?;
if !result.is_valid {
    return Err(AppError::BadRequest {
        message: result.errors.join(", "),
    });
}
```

### 2. Rate Limiting for Login

```rust
use rf_auth::RateLimiter;

// Limit to 5 login attempts per 15 minutes
let limiter = RateLimiter::new(5, Duration::minutes(15));

if limiter.is_limited(&req.email).await? {
    return Err(AppError::RateLimitExceeded);
}

// Process login...

limiter.record(&req.email).await?;
```

### 3. Secure Cookie Settings

```rust
use tower_sessions::{Session, SessionStore};

// Configure secure session cookies
let session_store = MemoryStore::default();
let session_layer = SessionManagerLayer::new(session_store)
    .with_secure(true)              // HTTPS only
    .with_http_only(true)           // Not accessible via JavaScript
    .with_same_site(SameSite::Strict) // CSRF protection
    .with_expiry(Expiry::OnInactivity(Duration::hours(24)));
```

### 4. CSRF Protection

```rust
use rf_auth::CsrfProtection;

// Generate CSRF token
let csrf_token = CsrfProtection::generate_token();

// Validate CSRF token
CsrfProtection::validate_token(&submitted_token, &session_token)?;
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let hasher = PasswordHasher::bcrypt(4).unwrap(); // Low cost for tests
        let password = "test_password";

        let hash = hasher.hash(password).unwrap();
        assert!(hasher.verify(password, &hash).unwrap());
        assert!(!hasher.verify("wrong", &hash).unwrap());
    }

    #[test]
    fn test_jwt_generation() {
        let jwt = JwtManager::new("test-secret-key-min-32-characters").unwrap();

        let claims = Claims::new(1, "test@example.com".into(), vec!["user".into()], 1);
        let token = jwt.generate_token(&claims).unwrap();

        let decoded = jwt.validate_token(&token).unwrap();
        assert_eq!(decoded.user_id, 1);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_registration_flow() {
    let test_db = TestDatabase::new().await;
    let hasher = PasswordHasher::bcrypt(4).unwrap();

    // Register user
    let req = RegisterRequest {
        email: "test@example.com".into(),
        password: "SecurePass123!".into(),
        name: "Test User".into(),
    };

    let response = register(
        Extension(Arc::new(test_db.manager().clone())),
        Extension(Arc::new(hasher)),
        Json(req),
    ).await.unwrap();

    assert_eq!(response.0.email, "test@example.com");
}
```

## Performance Considerations

### Password Hashing
- **Bcrypt Cost**: 12 = ~250ms per hash (secure default)
- **Argon2**: Similar performance, more resistant to ASIC attacks
- **Recommendation**: Use bcrypt cost 12 for production

### JWT Token Size
- Keep claims minimal (< 1KB)
- Store large data in database, reference by user_id
- Avoid embedding large objects in tokens

### Token Validation
- Validate tokens on every request (~0.1ms overhead)
- Consider caching validated tokens (with short TTL)
- Use middleware for automatic validation

## Summary

rf-auth provides:
- ✅ Secure password hashing (bcrypt/argon2)
- ✅ JWT token generation and validation
- ✅ Authentication middleware for Axum
- ✅ Claims extraction and role checking
- ✅ User registration and login flows
- ✅ Token refresh mechanism
- ✅ Password reset functionality
- ✅ Integration with rf-core, rf-orm, rf-web
- ✅ Security best practices built-in

Next: Implementation in `crates/rf-auth/`
