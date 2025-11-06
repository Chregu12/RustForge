# Security Guide

This document outlines security considerations and best practices for using Foundry OAuth2 Server in production.

## Table of Contents

- [JWT Secret Management](#jwt-secret-management)
- [PKCE (Proof Key for Code Exchange)](#pkce-proof-key-for-code-exchange)
- [Client Authentication](#client-authentication)
- [Token Storage](#token-storage)
- [Token Lifetimes](#token-lifetimes)
- [HTTPS Requirements](#https-requirements)
- [Rate Limiting](#rate-limiting)
- [Scope Security](#scope-security)
- [Timing Attack Prevention](#timing-attack-prevention)
- [Security Checklist](#security-checklist)

## JWT Secret Management

### Requirements

The JWT secret is the most critical security component. It must:

- Have at least **256 bits (32 bytes)** of entropy
- Be generated using a cryptographically secure random number generator
- Be kept strictly confidential
- Never be committed to version control
- Be rotated periodically (recommended: every 90 days)

### Generating a Secure Secret

```rust
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose::STANDARD};

fn generate_jwt_secret() -> String {
    let mut secret_bytes = vec![0u8; 32];  // 256 bits
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    STANDARD.encode(&secret_bytes)
}
```

Or use OpenSSL:

```bash
openssl rand -base64 32
```

### Storage

**Production:**
- Use environment variables
- Use a secrets manager (AWS Secrets Manager, HashiCorp Vault, etc.)
- Use encrypted configuration files
- Never hardcode secrets

```rust
use std::env;

let jwt_secret = env::var("OAUTH2_JWT_SECRET")
    .expect("OAUTH2_JWT_SECRET environment variable must be set");
```

**Development:**
- Use `.env` files (never commit to git)
- Add `.env` to `.gitignore`
- Use different secrets for dev/staging/production

### Secret Rotation

When rotating JWT secrets:

1. Keep old secret for token validation during transition period
2. Sign new tokens with new secret
3. Gradually phase out old secret after all tokens expire
4. Implement dual-secret validation if needed

```rust
// Example: Dual-secret validation
pub struct TokenValidator {
    current_secret: String,
    previous_secret: Option<String>,
}

impl TokenValidator {
    pub fn validate_with_rotation(&self, token: &str) -> OAuth2Result<TokenClaims> {
        // Try current secret
        match self.validate_with_secret(token, &self.current_secret) {
            Ok(claims) => Ok(claims),
            Err(_) => {
                // Fallback to previous secret if exists
                if let Some(prev) = &self.previous_secret {
                    self.validate_with_secret(token, prev)
                } else {
                    Err(OAuth2Error::InvalidToken)
                }
            }
        }
    }
}
```

## PKCE (Proof Key for Code Exchange)

### Why PKCE is Critical

PKCE (RFC 7636) protects against:
- Authorization code interception attacks
- Malicious apps on the same device
- Cross-site request forgery

### Requirements

**Public Clients (Mobile, SPA):**
- PKCE is **REQUIRED**
- Server enforces PKCE for public clients
- Must use S256 challenge method

**Confidential Clients:**
- PKCE is **RECOMMENDED**
- Provides defense-in-depth
- Mitigates compromised redirect URIs

### Implementation

```rust
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

// Client generates code verifier (43-128 characters)
fn generate_code_verifier() -> String {
    use rand::Rng;
    let random_bytes: Vec<u8> = rand::thread_rng()
        .sample_iter(rand::distributions::Standard)
        .take(32)
        .collect();
    URL_SAFE_NO_PAD.encode(&random_bytes)
}

// Client computes S256 challenge
fn compute_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(&hash)
}

// Server validates using constant-time comparison
// (already implemented in foundry-oauth-server)
```

### Server Configuration

```rust
let config = OAuth2Config {
    enable_pkce: true,  // Highly recommended
    ..Default::default()
};

// Server automatically enforces PKCE for public clients
let code = server.create_authorization_code(
    &client,
    user_id,
    redirect_uri,
    scopes,
    Some(code_challenge),      // Required for public clients
    Some("S256".to_string()),  // Always use S256
)?;
```

## Client Authentication

### Confidential Clients

Confidential clients (server-side apps) authenticate using client credentials:

```rust
// Client secret is automatically hashed using Argon2
let client = Client::new(
    "Backend Service".to_string(),
    vec!["https://api.example.com/callback".to_string()]
);

// Secret is hashed before storage
repo.store(client).await?;

// Authenticate with secret
let authenticated_client = server.validate_client(
    client_id,
    Some(&client_secret)
).await?;
```

### Public Clients

Public clients (mobile, SPA) cannot securely store secrets:

```rust
let client = Client::public(
    "Mobile App".to_string(),
    vec!["myapp://callback".to_string()]
);

// No secret - must use PKCE
let authenticated_client = server.validate_client(
    client_id,
    None  // No secret
).await?;
```

### Best Practices

- **Never expose client secrets** in frontend code, mobile apps, or public repositories
- **Rotate client secrets** periodically or after suspected compromise
- **Use different clients** for different platforms (web, mobile, desktop)
- **Revoke compromised clients** immediately

## Token Storage

### Access Tokens

**Client-Side (Browser/Mobile):**
- Store in memory (JavaScript variables)
- **Never use localStorage** (vulnerable to XSS)
- **Never use cookies** (unless httpOnly + secure)
- Clear on logout

**Server-Side:**
- No need to store (stateless JWT)
- Optional: Store for revocation tracking

### Refresh Tokens

**Critical: Refresh tokens are long-lived credentials**

**Client-Side:**
- Use httpOnly, secure, SameSite cookies (web)
- Use secure storage (Keychain on iOS, KeyStore on Android)
- **Never use localStorage or sessionStorage**

**Server-Side:**
- **Must store** refresh tokens in database
- Store token hash (not plaintext)
- Track: user_id, client_id, issued_at, expires_at, revoked
- Implement token rotation on use

```rust
// Example: Refresh token rotation
async fn rotate_refresh_token(
    old_refresh_token: &RefreshToken
) -> OAuth2Result<(AccessToken, RefreshToken)> {
    // 1. Validate old refresh token
    // 2. Generate new access token
    // 3. Generate new refresh token
    // 4. Revoke old refresh token
    // 5. Store new refresh token
    // 6. Return both tokens
}
```

### Personal Access Tokens

- Store hashed in database
- Allow users to view, name, and revoke tokens
- Show last used timestamp
- Implement scoped permissions

## Token Lifetimes

### Recommended Lifetimes

```rust
let config = OAuth2Config {
    access_token_lifetime: 3600,           // 1 hour (short)
    refresh_token_lifetime: 2592000,       // 30 days (medium)
    auth_code_lifetime: 600,               // 10 minutes (very short)
    personal_access_token_lifetime: 31536000, // 1 year (long)
};
```

### Guidelines

**Access Tokens:**
- Short-lived (15 minutes to 1 hour)
- Contains user/client identity and scopes
- Compromise impact: Limited by lifetime
- No revocation needed (expires quickly)

**Refresh Tokens:**
- Longer-lived (days to months)
- Used only to obtain new access tokens
- Must be revocable
- Rotate on each use (recommended)

**Authorization Codes:**
- Very short-lived (5-10 minutes)
- Single-use only
- Bind to client (PKCE)

**Personal Access Tokens:**
- Long-lived (months to years)
- User-managed
- Must be revocable
- Implement last-used tracking

### Adjust Based on Risk

**High Security:**
- Shorter access tokens (15 minutes)
- Mandatory refresh token rotation
- Frequent re-authentication

**Usability-Focused:**
- Longer access tokens (1 hour)
- Longer refresh tokens (90 days)
- Optional re-authentication

## HTTPS Requirements

### Production

**All OAuth2 endpoints MUST use HTTPS:**
- Authorization endpoint
- Token endpoint
- Introspection endpoint
- Revocation endpoint

**All redirect URIs MUST use HTTPS** (except localhost for development)

### Configuration

```rust
// Validate redirect URIs
fn validate_redirect_uri(uri: &str) -> Result<(), String> {
    let url = Url::parse(uri)
        .map_err(|e| format!("Invalid URL: {}", e))?;

    // Production: HTTPS only
    if cfg!(not(debug_assertions)) && url.scheme() != "https" {
        // Exception for localhost
        if url.host_str() != Some("localhost") && url.host_str() != Some("127.0.0.1") {
            return Err("Redirect URI must use HTTPS in production".to_string());
        }
    }

    Ok(())
}
```

### TLS Configuration

- Use TLS 1.2 or higher
- Use strong cipher suites
- Keep certificates up to date
- Enable HSTS (HTTP Strict Transport Security)

```
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

## Rate Limiting

Implement rate limiting on all OAuth2 endpoints to prevent:
- Brute force attacks
- Token enumeration
- Denial of service

### Critical Endpoints

**Token Endpoint:**
```rust
// Example rate limit: 10 requests per minute per IP
Rate limit: 10/min per IP
Rate limit: 100/hour per client_id
```

**Authorization Endpoint:**
```rust
Rate limit: 30/min per IP
Rate limit: 100/hour per client_id
```

**Introspection Endpoint:**
```rust
Rate limit: 60/min per IP
```

### Implementation

```rust
use governor::{Quota, RateLimiter};

async fn token_endpoint(
    rate_limiter: &RateLimiter,
    client_ip: IpAddr,
) -> Result<TokenResponse, OAuth2Error> {
    // Check rate limit
    if rate_limiter.check_key(&client_ip).is_err() {
        return Err(OAuth2Error::RateLimitExceeded);
    }

    // Process token request
    // ...
}
```

## Scope Security

### Principle of Least Privilege

- Grant minimum scopes necessary
- Users should explicitly consent to scopes
- Display clear scope descriptions

### Dangerous Scopes

Mark sensitive scopes as dangerous:

```rust
scope_manager.register(Scope::new(
    "users:delete".to_string(),
    "Permanently delete user accounts".to_string(),
    true,  // dangerous
));

scope_manager.register(Scope::new(
    "admin".to_string(),
    "Full administrative access".to_string(),
    true,  // dangerous
));
```

### Scope Validation

Always validate scopes at:
1. Authorization time (user consent)
2. Token issuance
3. Resource access (API endpoints)

```rust
// Validate client is authorized for requested scopes
let validated = server.validate_scopes(&client, &requested_scopes)?;

// Validate token has required scope at API endpoint
async fn protected_endpoint(claims: TokenClaims) -> Result<Response> {
    if !claims.scopes.contains(&"users:read".to_string()) {
        return Err(ApiError::InsufficientScope);
    }
    // Process request
}
```

## Timing Attack Prevention

### Constant-Time Comparison

The library uses constant-time comparison for all security-critical operations:

**PKCE Verification:**
```rust
use subtle::ConstantTimeEq;

// Prevents timing attacks on code_verifier
let is_valid = computed_challenge.as_bytes()
    .ct_eq(challenge.as_bytes())
    .into();
```

**Client Secret Verification:**
```rust
// Argon2 password verification (constant-time)
Argon2::default()
    .verify_password(secret.as_bytes(), &parsed_hash)
```

### Avoid Timing Leaks

**Bad:**
```rust
// DON'T: Early return leaks information
if secret != stored_secret {
    return Err(Error::InvalidCredentials);
}
```

**Good:**
```rust
// DO: Constant-time comparison
use subtle::ConstantTimeEq;
if !secret.as_bytes().ct_eq(stored_secret.as_bytes()).into() {
    return Err(Error::InvalidCredentials);
}
```

## Security Checklist

### Deployment

- [ ] Strong JWT secret (256+ bits) from secure random source
- [ ] JWT secret stored in environment variable or secrets manager
- [ ] PKCE enabled and enforced for public clients
- [ ] All endpoints use HTTPS in production
- [ ] Redirect URIs validated and whitelist-only
- [ ] Client secrets hashed with Argon2
- [ ] Rate limiting implemented on all endpoints
- [ ] Token lifetimes configured appropriately
- [ ] Refresh tokens stored securely and support revocation
- [ ] Personal access tokens are revocable
- [ ] Scope validation at authorization and API access
- [ ] Dangerous scopes clearly marked and require explicit consent
- [ ] Logging excludes tokens and secrets
- [ ] Error messages don't leak sensitive information

### Monitoring

- [ ] Monitor for unusual token issuance patterns
- [ ] Track failed authentication attempts
- [ ] Alert on high-frequency token requests
- [ ] Monitor refresh token usage
- [ ] Track scope escalation attempts
- [ ] Log all client credential changes
- [ ] Alert on expired token usage attempts

### Incident Response

- [ ] Procedure for revoking compromised clients
- [ ] Process for rotating JWT secrets
- [ ] Plan for mass token revocation
- [ ] Communication plan for security incidents
- [ ] Regular security audits scheduled

## Security Vulnerability Reporting

If you discover a security vulnerability, please email security@example.com with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

**Do not open public issues for security vulnerabilities.**

## Additional Resources

- [OAuth 2.0 Security Best Current Practice](https://datatracker.ietf.org/doc/html/draft-ietf-oauth-security-topics)
- [RFC 6749 - OAuth 2.0 Authorization Framework](https://tools.ietf.org/html/rfc6749)
- [RFC 7636 - PKCE](https://tools.ietf.org/html/rfc7636)
- [RFC 7519 - JWT](https://tools.ietf.org/html/rfc7519)
- [RFC 7662 - Token Introspection](https://tools.ietf.org/html/rfc7662)
- [OWASP OAuth 2.0 Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)
