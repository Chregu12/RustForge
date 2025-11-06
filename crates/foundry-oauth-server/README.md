# Foundry OAuth2 Server

A complete OAuth2 Authorization Server implementation for Rust, inspired by Laravel Passport. Build secure, production-ready OAuth2 servers with minimal boilerplate.

## Features

- **Complete OAuth2 Flows**
  - Authorization Code Grant (with PKCE support)
  - Client Credentials Grant
  - Password Grant (Resource Owner Password Credentials)
  - Refresh Token Grant

- **Personal Access Tokens** - Simple API tokens for first-party clients
- **Scope Management** - Fine-grained access control with OAuth2 scopes
- **Token Introspection** - RFC 7662 compliant token introspection
- **Token Revocation** - Revoke access and refresh tokens
- **JWT Access Tokens** - Cryptographically signed JWT tokens
- **PKCE Support** - Proof Key for Code Exchange for public clients
- **Client Management** - Register and manage OAuth2 clients
- **Production Ready** - Security best practices built-in

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
foundry-oauth-server = "0.1"
```

### Basic Usage

```rust
use foundry_oauth_server::{OAuth2Server, OAuth2Config, InMemoryClientRepository};
use foundry_oauth_server::ClientBuilder;

// 1. Configure the server
let config = OAuth2Config {
    access_token_lifetime: 3600,      // 1 hour
    refresh_token_lifetime: 2592000,  // 30 days
    enable_pkce: true,
    ..Default::default()
};

// 2. Create client repository (use database in production)
let repo = InMemoryClientRepository::new();

// 3. Register an OAuth2 client
let client = ClientBuilder::new()
    .name("My Application".to_string())
    .redirect_uri("https://myapp.com/callback".to_string())
    .grants(vec![
        "authorization_code".to_string(),
        "refresh_token".to_string()
    ])
    .scopes(vec!["users:read".to_string(), "users:write".to_string()])
    .build()?;

repo.store(client).await?;

// 4. Create the OAuth2 server
let server = OAuth2Server::new(config, repo);
```

## OAuth2 Flows

### Authorization Code Flow

The most secure flow for server-side applications:

```rust
use uuid::Uuid;

// Step 1: User authorizes, create authorization code
let code = server.create_authorization_code(
    &client,
    user_id,
    "https://myapp.com/callback".to_string(),
    vec!["users:read".to_string()],
    None,  // code_challenge for PKCE
    None,  // code_challenge_method
)?;

// Step 2: Exchange code for tokens
let token_response = server.exchange_authorization_code(
    &client,
    &code,
    None,  // code_verifier for PKCE
).await?;

println!("Access Token: {}", token_response.access_token);
println!("Refresh Token: {:?}", token_response.refresh_token);
```

### Authorization Code Flow with PKCE

For mobile and single-page applications:

```rust
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

// Client generates code verifier
let code_verifier = "random_generated_string_43_chars_min";

// Client computes S256 challenge
let mut hasher = Sha256::new();
hasher.update(code_verifier.as_bytes());
let hash = hasher.finalize();
let code_challenge = URL_SAFE_NO_PAD.encode(&hash);

// Step 1: Create authorization code with PKCE
let code = server.create_authorization_code(
    &client,
    user_id,
    "https://myapp.com/callback".to_string(),
    vec!["users:read".to_string()],
    Some(code_challenge),
    Some("S256".to_string()),
)?;

// Step 2: Exchange code with verifier
let token_response = server.exchange_authorization_code(
    &client,
    &code,
    Some(code_verifier.to_string()),
).await?;
```

### Client Credentials Flow

For machine-to-machine authentication:

```rust
let token_response = server.issue_client_credentials_token(
    &client,
    vec!["api:read".to_string()],
).await?;

// Note: No refresh token for client credentials
println!("Access Token: {}", token_response.access_token);
```

### Password Grant Flow

For first-party applications (use with caution):

```rust
// After authenticating user credentials yourself
let token_response = server.issue_password_token(
    &client,
    user_id,
    vec!["users:read".to_string(), "users:write".to_string()],
).await?;

println!("Access Token: {}", token_response.access_token);
println!("Refresh Token: {:?}", token_response.refresh_token);
```

### Refresh Token Flow

Exchange refresh token for new access token:

```rust
let token_response = server.refresh_token(
    &client,
    &refresh_token,
    &access_token,
).await?;

println!("New Access Token: {}", token_response.access_token);
println!("New Refresh Token: {:?}", token_response.refresh_token);
```

### Personal Access Tokens

Simple API tokens for first-party applications:

```rust
let token = server.create_personal_access_token(
    user_id,
    "My API Token".to_string(),
    vec!["users:read".to_string()],
)?;

println!("Personal Access Token: {}", token.token);
```

## Token Validation

### Validate Access Tokens

```rust
use foundry_oauth_server::tokens::TokenValidator;

let validator = TokenValidator::new(
    config.jwt_secret.clone(),
    config.issuer.clone()
);

// Validate and decode token
let claims = validator.validate_access_token(&access_token)?;

println!("User ID: {:?}", claims.user_id);
println!("Client ID: {}", claims.client_id);
println!("Scopes: {:?}", claims.scopes);
println!("Expires: {}", claims.exp);
```

### Token Introspection (RFC 7662)

```rust
let introspection = server.introspect_token(&access_token);

if introspection.active {
    println!("Token is valid");
    println!("Scopes: {:?}", introspection.scope);
    println!("Client: {:?}", introspection.client_id);
} else {
    println!("Token is invalid or expired");
}
```

## Client Management

### Confidential Clients

For server-side applications with secure secret storage:

```rust
let client = ClientBuilder::new()
    .name("Backend Service".to_string())
    .redirect_uri("https://api.example.com/callback".to_string())
    .confidential()  // Default
    .grants(vec!["authorization_code".to_string()])
    .build()?;

let stored_client = repo.store(client).await?;

// Client secret is hashed, original is redacted
println!("Client ID: {}", stored_client.id);
println!("Client Secret: {:?}", stored_client.secret); // "***REDACTED***"
```

### Public Clients

For mobile and single-page applications (PKCE required):

```rust
let client = ClientBuilder::new()
    .name("Mobile App".to_string())
    .redirect_uri("myapp://callback".to_string())
    .public()  // No client secret
    .grants(vec!["authorization_code".to_string()])
    .build()?;

repo.store(client).await?;
```

### Authenticate Clients

```rust
// Confidential client authentication
let client = server.validate_client(
    client_id,
    Some(&client_secret)
).await?;

// Public client (no secret)
let client = server.validate_client(
    client_id,
    None
).await?;
```

## Scope Management

### Define Custom Scopes

```rust
use foundry_oauth_server::scopes::{Scope, ScopeManager};

let mut scope_manager = ScopeManager::new();

scope_manager.register(Scope::new(
    "users:read".to_string(),
    "Read user information".to_string(),
    false,  // not dangerous
));

scope_manager.register(Scope::new(
    "users:delete".to_string(),
    "Delete user accounts".to_string(),
    true,  // dangerous scope
));

scope_manager.register(Scope::new(
    "admin".to_string(),
    "Full administrative access".to_string(),
    true,  // dangerous scope
));
```

### Validate Scopes

```rust
// Validate requested scopes exist
let validated = scope_manager.validate(&["users:read", "users:write"])?;

// Check if granted scopes satisfy required scopes
let has_access = scope_manager.satisfies(
    &["users:read", "users:write"],  // granted
    &["users:read"]                   // required
);
```

### Filter Scopes

```rust
// Get all dangerous scopes
let dangerous = scope_manager.dangerous();

// Filter by prefix
let user_scopes = scope_manager.filter("users:");
```

## Configuration

### Production Configuration

```rust
use foundry_oauth_server::OAuth2Config;

let config = OAuth2Config {
    // Token lifetimes
    access_token_lifetime: 3600,           // 1 hour
    refresh_token_lifetime: 2592000,       // 30 days
    auth_code_lifetime: 600,               // 10 minutes
    personal_access_token_lifetime: 31536000, // 1 year

    // JWT configuration
    jwt_secret: env::var("OAUTH2_JWT_SECRET")
        .expect("OAUTH2_JWT_SECRET must be set"),
    issuer: "https://auth.yourapp.com".to_string(),

    // Security
    enable_pkce: true,  // Highly recommended
};
```

### Environment Variables

```bash
# Required
OAUTH2_JWT_SECRET=your-256-bit-secret-key-base64-encoded

# Optional
OAUTH2_ISSUER=https://auth.yourapp.com
OAUTH2_ACCESS_TOKEN_LIFETIME=3600
OAUTH2_REFRESH_TOKEN_LIFETIME=2592000
```

## Integration with Web Frameworks

### Axum Example

```rust
use axum::{routing::post, Router, Json};
use foundry_oauth_server::OAuth2Server;

async fn token_endpoint(
    State(server): State<Arc<OAuth2Server<MyRepository>>>,
    Form(request): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, OAuth2Error> {
    match request.grant_type.as_str() {
        "authorization_code" => {
            // Handle authorization code exchange
            let client = server.validate_client(
                request.client_id,
                request.client_secret.as_deref()
            ).await?;

            // Exchange code for tokens
            let response = server.exchange_authorization_code(
                &client,
                &code,
                request.code_verifier
            ).await?;

            Ok(Json(response))
        }
        "client_credentials" => {
            // Handle client credentials
            // ...
        }
        _ => Err(OAuth2Error::UnsupportedGrantType)
    }
}

let app = Router::new()
    .route("/oauth/token", post(token_endpoint))
    .with_state(Arc::new(server));
```

## Security Best Practices

### JWT Secret

- Use at least 256 bits (32 bytes) of entropy
- Generate using cryptographically secure random generator
- Store securely (environment variable, secrets manager)
- Rotate periodically

```rust
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose::STANDARD};

// Generate secure JWT secret
let mut secret_bytes = vec![0u8; 32];
rand::thread_rng().fill_bytes(&mut secret_bytes);
let jwt_secret = STANDARD.encode(&secret_bytes);
```

### PKCE

- **Always require PKCE for public clients**
- Recommend PKCE for all clients
- Use S256 challenge method (SHA-256)

### Token Storage

- **Access tokens**: Store client-side (memory, not localStorage)
- **Refresh tokens**: Store securely (httpOnly cookie, secure storage)
- **Never log tokens**

### HTTPS Only

- All OAuth2 endpoints must use HTTPS in production
- Redirect URIs must use HTTPS (except localhost for development)

### Client Secrets

- Hash client secrets using Argon2 (done automatically)
- Never expose secrets in responses
- Rotate secrets periodically

## Database Integration

### Implement ClientRepository Trait

```rust
use foundry_oauth_server::clients::{ClientRepository, Client};
use async_trait::async_trait;

pub struct DatabaseClientRepository {
    pool: sqlx::PgPool,
}

#[async_trait]
impl ClientRepository for DatabaseClientRepository {
    async fn find(&self, client_id: Uuid) -> OAuth2Result<Option<Client>> {
        let client = sqlx::query_as::<_, Client>(
            "SELECT * FROM oauth_clients WHERE id = $1"
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(client)
    }

    async fn find_by_credentials(
        &self,
        client_id: Uuid,
        client_secret: &str,
    ) -> OAuth2Result<Option<Client>> {
        // Implement with secure password verification
        // ...
    }

    // Implement other methods...
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use foundry_oauth_server::clients::InMemoryClientRepository;

    #[tokio::test]
    async fn test_authorization_code_flow() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();
        let server = OAuth2Server::new(config, repo);

        // Create client
        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()]
        );

        server.client_repository()
            .store(client.clone())
            .await
            .unwrap();

        // Test authorization code creation
        let code = server.create_authorization_code(
            &client,
            Uuid::new_v4(),
            "http://localhost/callback".to_string(),
            vec!["read".to_string()],
            None,
            None,
        ).unwrap();

        // Test code exchange
        let response = server.exchange_authorization_code(
            &client,
            &code,
            None,
        ).await.unwrap();

        assert!(!response.access_token.is_empty());
        assert!(response.refresh_token.is_some());
    }
}
```

## Production Deployment Checklist

- [ ] Use strong JWT secret (256+ bits)
- [ ] Enable PKCE for all clients
- [ ] Use HTTPS for all endpoints
- [ ] Implement proper client repository (database)
- [ ] Set appropriate token lifetimes
- [ ] Implement token storage (database)
- [ ] Add rate limiting on token endpoints
- [ ] Monitor for suspicious activity
- [ ] Implement token revocation
- [ ] Regular security audits
- [ ] Keep dependencies updated

## Examples

See [EXAMPLES.md](EXAMPLES.md) for more comprehensive examples including:
- Complete authorization server setup
- Integration with different web frameworks
- Database implementation examples
- Testing strategies

## Security

See [SECURITY.md](SECURITY.md) for detailed security considerations and best practices.

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please see our contributing guidelines.
