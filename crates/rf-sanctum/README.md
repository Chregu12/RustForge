# rf-sanctum

Laravel Sanctum-style API token authentication for RustForge with per-token abilities.

## Features

### Core Features
- **Personal Access Tokens (PAT)** with SHA-256 hashing
- **Token Abilities/Scopes**: Fine-grained permissions per token
- **Token Expiration**: Optional automatic expiration
- **Last Used Tracking**: Track when tokens were last used
- **SPA Cookie Authentication**: CSRF-protected authentication for SPAs
- **Database Persistence**: Full SeaORM integration with PostgreSQL/SQLite
- **Middleware Support**: Easy-to-use middleware for ability checking

### Extended Features (Laravel Sanctum Complete)
- **Token Hashing**: SHA-256 secure token storage
- **Device Tracking**: User agent and IP address tracking
- **Token Pruning**: Clean up expired and old tokens
- **SanctumGuard**: Unified authentication for API and SPA
- **Transient Tokens**: In-memory tokens for testing
- **Token Statistics**: Analytics on token usage
- **Configuration**: Flexible configuration system

## Installation

```toml
[dependencies]
rf-sanctum = "0.1"
sea-orm = "1.1"
```

## Quick Start

### 1. Run Migrations

```sql
-- See migrations/create_personal_access_tokens.sql
CREATE TABLE personal_access_tokens (
    id BIGSERIAL PRIMARY KEY,
    tokenable_type VARCHAR(255) NOT NULL,
    tokenable_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    token VARCHAR(64) UNIQUE NOT NULL,
    abilities JSON NOT NULL DEFAULT '[]',
    last_used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
```

### 2. Implement Traits for Your User Model

```rust
use rf_sanctum::{Tokenable, LoadFromToken, SanctumError};
use sea_orm::DatabaseConnection;
use async_trait::async_trait;

#[derive(Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
}

// Allow users to create tokens
#[async_trait]
impl Tokenable for User {
    fn tokenable_type() -> &'static str {
        "User"
    }

    fn tokenable_id(&self) -> i64 {
        self.id
    }
}

// Load user from token
#[async_trait]
impl LoadFromToken for User {
    async fn load_from_token(
        tokenable_id: i64,
        db: &DatabaseConnection,
    ) -> Result<Self, SanctumError> {
        // Load user from database using tokenable_id
        // ...
    }
}
```

### 3. Create Tokens

```rust
use rf_sanctum::Tokenable;

// Create a token with abilities
let new_token = user
    .create_token(
        "mobile-app",
        vec!["read:posts", "write:posts"],
        None, // No expiration
        &db,
    )
    .await?;

// Give this token to the user (only shown once!)
println!("Token: {}", new_token.access_token);

// Create token with expiration
let new_token = user
    .create_token_with_expiry(
        "temporary-token",
        vec!["read:posts"],
        24, // Expires in 24 hours
        &db,
    )
    .await?;
```

### 4. Protect Routes

```rust
use axum::{routing::get, Router};
use rf_sanctum::SanctumAuth;

async fn protected_route(
    SanctumAuth(user, token): SanctumAuth<User>,
) -> String {
    format!("Hello, {}! Your token has abilities: {:?}",
        user.name,
        token.abilities
    )
}

let app = Router::new()
    .route("/api/user", get(protected_route));
```

### 5. Check Abilities

```rust
async fn admin_route(
    SanctumAuth(user, token): SanctumAuth<User>,
) -> Result<String, SanctumError> {
    if !token.can("admin") {
        return Err(SanctumError::InsufficientPermissions(
            "admin ability required".to_string()
        ));
    }

    Ok(format!("Welcome, admin {}!", user.name))
}
```

### 6. Use Ability Middleware

```rust
use rf_sanctum::middleware::require_abilities;

let app = Router::new()
    .route("/api/admin", get(admin_handler))
    .layer(require_abilities!(["admin"]));

// Require ANY of multiple abilities
use rf_sanctum::require_any_ability;

let app = Router::new()
    .route("/api/posts", get(posts_handler))
    .layer(require_any_ability!(["read:posts", "admin"]));
```

## SPA Authentication

For single-page applications, use cookie-based CSRF protection:

```rust
use rf_sanctum::spa::sanctum_csrf_cookie;

let app = Router::new()
    .route("/sanctum/csrf-cookie", get(sanctum_csrf_cookie));
```

Client-side:

```javascript
// 1. Get CSRF cookie
await fetch('/sanctum/csrf-cookie', { credentials: 'include' });

// 2. Make authenticated requests
await fetch('/api/user', {
    credentials: 'include',
    headers: {
        'X-XSRF-TOKEN': getCookie('XSRF-TOKEN'),
    },
});
```

## Token Management

```rust
// List all tokens for a user
let tokens = user.tokens(&db).await?;

// Revoke a specific token
user.revoke_token(token_id, &db).await?;

// Revoke all tokens
user.revoke_all_tokens(&db).await?;

// Clean up expired tokens (run periodically)
let repo = TokenRepository::new(&db);
let deleted_count = repo.cleanup_expired().await?;

// Prune old tokens
let deleted = repo.prune_tokens_older_than(90).await?; // Delete tokens older than 90 days
let deleted = repo.prune_unused_tokens(30).await?; // Delete tokens not used in 30 days

// Get token statistics
let stats = repo.get_token_stats("User", user.id).await?;
println!("Total: {}, Active: {}, Expired: {}", stats.total, stats.active, stats.expired);
```

## Advanced Usage

### Device Tracking

Track user agents and IP addresses:

```rust
// Create token with device info
let new_token = user.create_token_with_device(
    "mobile-app",
    vec!["*"],
    None,
    Some("Mozilla/5.0 (iPhone...)".to_string()), // User agent
    Some("192.168.1.1".to_string()),              // IP address
    &db
).await?;

// Check device info
if let Some(device) = token.device_name() {
    println!("Device: {}", device); // "Mobile Device", "Desktop", "Tablet"
}

if let Some(browser) = token.browser_name() {
    println!("Browser: {}", browser); // "Chrome", "Firefox", "Safari", etc.
}

// Check if recently used
if token.is_recently_used(10) {
    println!("Token was used in the last 10 minutes");
}
```

### Sanctum Guard (Unified Authentication)

Use `SanctumGuard` for both API token and SPA cookie authentication:

```rust
use rf_sanctum::SanctumGuard;

async fn protected(guard: SanctumGuard<User>) -> String {
    // Access the authenticated user
    let user = guard.user;

    // Check authentication method
    if guard.is_token_auth() {
        // Authenticated via Bearer token
        if let Some(token) = guard.current_access_token() {
            format!("Token: {}", token.name)
        }
    } else if guard.is_cookie_auth() {
        // Authenticated via session/cookie
        format!("Session user: {}", user.name)
    }
}
```

### Transient Tokens (Testing)

In-memory tokens for testing without database:

```rust
use rf_sanctum::{TransientTokenStore, TransientTokenBuilder};

let store = TransientTokenStore::new();

// Create transient token
let (plain, token) = TransientTokenBuilder::new("User", 1, "test-token")
    .with_abilities(vec!["read:posts".to_string()])
    .with_expiration(Utc::now() + Duration::hours(1))
    .build();

store.store(token)?;

// Find and use token
let found = store.find(&hashed_token)?;
```

### Configuration

Customize Sanctum behavior:

```rust
use rf_sanctum::SanctumConfig;
use chrono::Duration;

let config = SanctumConfig::new()
    .with_prefix("myapp_")
    .with_expiration(Duration::days(7))
    .with_prune_after_days(30)
    .with_device_tracking(true)
    .add_stateful_domain("app.example.com");

// Store in request extensions for guard to use
// parts.extensions.insert(config);
```

### Wildcard Abilities

```rust
// Create token with all abilities
let token = user.create_token("admin-app", vec!["*"], None, &db).await?;

// This token can do anything
assert!(token.token.can("read:posts"));
assert!(token.token.can("delete:everything"));
```

### Pattern Matching

```rust
// Token with "posts:*" can do any post operation
if token.can("posts:read") || token.can("posts:write") {
    // Handle posts
}

// Check multiple abilities
if token.can_all(&["read:posts", "write:posts"]) {
    // User can both read and write
}

if token.can_any(&["read:posts", "admin"]) {
    // User can either read posts or is an admin
}
```

## Security Best Practices

1. **Store tokens securely**: Never log or expose plaintext tokens after creation
2. **Use HTTPS**: Always use HTTPS in production to prevent token interception
3. **Set expiration**: Use token expiration for sensitive operations
4. **Rotate tokens**: Implement token rotation for long-lived applications
5. **Audit tokens**: Regularly check `last_used_at` to detect unused tokens
6. **Revoke on logout**: Always revoke tokens when users log out
7. **Limit abilities**: Grant minimum required abilities per token

## Differences from Laravel Sanctum

- Uses SeaORM instead of Eloquent
- Abilities stored as JSON array instead of comma-separated string
- Async/await throughout
- Type-safe ability checking
- No built-in rate limiting (use separate middleware)

## Complete Example

See `examples/full_example.rs` for a complete working example with:
- Token creation API
- Token listing
- Token revocation
- Protected routes
- Ability-based authorization
- SPA CSRF protection

## License

MIT
