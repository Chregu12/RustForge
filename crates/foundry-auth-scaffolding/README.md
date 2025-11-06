# Foundry Auth Scaffolding

Complete authentication system for Rust web applications. Inspired by Laravel Breeze and Jetstream, providing drop-in authentication with minimal configuration.

## Features

- **User Registration & Login** - Secure password authentication with Argon2 hashing
- **Password Reset** - Email-based password reset flow with time-limited tokens
- **Email Verification** - Optional email verification for new accounts
- **Two-Factor Authentication** - TOTP-based 2FA with QR codes and recovery codes
- **Session Management** - Secure session handling with configurable lifetimes
- **Remember Me** - Extended sessions for trusted devices
- **Profile Management** - User profile updates and password changes
- **Axum Integration** - Pre-built route handlers for Axum web framework
- **Security First** - Built-in protection against common vulnerabilities

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
foundry-auth-scaffolding = "0.1"
```

### Basic Setup

```rust
use foundry_auth_scaffolding::{
    AuthService, AuthConfig, RegisterData, Credentials
};

// 1. Configure authentication
let config = AuthConfig {
    session_lifetime: 7200,  // 2 hours
    require_email_verification: false,
    enable_two_factor: false,
    app_name: "My App".to_string(),
    ..Default::default()
};

// 2. Create authentication service
let auth_service = AuthService::new(config);
```

### User Registration

```rust
use foundry_auth_scaffolding::RegisterData;

let register_data = RegisterData {
    name: "John Doe".to_string(),
    email: "john@example.com".to_string(),
    password: "secure_password_123".to_string(),
    password_confirmation: "secure_password_123".to_string(),
};

// Validate and register user
let user = auth_service.register(register_data)?;

// Password is automatically hashed with Argon2
println!("User created: {}", user.id);
```

### User Login

```rust
use foundry_auth_scaffolding::Credentials;

let credentials = Credentials {
    email: "john@example.com".to_string(),
    password: "secure_password_123".to_string(),
    remember: false,  // Set true for "Remember Me"
};

// Attempt authentication
auth_service.attempt(&user, &credentials)?;

// Create session
let session = auth_service.create_session(&user, credentials.remember);

println!("Session token: {}", session.token);
println!("Expires at: {}", session.expires_at);
```

## Web Framework Integration

### Axum

Pre-built Axum handlers are included:

```rust
use axum::{Router, extract::State};
use foundry_auth_scaffolding::{
    AuthService, AuthConfig, handlers::auth_routes
};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    auth_service: AuthService,
}

#[tokio::main]
async fn main() {
    let config = AuthConfig::default();
    let auth_service = AuthService::new(config);

    let state = Arc::new(AppState { auth_service });

    let app = Router::new()
        .nest("/auth", auth_routes())
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

This provides the following routes:

- `GET /auth/login` - Login page
- `POST /auth/login` - Login submission
- `GET /auth/register` - Registration page
- `POST /auth/register` - Registration submission
- `POST /auth/logout` - Logout
- `GET /auth/password/forgot` - Forgot password page
- `POST /auth/password/forgot` - Forgot password submission
- `GET /auth/password/reset` - Reset password page
- `POST /auth/password/reset` - Reset password submission
- `GET /auth/email/verify/:token` - Email verification

## Email Verification

### Enable Email Verification

```rust
let config = AuthConfig {
    require_email_verification: true,
    email_verification_lifetime: 86400,  // 24 hours
    ..Default::default()
};
```

### Generate Verification Token

```rust
use foundry_auth_scaffolding::models::EmailVerification;

let token = auth_service.generate_token();
let verification = EmailVerification::new(
    user.id,
    token.clone(),
    config.email_verification_lifetime
);

// Store verification in database
// Send email with verification link
let verification_url = format!(
    "{}/auth/email/verify/{}",
    config.app_url,
    token
);
```

### Verify Email

```rust
// Look up verification by token
let verification = db.find_verification(&token).await?;

// Validate token hasn't expired
if verification.is_expired() {
    return Err(AuthError::TokenExpired);
}

// Mark user email as verified
user.mark_email_as_verified();
db.update_user(&user).await?;

// Delete verification token
db.delete_verification(&verification.id).await?;
```

## Password Reset

### Request Password Reset

```rust
use foundry_auth_scaffolding::models::PasswordReset;

// User requests password reset
let email = "john@example.com";

// Generate reset token
let token = auth_service.generate_token();
let reset = PasswordReset::new(
    email.to_string(),
    token.clone(),
    config.password_reset_lifetime  // 1 hour
);

// Store reset token in database
db.store_password_reset(&reset).await?;

// Send password reset email
let reset_url = format!(
    "{}/auth/password/reset?token={}&email={}",
    config.app_url,
    token,
    urlencoding::encode(email)
);
```

### Reset Password

```rust
// Validate reset token
let reset = db.find_password_reset(&token).await?;

if reset.is_expired() {
    return Err(AuthError::TokenExpired);
}

// Validate email matches
if reset.email != user.email {
    return Err(AuthError::InvalidToken);
}

// Hash new password
let new_password_hash = auth_service.hash_password(&new_password)?;

// Update user password
user.password_hash = new_password_hash;
db.update_user(&user).await?;

// Delete reset token
db.delete_password_reset(&reset.id).await?;
```

## Two-Factor Authentication

### Enable 2FA

```rust
use foundry_auth_scaffolding::two_factor::TwoFactorService;

let two_factor = TwoFactorService::new(config.app_name.clone());

// Generate TOTP secret
let secret = two_factor.generate_secret();

// Generate recovery codes
let recovery_codes = two_factor.generate_recovery_codes(10);

// Generate QR code for authenticator app
let qr_code_svg = two_factor.generate_qr_code(&user.email, &secret)?;

// Display QR code and recovery codes to user
// User scans QR code with authenticator app (Google Authenticator, Authy, etc.)

// After user confirms setup, enable 2FA
user.enable_two_factor(secret, recovery_codes);
db.update_user(&user).await?;
```

### Verify 2FA Code

```rust
// During login, after password verification
if user.has_two_factor() {
    // Prompt user for 2FA code
    let code = "123456";  // From user input

    // Verify TOTP code
    if two_factor.verify_code(&user.two_factor_secret.unwrap(), code)? {
        // 2FA successful
        let session = auth_service.create_session(&user, remember);
        Ok(session)
    } else {
        Err(AuthError::InvalidTwoFactorCode)
    }
}
```

### Recovery Codes

```rust
// User lost access to authenticator app
let recovery_code = "1234-5678";  // From user input

// Verify and use recovery code
if let Some(mut codes) = user.two_factor_recovery_codes.clone() {
    if two_factor.use_recovery_code(&mut codes, recovery_code) {
        // Recovery code valid, update user
        user.two_factor_recovery_codes = Some(codes);
        db.update_user(&user).await?;

        // Allow login
        let session = auth_service.create_session(&user, false);
        Ok(session)
    } else {
        Err(AuthError::InvalidRecoveryCode)
    }
}
```

### Disable 2FA

```rust
// Verify password first
auth_service.verify_password(&password, &user.password_hash)?;

// Disable 2FA
user.disable_two_factor();
db.update_user(&user).await?;
```

## Session Management

### Session Configuration

```rust
let config = AuthConfig {
    session_lifetime: 7200,        // 2 hours (normal)
    remember_lifetime: 2592000,    // 30 days (remember me)
    ..Default::default()
};
```

### Create Session

```rust
let session = auth_service.create_session(&user, remember);

// Store session in database
db.store_session(&session).await?;

// Set session cookie
let cookie = Cookie::build("session_token", session.token.clone())
    .path("/")
    .http_only(true)
    .secure(true)  // HTTPS only
    .same_site(SameSite::Lax)
    .finish();
```

### Validate Session

```rust
// Get session token from cookie
let token = cookie.value();

// Look up session
let session = db.find_session_by_token(token).await?;

// Check if expired
if session.is_expired() {
    return Err(AuthError::SessionExpired);
}

// Update last activity
session.touch();
db.update_session(&session).await?;

// Get user
let user = db.find_user(session.user_id).await?;
```

### Logout

```rust
// Invalidate session
db.delete_session(&session.id).await?;

// Clear session cookie
let cookie = Cookie::build("session_token", "")
    .path("/")
    .max_age(Duration::seconds(0))
    .finish();
```

### Logout All Devices

```rust
// Delete all sessions for user
db.delete_all_user_sessions(user.id).await?;
```

## Middleware

### Require Authentication

```rust
use foundry_auth_scaffolding::middleware::RequireAuth;
use axum::{Router, routing::get};

let protected_routes = Router::new()
    .route("/dashboard", get(dashboard))
    .route("/profile", get(profile))
    .layer(RequireAuth::new(db.clone()));
```

### Optional Authentication

```rust
use foundry_auth_scaffolding::middleware::OptionalAuth;

let mixed_routes = Router::new()
    .route("/home", get(home))  // Works for both authenticated and guest users
    .layer(OptionalAuth::new(db.clone()));
```

## Password Requirements

The default password validation requires:
- Minimum 8 characters
- Password confirmation must match

### Custom Password Validation

```rust
impl RegisterData {
    pub fn validate_custom(&self) -> Result<(), AuthError> {
        if self.password != self.password_confirmation {
            return Err(AuthError::PasswordMismatch);
        }

        if self.password.len() < 12 {
            return Err(AuthError::PasswordTooShort);
        }

        if !self.password.chars().any(|c| c.is_numeric()) {
            return Err(AuthError::PasswordNeedsNumber);
        }

        if !self.password.chars().any(|c| c.is_uppercase()) {
            return Err(AuthError::PasswordNeedsUppercase);
        }

        if !self.password.chars().any(|c| "!@#$%^&*".contains(c)) {
            return Err(AuthError::PasswordNeedsSpecialChar);
        }

        Ok(())
    }
}
```

## Database Integration

### User Model

```rust
use sqlx::FromRow;
use foundry_auth_scaffolding::models::User;

// Implement database operations
impl User {
    async fn find_by_email(
        pool: &PgPool,
        email: &str
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(pool)
        .await
    }

    async fn create(
        pool: &PgPool,
        user: &User
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, name, email, password_hash, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *"
        )
        .bind(user.id)
        .bind(&user.name)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(pool)
        .await
    }
}
```

### Session Model

```rust
impl Session {
    async fn find_by_token(
        pool: &PgPool,
        token: &str
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(pool)
        .await
    }

    async fn delete_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM sessions WHERE expires_at < NOW()"
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
```

## Email Integration

Enable the `email` feature to use email functionality:

```toml
[dependencies]
foundry-auth-scaffolding = { version = "0.1", features = ["email"] }
```

### Send Email

```rust
#[cfg(feature = "email")]
use foundry_auth_scaffolding::email::EmailService;

let email_service = EmailService::new(
    "smtp.example.com".to_string(),
    587,
    "noreply@example.com".to_string(),
    "smtp_password".to_string(),
);

// Send verification email
email_service.send_verification_email(
    &user.email,
    &user.name,
    &verification_url
).await?;

// Send password reset email
email_service.send_password_reset_email(
    &user.email,
    &user.name,
    &reset_url
).await?;
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use foundry_auth_scaffolding::{AuthService, AuthConfig, RegisterData};

    #[test]
    fn test_user_registration() {
        let config = AuthConfig::default();
        let service = AuthService::new(config);

        let data = RegisterData {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            password_confirmation: "password123".to_string(),
        };

        let user = service.register(data).unwrap();
        assert_eq!(user.email, "test@example.com");
        assert!(!user.has_verified_email());
    }

    #[test]
    fn test_password_verification() {
        let config = AuthConfig::default();
        let service = AuthService::new(config);

        let password = "my_secure_password";
        let hash = service.hash_password(password).unwrap();

        assert!(service.verify_password(password, &hash).unwrap());
        assert!(!service.verify_password("wrong", &hash).unwrap());
    }
}
```

## Security Best Practices

See [SECURITY.md](SECURITY.md) for comprehensive security guidelines including:
- Password hashing with Argon2
- Session security
- CSRF protection
- Rate limiting
- Two-factor authentication
- Secure token generation

## Examples

See [EXAMPLES.md](EXAMPLES.md) for complete examples including:
- Full authentication flow
- Email verification setup
- Password reset implementation
- Two-factor authentication
- Session management
- Integration with different web frameworks

## Production Deployment

- [ ] Enable HTTPS for all authentication endpoints
- [ ] Use environment variables for sensitive configuration
- [ ] Implement rate limiting on login/register endpoints
- [ ] Enable email verification for production
- [ ] Configure session cleanup job
- [ ] Set up monitoring for failed login attempts
- [ ] Implement account lockout after multiple failed attempts
- [ ] Use secure, httpOnly cookies for sessions
- [ ] Configure CSRF protection
- [ ] Set appropriate session lifetimes

## License

MIT OR Apache-2.0
