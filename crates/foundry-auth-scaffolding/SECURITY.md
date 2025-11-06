# Security Guide - Auth Scaffolding

This document outlines security considerations and best practices for using Foundry Auth Scaffolding in production environments.

## Table of Contents

- [Password Security](#password-security)
- [Session Security](#session-security)
- [Two-Factor Authentication](#two-factor-authentication)
- [Email Verification](#email-verification)
- [Password Reset Security](#password-reset-security)
- [CSRF Protection](#csrf-protection)
- [Rate Limiting](#rate-limiting)
- [Account Lockout](#account-lockout)
- [Timing Attack Prevention](#timing-attack-prevention)
- [HTTPS Requirements](#https-requirements)
- [Security Checklist](#security-checklist)

## Password Security

### Password Hashing

Foundry Auth uses Argon2, the winner of the Password Hashing Competition, for all password hashing:

**Why Argon2:**
- Memory-hard (resistant to GPU/ASIC attacks)
- Configurable memory, time, and parallelism
- Recommended by OWASP and security experts
- Protection against side-channel attacks

**Implementation:**
```rust
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{SaltString, PasswordHash};

// Hash password (done automatically by AuthService)
let salt = SaltString::generate(&mut OsRng);
let argon2 = Argon2::default();
let password_hash = argon2
    .hash_password(password.as_bytes(), &salt)?
    .to_string();

// Verify password (constant-time)
let parsed_hash = PasswordHash::new(&password_hash)?;
Argon2::default()
    .verify_password(password.as_bytes(), &parsed_hash)
    .is_ok()
```

### Password Requirements

**Default Requirements:**
- Minimum 8 characters
- Password confirmation must match

**Recommended Enhanced Requirements:**
```rust
pub fn validate_password_strength(password: &str) -> Result<(), String> {
    if password.len() < 12 {
        return Err("Password must be at least 12 characters".to_string());
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

    if !has_uppercase {
        return Err("Password must contain uppercase letter".to_string());
    }
    if !has_lowercase {
        return Err("Password must contain lowercase letter".to_string());
    }
    if !has_digit {
        return Err("Password must contain number".to_string());
    }
    if !has_special {
        return Err("Password must contain special character".to_string());
    }

    Ok(())
}
```

### Password Storage

**Never:**
- Store plaintext passwords
- Log passwords
- Include passwords in error messages
- Send passwords in URLs or GET parameters
- Return passwords in API responses

**Always:**
- Hash passwords using Argon2
- Use unique salts per password (automatic with Argon2)
- Validate password strength before hashing
- Clear password from memory after hashing

## Session Security

### Session Token Generation

Session tokens are generated using cryptographically secure random number generator:

```rust
use rand::Rng;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

fn generate_session_token() -> String {
    let random_bytes: Vec<u8> = rand::thread_rng()
        .sample_iter(rand::distributions::Standard)
        .take(64)  // 512 bits
        .collect();
    URL_SAFE_NO_PAD.encode(&random_bytes)
}
```

### Session Configuration

```rust
let config = AuthConfig {
    session_lifetime: 7200,        // 2 hours (short-lived)
    remember_lifetime: 2592000,    // 30 days (for remember me)
    ..Default::default()
};
```

**Guidelines:**
- Normal sessions: 2 hours (7200 seconds)
- Remember me: 30 days maximum (2592000 seconds)
- Administrative actions: 15 minutes
- Financial transactions: 10 minutes

### Session Cookies

**Required Settings:**
```rust
use axum::http::header::{SET_COOKIE, HeaderValue};
use cookie::{Cookie, SameSite};

let cookie = Cookie::build("session_token", session_token)
    .path("/")
    .http_only(true)        // Prevent JavaScript access
    .secure(true)           // HTTPS only (production)
    .same_site(SameSite::Lax)  // CSRF protection
    .max_age(Duration::seconds(config.session_lifetime))
    .finish();
```

**Security Attributes:**
- `HttpOnly`: Prevents XSS attacks from stealing cookies
- `Secure`: Only transmitted over HTTPS
- `SameSite=Lax`: Prevents CSRF attacks
- `Path=/`: Restricts cookie scope
- `Domain`: Set appropriately for your domain

### Session Storage

**Server-Side Storage (Required):**

```rust
// Store session in database
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    token VARCHAR(255) UNIQUE NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    last_activity TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    INDEX idx_token (token),
    INDEX idx_user_id (user_id),
    INDEX idx_expires_at (expires_at)
);
```

**Best Practices:**
- Store session metadata (IP, user agent) for security monitoring
- Index token column for fast lookups
- Implement session cleanup job to remove expired sessions
- Track last activity for automatic session extension
- Allow users to view and revoke active sessions

### Session Cleanup

```rust
// Run periodically (e.g., hourly cron job)
async fn cleanup_expired_sessions(pool: &PgPool) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM sessions WHERE expires_at < NOW()"
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
```

### Session Fixation Prevention

**Regenerate session on:**
- Login
- Logout
- Privilege escalation
- Password change

```rust
async fn regenerate_session(
    db: &Database,
    old_session: &Session,
    user: &User
) -> Result<Session> {
    // Delete old session
    db.delete_session(&old_session.id).await?;

    // Create new session
    let new_token = generate_session_token();
    let new_session = Session::new(user.id, new_token, lifetime);

    db.store_session(&new_session).await?;

    Ok(new_session)
}
```

## Two-Factor Authentication

### TOTP Security

Time-based One-Time Passwords (TOTP) provide strong second-factor authentication:

**Implementation:**
```rust
use totp_rs::{Algorithm, Secret, TOTP};

// Generate secret (base32 encoded)
let secret = two_factor.generate_secret();

// Create TOTP instance
let totp = TOTP::new(
    Algorithm::SHA1,  // Standard for authenticator apps
    6,                // 6-digit codes
    1,                // 1 step tolerance
    30,               // 30-second time step
    Secret::Encoded(secret.to_string()).to_bytes()?,
)?;

// Verify code with time tolerance
totp.check_current(&code)?
```

### Recovery Codes

**Generation:**
```rust
// Generate 10 recovery codes
let recovery_codes = two_factor.generate_recovery_codes(10);

// Format: XXXX-XXXX (8 digits with separator)
// Example: "1234-5678"
```

**Storage:**
```rust
// Hash recovery codes before storing (like passwords)
use argon2::{Argon2, PasswordHasher};

let hashed_codes: Vec<String> = recovery_codes
    .iter()
    .map(|code| {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(code.as_bytes(), &salt)
            .unwrap()
            .to_string()
    })
    .collect();

user.two_factor_recovery_codes = Some(hashed_codes);
```

**Usage:**
- Display recovery codes only once during setup
- Require user to download or write them down
- Mark codes as used (delete from list)
- Prompt to regenerate when few codes remain
- Use constant-time comparison to prevent timing attacks

### Backup Methods

Provide alternative 2FA recovery methods:
- SMS/phone verification (less secure, but better than nothing)
- Email recovery link (for account recovery)
- Trusted device recognition
- Account recovery through support

### 2FA Enforcement

**For High-Risk Accounts:**
```rust
// Require 2FA for administrators
if user.is_admin() && !user.has_two_factor() {
    return Err(AuthError::TwoFactorRequired);
}

// Require 2FA after certain actions
if user.accessed_sensitive_data() && !verified_2fa_recently() {
    return Err(AuthError::TwoFactorRequired);
}
```

## Email Verification

### Verification Token Security

```rust
use rand::Rng;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

// Generate secure random token
let random_bytes: Vec<u8> = rand::thread_rng()
    .sample_iter(rand::distributions::Standard)
    .take(32)  // 256 bits
    .collect();
let token = URL_SAFE_NO_PAD.encode(&random_bytes);

// Create verification with expiration
let verification = EmailVerification::new(
    user.id,
    token.clone(),
    86400  // 24 hours
);
```

### Token Lifetime

- **Recommended:** 24 hours (86400 seconds)
- **Maximum:** 7 days
- **Minimum:** 1 hour

### Token Validation

```rust
// Validate token
let verification = db.find_verification(&token).await?;

// Check expiration
if verification.is_expired() {
    db.delete_verification(&verification.id).await?;
    return Err(AuthError::TokenExpired);
}

// Verify user ID matches
if verification.user_id != user.id {
    return Err(AuthError::InvalidToken);
}

// Mark email as verified
user.mark_email_as_verified();
db.update_user(&user).await?;

// Delete token (single-use)
db.delete_verification(&verification.id).await?;
```

### Resend Limits

Prevent email flooding:
```rust
// Limit: 3 verification emails per hour
const MAX_VERIFICATION_EMAILS: usize = 3;
const VERIFICATION_WINDOW: i64 = 3600;

async fn can_resend_verification(
    db: &Database,
    user_id: Uuid
) -> Result<bool> {
    let count = db.count_verification_emails_sent(
        user_id,
        Utc::now() - Duration::seconds(VERIFICATION_WINDOW)
    ).await?;

    Ok(count < MAX_VERIFICATION_EMAILS)
}
```

## Password Reset Security

### Reset Token Generation

```rust
// Generate cryptographically secure token
let token = auth_service.generate_token();  // 256 bits

let reset = PasswordReset::new(
    email.to_string(),
    token.clone(),
    3600  // 1 hour expiration
);
```

### Token Lifetime

- **Recommended:** 1 hour (3600 seconds)
- **Maximum:** 24 hours
- **Minimum:** 15 minutes

### Reset Token Validation

```rust
// Validate reset token
let reset = db.find_password_reset(&token).await?;

// Check expiration
if reset.is_expired() {
    db.delete_password_reset(&reset.id).await?;
    return Err(AuthError::TokenExpired);
}

// Verify email matches user
if reset.email != user.email {
    return Err(AuthError::InvalidToken);
}

// Update password
let new_hash = auth_service.hash_password(&new_password)?;
user.password_hash = new_hash;
db.update_user(&user).await?;

// Delete token (single-use)
db.delete_password_reset(&reset.id).await?;

// Invalidate all sessions (force re-login)
db.delete_all_user_sessions(user.id).await?;

// Optional: Send email notification
email_service.send_password_changed_notification(&user).await?;
```

### Rate Limiting

Prevent abuse:
```rust
// Limit: 3 password reset requests per hour per email
const MAX_RESET_REQUESTS: usize = 3;
const RESET_WINDOW: i64 = 3600;

async fn can_request_reset(
    db: &Database,
    email: &str
) -> Result<bool> {
    let count = db.count_reset_requests(
        email,
        Utc::now() - Duration::seconds(RESET_WINDOW)
    ).await?;

    Ok(count < MAX_RESET_REQUESTS)
}
```

## CSRF Protection

### Token Generation

```rust
use rand::Rng;
use base64::{Engine as _, engine::general_purpose::STANDARD};

fn generate_csrf_token() -> String {
    let random_bytes: Vec<u8> = rand::thread_rng()
        .sample_iter(rand::distributions::Standard)
        .take(32)
        .collect();
    STANDARD.encode(&random_bytes)
}
```

### Implementation

**Store in Session:**
```rust
// Generate and store with session
let csrf_token = generate_csrf_token();
session.csrf_token = Some(csrf_token.clone());
db.update_session(&session).await?;
```

**Include in Forms:**
```html
<form method="POST" action="/auth/login">
    <input type="hidden" name="_csrf" value="{{ csrf_token }}">
    <!-- other fields -->
</form>
```

**Validate on Submission:**
```rust
async fn validate_csrf(
    session: &Session,
    submitted_token: &str
) -> Result<()> {
    use subtle::ConstantTimeEq;

    let session_token = session.csrf_token
        .as_ref()
        .ok_or(AuthError::MissingCsrfToken)?;

    // Constant-time comparison
    if !session_token.as_bytes().ct_eq(submitted_token.as_bytes()).into() {
        return Err(AuthError::InvalidCsrfToken);
    }

    Ok(())
}
```

### SameSite Cookie Attribute

Provides automatic CSRF protection:
```rust
let cookie = Cookie::build("session_token", token)
    .same_site(SameSite::Lax)  // Prevents cross-site requests
    .finish();
```

## Rate Limiting

### Login Attempts

```rust
// Limit: 5 failed attempts per IP per 15 minutes
const MAX_LOGIN_ATTEMPTS: usize = 5;
const LOGIN_WINDOW: i64 = 900;  // 15 minutes

async fn check_login_rate_limit(
    db: &Database,
    ip: &str
) -> Result<()> {
    let attempts = db.count_failed_logins(
        ip,
        Utc::now() - Duration::seconds(LOGIN_WINDOW)
    ).await?;

    if attempts >= MAX_LOGIN_ATTEMPTS {
        return Err(AuthError::RateLimitExceeded);
    }

    Ok(())
}
```

### Registration Rate Limiting

```rust
// Limit: 3 registrations per IP per hour
const MAX_REGISTRATIONS: usize = 3;
const REGISTRATION_WINDOW: i64 = 3600;

async fn check_registration_rate_limit(
    db: &Database,
    ip: &str
) -> Result<()> {
    let count = db.count_registrations(
        ip,
        Utc::now() - Duration::seconds(REGISTRATION_WINDOW)
    ).await?;

    if count >= MAX_REGISTRATIONS {
        return Err(AuthError::RateLimitExceeded);
    }

    Ok(())
}
```

## Account Lockout

### Failed Login Tracking

```rust
// Lock account after 10 failed attempts
const MAX_FAILED_ATTEMPTS: usize = 10;
const LOCKOUT_DURATION: i64 = 1800;  // 30 minutes

async fn handle_failed_login(
    db: &Database,
    user: &mut User
) -> Result<()> {
    user.failed_login_attempts += 1;
    user.last_failed_login = Some(Utc::now());

    if user.failed_login_attempts >= MAX_FAILED_ATTEMPTS {
        user.locked_until = Some(
            Utc::now() + Duration::seconds(LOCKOUT_DURATION)
        );
    }

    db.update_user(user).await?;
    Ok(())
}

async fn check_account_lockout(user: &User) -> Result<()> {
    if let Some(locked_until) = user.locked_until {
        if Utc::now() < locked_until {
            return Err(AuthError::AccountLocked {
                until: locked_until
            });
        }
    }

    Ok(())
}
```

### Unlock Account

```rust
async fn unlock_account(db: &Database, user: &mut User) -> Result<()> {
    user.failed_login_attempts = 0;
    user.locked_until = None;
    user.last_failed_login = None;
    db.update_user(user).await?;
    Ok(())
}
```

## Timing Attack Prevention

### Constant-Time Comparison

Always use constant-time comparison for security-critical operations:

```rust
use subtle::ConstantTimeEq;

// Password verification (done automatically by Argon2)
// Session token validation
// CSRF token validation
// Recovery code validation

fn verify_token(expected: &str, provided: &str) -> bool {
    expected.as_bytes().ct_eq(provided.as_bytes()).into()
}
```

### Prevent Username Enumeration

**Bad:**
```rust
// DON'T: Reveals if email exists
let user = db.find_user_by_email(&email).await?;
if user.is_none() {
    return Err(AuthError::EmailNotFound);
}
```

**Good:**
```rust
// DO: Same response for invalid email or password
let user = match db.find_user_by_email(&email).await? {
    Some(u) => u,
    None => {
        // Still hash password to prevent timing leak
        let _ = auth_service.hash_password("dummy");
        return Err(AuthError::InvalidCredentials);
    }
};

if !auth_service.verify_password(&password, &user.password_hash)? {
    return Err(AuthError::InvalidCredentials);
}
```

## HTTPS Requirements

### Production Requirements

**All authentication endpoints MUST use HTTPS:**
- Login
- Registration
- Password reset
- Email verification
- Two-factor authentication
- Session management

### Cookie Security

```rust
let cookie = Cookie::build("session_token", token)
    .secure(cfg!(not(debug_assertions)))  // HTTPS only in production
    .finish();
```

### Redirect HTTP to HTTPS

```rust
use axum::middleware;

async fn redirect_https(req: Request<Body>) -> Result<Response> {
    if !req.uri().scheme_str().map(|s| s == "https").unwrap_or(false) {
        let https_url = format!("https://{}{}", req.uri().authority()?, req.uri().path());
        Ok(Redirect::permanent(&https_url).into_response())
    } else {
        Ok(next.run(req).await)
    }
}
```

## Security Checklist

### Development
- [ ] Never commit secrets to version control
- [ ] Use environment variables for configuration
- [ ] Implement proper error handling (don't leak info)
- [ ] Write security tests
- [ ] Review dependencies for vulnerabilities

### Deployment
- [ ] Enable HTTPS for all endpoints
- [ ] Use secure, httpOnly session cookies
- [ ] Configure SameSite cookie attribute
- [ ] Implement CSRF protection
- [ ] Enable rate limiting on auth endpoints
- [ ] Implement account lockout
- [ ] Configure appropriate session lifetimes
- [ ] Enable email verification (recommended)
- [ ] Offer two-factor authentication
- [ ] Implement session cleanup job
- [ ] Log security events (failed logins, lockouts, etc.)
- [ ] Set up monitoring and alerting

### Monitoring
- [ ] Monitor failed login attempts
- [ ] Track account lockouts
- [ ] Alert on unusual patterns
- [ ] Monitor 2FA enrollment rate
- [ ] Track password reset requests
- [ ] Monitor session creation/destruction
- [ ] Log privilege escalations

### Maintenance
- [ ] Regular security audits
- [ ] Keep dependencies updated
- [ ] Review and rotate secrets
- [ ] Clean up old sessions
- [ ] Review and update rate limits
- [ ] Test backup/recovery procedures
- [ ] Update security documentation

## Security Vulnerability Reporting

If you discover a security vulnerability, please email security@example.com with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

**Do not open public issues for security vulnerabilities.**

## Additional Resources

- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [OWASP Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [NIST Digital Identity Guidelines](https://pages.nist.gov/800-63-3/)
