# rf-passport

Laravel Passport-style OAuth2 server implementation for RustForge.

## Features

- **Complete OAuth2 Server**: Full RFC 6749 implementation
- **Multiple Grant Types**:
  - Authorization Code with PKCE (RFC 7636)
  - Password Grant (Resource Owner Password Credentials)
  - Client Credentials
  - Implicit (deprecated but included for compatibility)
  - Refresh Token
- **PKCE Support**: Enhanced security for public clients
- **Personal Access Tokens**: Issue tokens without full OAuth flow
- **Scope Management**: Fine-grained permission control
- **Client Management**: Create and manage OAuth clients
- **Token Lifecycle**: Complete token management with revocation
- **Axum Integration**: Built-in middleware and extractors
- **SeaORM Models**: Database-backed persistence

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-passport = { path = "../rf-passport" }
```

## Quick Start

### 1. Configure Passport

```rust
use rf_passport::{PassportConfig, register_scopes};

let config = PassportConfig::new()
    .access_token_lifetime(3600)      // 1 hour
    .refresh_token_lifetime(2592000)  // 30 days
    .enforce_pkce(true)               // Require PKCE
    .enable_password_grant(false);    // Disable password grant

// Register available scopes
register_scopes! {
    "read:posts" => "Read blog posts",
    "write:posts" => "Create and edit blog posts",
    "delete:posts" => "Delete blog posts",
}
```

### 2. Setup Database Tables

Create migrations for:
- `oauth_clients`
- `oauth_access_tokens`
- `oauth_refresh_tokens`
- `oauth_auth_codes`

### 3. Personal Access Tokens

```rust
use rf_passport::HasApiTokens;

// Implement trait for your User model
#[async_trait]
impl HasApiTokens for User {
    fn get_id(&self) -> i64 {
        self.id
    }
}

// Create a token
let token = user.create_token(
    "mobile-app",
    vec!["read:posts".to_string(), "write:posts".to_string()],
    &db,
    &config
).await?;
```

### 4. Protect Routes

```rust
use rf_passport::PassportAuth;
use axum::{response::IntoResponse, Json};

// Basic authentication
async fn protected_route(
    PassportAuth(user_id, token): PassportAuth
) -> impl IntoResponse {
    Json(json!({
        "user_id": user_id,
        "scopes": token.get_scopes()
    }))
}

// With scope checking
async fn write_post(
    PassportAuth(user_id, token): PassportAuth
) -> Result<impl IntoResponse, PassportError> {
    check_scopes(&token, &["write:posts"]).await?;

    // Create post
    Ok(Json(json!({ "post_id": 123 })))
}
```

## OAuth2 Flows

### Authorization Code Flow (with PKCE)

**Step 1: Authorization Request**

```rust
use rf_passport::{AuthorizationCodeGrant, AuthorizationRequest};

let request = AuthorizationRequest {
    response_type: "code".to_string(),
    client_id: 1,
    redirect_uri: "http://localhost:3000/callback".to_string(),
    scope: Some("read:posts write:posts".to_string()),
    state: Some("random-state".to_string()),
    code_challenge: Some(challenge),
    code_challenge_method: Some("S256".to_string()),
};

let grant = AuthorizationCodeGrant::new(&db, &config);
let response = grant.authorize(user_id, request).await?;
// Returns: { code: "...", state: "..." }
```

**Step 2: Token Exchange**

```rust
let token_request = AuthorizationCodeTokenRequest {
    grant_type: "authorization_code".to_string(),
    code: response.code,
    redirect_uri: "http://localhost:3000/callback".to_string(),
    client_id: 1,
    client_secret: Some("secret".to_string()),
    code_verifier: Some(verifier),
};

let tokens = grant.exchange_token(token_request).await?;
// Returns: { access_token, refresh_token, expires_in, ... }
```

### Password Grant

```rust
use rf_passport::{PasswordGrant, PasswordVerifier};

// Implement password verifier
struct MyPasswordVerifier;

#[async_trait]
impl PasswordVerifier for MyPasswordVerifier {
    async fn verify(&self, username: &str, password: &str) -> PassportResult<i64> {
        // Verify credentials and return user ID
        let user = find_user_by_email(username).await?;
        verify_password(&user, password)?;
        Ok(user.id)
    }
}

// Issue token
let grant = PasswordGrant::new(&db, &config);
let tokens = grant.issue_token(request, &MyPasswordVerifier).await?;
```

### Client Credentials

```rust
use rf_passport::ClientCredentialsGrant;

let request = ClientCredentialsRequest {
    grant_type: "client_credentials".to_string(),
    client_id: 1,
    client_secret: "secret".to_string(),
    scope: Some("api:read".to_string()),
};

let grant = ClientCredentialsGrant::new(&db, &config);
let tokens = grant.issue_token(request).await?;
```

### Refresh Token

```rust
use rf_passport::RefreshTokenGrant;

let request = RefreshTokenRequest {
    grant_type: "refresh_token".to_string(),
    refresh_token: "old-refresh-token".to_string(),
    client_id: 1,
    client_secret: Some("secret".to_string()),
    scope: None, // Use same scopes as original token
};

let grant = RefreshTokenGrant::new(&db, &config);
let tokens = grant.refresh(request).await?;
```

## PKCE (Proof Key for Code Exchange)

PKCE enhances security for public clients (mobile/SPA apps):

```rust
use rf_passport::{generate_code_verifier, generate_code_challenge, CodeChallengeMethod};

// Step 1: Generate verifier and challenge
let verifier = generate_code_verifier();
let challenge = generate_code_challenge(&verifier, &CodeChallengeMethod::S256)?;

// Step 2: Use challenge in authorization request
// (send challenge to server)

// Step 3: Use verifier in token request
// (send verifier to server for verification)
```

## Scope Management

```rust
use rf_passport::{Scope, ScopeRepository, register_scopes};

// Method 1: Register with macro
register_scopes! {
    "read:posts" => "Read blog posts",
    "write:posts" => "Create and edit blog posts",
}

// Method 2: Register manually
ScopeRepository::register(Scope::new("admin", "Full admin access"));

// Validate scopes
let valid = ScopeRepository::validate(&["read:posts", "write:posts"])?;
```

## Client Management

```rust
use rf_passport::ClientRepository;

let client_repo = ClientRepository::new(&db);

// Create confidential client
let (client, secret) = client_repo.create(
    Some(user_id),
    "My Application",
    vec!["http://localhost:3000/callback".to_string()],
    false, // not personal access client
    false, // not password client
    true,  // confidential
).await?;

// Create password grant client
let (client, secret) = client_repo.create(
    None,
    "Password Client",
    vec![],
    false,
    true,  // password client
    true,
).await?;
```

## Token Management

```rust
use rf_passport::TokenRepository;

let token_repo = TokenRepository::new(&db);

// Find token
let token = token_repo.find_valid_access_token("token-id").await?;

// Revoke token
token_repo.revoke_access_token("token-id").await?;

// Revoke all user tokens
token_repo.revoke_all_user_tokens(user_id).await?;

// Cleanup expired tokens
let count = token_repo.cleanup_expired_access_tokens().await?;
```

## Route Handlers

rf-passport provides ready-to-use Axum handlers:

```rust
use rf_passport::handlers::*;
use axum::{Router, routing::{get, post, delete}};

let app = Router::new()
    // OAuth endpoints
    .route("/oauth/token", post(token_endpoint))
    .route("/oauth/tokens", get(list_tokens))
    .route("/oauth/tokens/:id", delete(revoke_token))
    .route("/oauth/clients", get(list_clients))
    .route("/oauth/clients", post(create_client))
    .route("/oauth/clients/:id", delete(delete_client))
    .with_state(passport_state);
```

## Configuration Options

```rust
PassportConfig {
    access_token_lifetime: 3600,           // 1 hour
    refresh_token_lifetime: 2592000,       // 30 days
    auth_code_lifetime: 600,               // 10 minutes
    personal_access_token_lifetime: None,  // Never expires
    enforce_pkce: true,                    // Require PKCE
    allow_plain_pkce: false,               // Only S256
    default_scopes: vec![],                // No default scopes
    enable_password_grant: false,          // Disabled (security)
    enable_implicit_grant: false,          // Deprecated
    enable_client_credentials_grant: true,
    enable_authorization_code_grant: true,
    enable_refresh_token_grant: true,
    require_client_authentication: true,
    token_length: 80,
}
```

## Security Best Practices

1. **Always use PKCE** for authorization code flow
2. **Disable password grant** in production (use authorization code instead)
3. **Never use implicit grant** (deprecated by OAuth 2.1)
4. **Use S256 for PKCE** (not plain)
5. **Rotate refresh tokens** (automatic in rf-passport)
6. **Set appropriate token lifetimes**
7. **Validate redirect URIs strictly**
8. **Use HTTPS in production**

## Database Schema

### oauth_clients

```sql
CREATE TABLE oauth_clients (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT,
    name VARCHAR(255) NOT NULL,
    secret VARCHAR(255),
    provider VARCHAR(255),
    redirect JSON NOT NULL,
    personal_access_client BOOLEAN NOT NULL DEFAULT FALSE,
    password_client BOOLEAN NOT NULL DEFAULT FALSE,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

### oauth_access_tokens

```sql
CREATE TABLE oauth_access_tokens (
    id VARCHAR(255) PRIMARY KEY,
    user_id BIGINT,
    client_id BIGINT NOT NULL,
    name VARCHAR(255),
    scopes JSON NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

### oauth_refresh_tokens

```sql
CREATE TABLE oauth_refresh_tokens (
    id VARCHAR(255) PRIMARY KEY,
    access_token_id VARCHAR(255) NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

### oauth_auth_codes

```sql
CREATE TABLE oauth_auth_codes (
    id VARCHAR(255) PRIMARY KEY,
    user_id BIGINT NOT NULL,
    client_id BIGINT NOT NULL,
    scopes JSON NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP NOT NULL,
    code_challenge VARCHAR(255),
    code_challenge_method VARCHAR(10),
    redirect_uri VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL
);
```

## Examples

See `examples/full_example.rs` for a complete working example demonstrating all features.

Run with:
```bash
cargo run --example full_example
```

## API Reference

See the [API documentation](https://docs.rs/rf-passport) for detailed information.

## License

MIT
