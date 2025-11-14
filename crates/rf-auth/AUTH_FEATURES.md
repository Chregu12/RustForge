# Authentication Features - Security Documentation

## Overview

RustForge `rf-auth` now includes three production-ready authentication flows:

1. **Email Verification** - Secure email verification with JWT tokens
2. **Password Reset** - Secure password reset flow with time-limited tokens
3. **Remember Me** - Long-lived sessions with secure cookie management

All features follow security best practices and are ready for production use.

---

## Email Verification

### Features

- JWT-based verification tokens
- Configurable expiration (default: 24 hours)
- Email/User ID validation
- Middleware for protecting verified-only routes
- Integration with rf-mail for sending emails

### Security

- Tokens are signed with HS256 algorithm
- User ID and email are embedded in token for verification
- Tokens expire after configurable duration
- Token cannot be reused after verification

### Usage Example

```rust
use rf_auth::verification::{EmailVerification, Verifiable};
use rf_mail::{Mailer, MemoryMailer};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::time::Duration;

// 1. Implement Verifiable trait on your User model
#[derive(Clone)]
struct User {
    id: i64,
    email: String,
    email_verified_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl Verifiable for User {
    fn verification_email(&self) -> &str {
        &self.email
    }

    fn verification_user_id(&self) -> i64 {
        self.id
    }

    fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    async fn mark_verified(&mut self) -> rf_auth::AuthResult<()> {
        self.email_verified_at = Some(Utc::now());
        // Save to database here
        Ok(())
    }
}

// 2. Create verification manager
let verification = EmailVerification::with_default_ttl(
    std::env::var("APP_KEY").unwrap()
);

// 3. Send verification email (during registration)
let mailer = MemoryMailer::new();
user.send_verification_email(
    &mailer,
    &verification,
    "https://example.com"
).await?;

// 4. Verify email (when user clicks link)
user.verify_email(&token, &verification).await?;

// 5. Protect routes with RequireVerified middleware
use axum::{Router, routing::get, middleware};
use rf_auth::verification::RequireVerified;

let app = Router::new()
    .route("/dashboard", get(dashboard_handler))
    .layer(middleware::from_fn(RequireVerified::middleware::<User>));
```

### API Reference

#### `EmailVerification`

- `new(secret: String, ttl: Duration)` - Create with custom TTL
- `with_default_ttl(secret: String)` - Create with 24h TTL
- `generate_token(user_id, email)` - Generate verification token
- `verify_token(token)` - Verify and decode token
- `generate_url(base_url, user_id, email)` - Generate full verification URL

#### `Verifiable` Trait

- `verification_email()` - Get email to verify
- `verification_user_id()` - Get user ID
- `is_verified()` - Check verification status
- `mark_verified()` - Mark as verified (saves to DB)
- `send_verification_email(mailer, verification, base_url)` - Send email
- `verify_email(token, verification)` - Verify with token
- `resend_verification_email(...)` - Resend if not verified

---

## Password Reset

### Features

- JWT-based reset tokens
- Short expiration (default: 1 hour)
- Email/User ID validation
- Password hashing integration
- Integration with rf-mail for sending emails

### Security

- Tokens expire after 1 hour (configurable)
- Tokens are signed with HS256 algorithm
- User ID and email validation prevents token misuse
- New password is hashed before storage
- Tokens should be single-use (invalidate after reset)

### Usage Example

```rust
use rf_auth::password_reset::{PasswordReset, Resettable};
use rf_auth::PasswordHasher;
use rf_mail::{Mailer, MemoryMailer};
use async_trait::async_trait;
use std::time::Duration;

// 1. Implement Resettable trait on your User model
struct User {
    id: i64,
    email: String,
    password: String,
}

#[async_trait]
impl Resettable for User {
    fn reset_email(&self) -> &str {
        &self.email
    }

    fn reset_user_id(&self) -> i64 {
        self.id
    }

    async fn update_password(&mut self, new_password_hash: String) -> rf_auth::AuthResult<()> {
        self.password = new_password_hash;
        // Save to database here
        Ok(())
    }
}

// 2. Create password reset manager
let reset = PasswordReset::with_default_ttl(
    std::env::var("APP_KEY").unwrap()
);

// 3. User requests password reset
let mailer = MemoryMailer::new();
user.send_password_reset(
    &mailer,
    &reset,
    "https://example.com"
).await?;

// 4. User clicks reset link and submits new password
let hasher = PasswordHasher::bcrypt(12)?;
user.reset_password(&token, "new_password", &hasher, &reset).await?;

// 5. Optional: Verify token before showing form
let claims = user.verify_reset_token(&token, &reset)?;
```

### API Reference

#### `PasswordReset`

- `new(secret: String, ttl: Duration)` - Create with custom TTL
- `with_default_ttl(secret: String)` - Create with 1h TTL
- `generate_token(user_id, email)` - Generate reset token
- `verify_token(token)` - Verify and decode token
- `generate_url(base_url, user_id, email)` - Generate full reset URL

#### `Resettable` Trait

- `reset_email()` - Get email for reset
- `reset_user_id()` - Get user ID
- `update_password(new_hash)` - Update password (saves to DB)
- `send_password_reset(mailer, reset, base_url)` - Send reset email
- `reset_password(token, new_password, hasher, reset)` - Reset password
- `verify_reset_token(token, reset)` - Verify token validity

---

## Remember Me

### Features

- JWT-based remember tokens
- HTTP-only, secure, SameSite cookies
- Long expiration (default: 30 days)
- Optional token rotation for enhanced security
- Automatic authentication from cookie

### Security

- HTTP-only cookies (not accessible via JavaScript)
- Secure flag (transmitted only over HTTPS)
- SameSite=Strict (prevents CSRF attacks)
- Token rotation on each use (optional, recommended)
- Tokens expire after 30 days (configurable)

### Usage Example

```rust
use rf_auth::remember_me::{RememberMe, RememberMeMiddleware};
use axum::{
    Router, routing::get,
    response::{Response, IntoResponse},
    http::header,
    Json,
};
use std::sync::Arc;
use std::time::Duration;

// 1. Create remember me manager
let remember = Arc::new(RememberMe::with_default_ttl(
    std::env::var("APP_KEY").unwrap()
));

// 2. Login with remember me (set cookie)
async fn login(
    remember: Arc<RememberMe>,
    user_id: i64,
    remember_me: bool,
) -> Response {
    // Authenticate user...

    let mut response = Json(json!({"success": true})).into_response();

    if remember_me {
        if let Ok(cookie) = remember.create_cookie(user_id) {
            response.headers_mut().insert(header::SET_COOKIE, cookie);
        }
    }

    response
}

// 3. Use middleware for automatic authentication
async fn load_user(user_id: i64) -> Option<User> {
    // Load user from database
    User::find_by_id(user_id).await.ok()
}

let app = Router::new()
    .route("/", get(handler))
    .layer(axum::middleware::from_fn_with_state(
        remember.clone(),
        RememberMeMiddleware::middleware::<User, _>(load_user)
    ));

// 4. Logout (delete cookie)
async fn logout(remember: Arc<RememberMe>) -> Response {
    let mut response = Json(json!({"success": true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        remember.delete_cookie()
    );
    response
}
```

### API Reference

#### `RememberMe`

- `new(secret: String, ttl: Duration)` - Create with custom TTL
- `with_default_ttl(secret: String)` - Create with 30 days TTL
- `generate_token(user_id)` - Generate remember token
- `verify_token(token)` - Verify token and get user ID
- `verify_token_full(token)` - Get full token claims
- `create_cookie(user_id)` - Create secure cookie header
- `delete_cookie()` - Create cookie deletion header
- `rotate_token(old_token)` - Generate new token for same user

#### `RememberMeMiddleware`

- `middleware<T, F>(load_user)` - Middleware with token rotation
- `middleware_no_rotation<T, F>(load_user)` - Middleware without rotation

---

## Security Considerations

### Secret Key Management

All features require a secret key for JWT signing. Best practices:

- **Minimum 32 characters** (enforced where possible)
- **Cryptographically random** (use a password generator)
- **Environment variable** (never commit to version control)
- **Different per environment** (dev, staging, production)

Example:
```bash
# .env
APP_KEY=your-super-secret-key-at-least-32-characters-long-and-random
```

### Token Expiration

Recommended token lifetimes:

- **Email Verification:** 24 hours (default)
- **Password Reset:** 1 hour (default) - Short window reduces risk
- **Remember Me:** 30 days (default) - Balance security vs UX

### HTTPS Required

All authentication features should be used over HTTPS in production:

- Prevents token interception
- Enables secure cookies
- Required for password transmission

### Rate Limiting

Consider implementing rate limiting for:

- Password reset requests (prevent email bombing)
- Email verification resends (prevent spam)
- Login attempts (prevent brute force)

### Database Security

- Hash passwords with bcrypt (cost >= 12) or argon2
- Store verification/reset attempts in logs
- Consider implementing maximum attempts
- Invalidate tokens after use (store used tokens)

---

## Testing

Comprehensive test suite included:

- Email verification flow (generation, verification, expiration)
- Password reset flow (request, reset, expiration)
- Remember me flow (creation, validation, rotation)
- Security tests (invalid tokens, different secrets)
- Integration tests (complete user flows)

Run tests:
```bash
cargo test -p rf-auth --all-features
```

---

## Feature Flags

The email verification and password reset features require the `mail` feature flag:

```toml
[dependencies]
rf-auth = { version = "0.1", features = ["mail"] }
```

Remember Me is always available (no mail dependency).

---

## Migration Guide

If you're upgrading from a previous version:

1. Add `mail` feature flag if using email verification or password reset
2. Implement the new traits (`Verifiable`, `Resettable`) on your User model
3. Update routes to use new middleware
4. Configure secret key in environment variables
5. Test all flows in development before deploying

---

## Performance

- Token generation: <1ms (JWT signing)
- Token verification: <1ms (JWT verification)
- Email sending: Depends on mailer backend
- Cookie operations: <0.1ms

All operations are non-blocking and async-friendly.

---

## Troubleshooting

### Token Verification Fails

- Check secret key matches between generation and verification
- Verify token hasn't expired
- Ensure token is complete (no truncation)

### Cookies Not Working

- Verify HTTPS in production (required for secure cookies)
- Check SameSite settings for your use case
- Ensure cookies aren't blocked by browser

### Email Not Sending

- Check rf-mail configuration
- Verify SMTP credentials (if using SMTP)
- Check logs for error messages

---

## Support

For issues and questions:

- GitHub Issues: https://github.com/your-repo/rust-dx-framework/issues
- Documentation: See module-level docs in code
- Examples: See integration tests for usage examples

---

Generated with RustForge Framework v0.1.0
