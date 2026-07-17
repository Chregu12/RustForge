# rf-socialite

OAuth2 and social login for RustForge - Laravel Socialite equivalent for Rust.

## Features

- **Multiple OAuth2 Providers**: GitHub, Google, Facebook, Twitter, and custom providers
- **PKCE Support**: Enhanced security with Proof Key for Code Exchange
- **State Management**: Built-in CSRF protection
- **Account Linking**: Link social accounts to existing users
- **Token Refresh**: Automatic token refresh support
- **Type-Safe**: Full type safety with comprehensive error handling
- **Laravel-Compatible API**: Similar API to Laravel Socialite

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-socialite = "0.1"
```

## Quick Start

### Basic Usage

```rust
use rf_socialite::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a driver
    let driver = Socialite::driver(Provider::GitHub)
        .client_id("your-client-id")
        .client_secret("your-client-secret")
        .redirect_url("http://localhost:8000/auth/github/callback")
        .build()?;

    // Get authorization URL
    let auth_url = driver.redirect()?;
    println!("Redirect user to: {}", auth_url);

    // After callback, exchange code for user
    let user = driver.user_from_code("authorization-code").await?;
    println!("Logged in as: {}", user.name);

    Ok(())
}
```

### Using the Manager

```rust
use rf_socialite::*;

// Load configuration from environment
let manager = SocialiteManager::from_env();

// Generate state for CSRF protection
let state = manager.generate_state();

// Get driver with state
let mut driver = manager.github()?
    .state(state)
    .with_pkce()  // Enable PKCE
    .build()?;

// Redirect to provider
let auth_url = driver.redirect()?;
```

## Configuration

Set environment variables:

```bash
# GitHub
GITHUB_CLIENT_ID=your-github-client-id
GITHUB_CLIENT_SECRET=your-github-client-secret
GITHUB_REDIRECT_URI=http://localhost:8000/auth/github/callback

# Google
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
GOOGLE_REDIRECT_URI=http://localhost:8000/auth/google/callback

# Facebook
FACEBOOK_CLIENT_ID=your-facebook-app-id
FACEBOOK_CLIENT_SECRET=your-facebook-app-secret
FACEBOOK_REDIRECT_URI=http://localhost:8000/auth/facebook/callback
```

Or configure programmatically:

```rust
use rf_socialite::*;

let config = SocialiteConfig::new()
    .with_github(ProviderConfig::new(
        "github-client-id",
        "github-client-secret",
        "http://localhost:8000/auth/github/callback",
    ))
    .with_google(ProviderConfig::new(
        "google-client-id",
        "google-client-secret",
        "http://localhost:8000/auth/google/callback",
    ));

let manager = SocialiteManager::new(config);
```

## Supported Providers

### GitHub

```rust
let driver = manager.github()?.build()?;
```

Default scopes: `user:email`

### Google

```rust
let driver = manager.google()?.build()?;
```

Default scopes: `userinfo.email`, `userinfo.profile`

### Facebook

```rust
let driver = manager.facebook()?.build()?;
```

Default scopes: `email`, `public_profile`

### Twitter

```rust
let driver = manager.twitter()?.build()?;
```

### Custom Provider

```rust
use rf_socialite::providers::GenericProvider;

let custom = GenericProvider {
    name: "custom".to_string(),
    authorize_url: "https://custom.com/oauth/authorize".to_string(),
    token_url: "https://custom.com/oauth/token".to_string(),
    user_url: "https://custom.com/api/user".to_string(),
    scopes: vec!["read".to_string()],
};

let driver = Socialite::driver(Provider::Generic(custom)).build()?;
```

## PKCE (Proof Key for Code Exchange)

PKCE adds an extra layer of security, especially for public clients:

```rust
let mut driver = manager.github()?
    .with_pkce()
    .build()?;

let auth_url = driver.redirect()?;  // Includes code_challenge
let user = driver.user_from_code(code).await?;  // Sends code_verifier
```

## State Management

Protect against CSRF attacks with state tokens:

```rust
let manager = SocialiteManager::from_env();

// Generate state
let state = manager.generate_state();

// Use in OAuth flow
let mut driver = manager.github()?
    .state(state.clone())
    .build()?;

// Verify on callback
if !manager.verify_state(&received_state) {
    return Err("Invalid state");
}
```

## Account Linking

Link social accounts to existing users:

```rust
use rf_socialite::*;

async fn find_or_create_user(social_user: &User) -> Result<LocalUser> {
    // Check if social account exists
    if let Some(account) = SocialAccount::find_by_provider(
        &social_user.provider,
        &social_user.id
    ).await? {
        return LocalUser::find(account.user_id).await;
    }

    // Check if user with email exists
    if let Some(email) = &social_user.email {
        if let Some(user) = LocalUser::find_by_email(email).await? {
            // Link to existing user
            link_social_account(&user, social_user).await?;
            return Ok(user);
        }
    }

    // Create new user
    let user = LocalUser::create_from_social(social_user).await?;
    link_social_account(&user, social_user).await?;
    Ok(user)
}

async fn link_social_account(user: &LocalUser, social: &User) -> Result<()> {
    let account = SocialAccount::new(
        user.id,
        &social.provider,
        &social.id,
        &social.token,
    );
    account.save().await?;
    Ok(())
}
```

### Database Schema

Create a `social_accounts` table:

```sql
CREATE TABLE social_accounts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_provider_user UNIQUE (provider, provider_user_id)
);

CREATE INDEX idx_social_accounts_user_id ON social_accounts(user_id);
```

## Token Refresh

Automatically refresh expired tokens:

```rust
async fn get_valid_token(account: &SocialAccount) -> Result<String> {
    if !account.needs_refresh() {
        return Ok(account.access_token.clone());
    }

    if let Some(refresh_token) = &account.refresh_token {
        let manager = SocialiteManager::from_env();
        let driver = manager.driver(&account.provider)?.build()?;

        let new_token = driver.refresh_token(refresh_token).await?;
        account.update_token(&new_token.access_token).await?;

        return Ok(new_token.access_token);
    }

    Err(Error::TokenExpired)
}
```

## Web Framework Integration

### Axum Example

```rust
use axum::{Router, routing::get, extract::{Path, Query, State}};
use rf_socialite::*;

struct AppState {
    socialite: SocialiteManager,
}

fn create_routes() -> Router {
    Router::new()
        .route("/auth/:provider", get(auth_redirect))
        .route("/auth/:provider/callback", get(auth_callback))
        .with_state(AppState {
            socialite: SocialiteManager::from_env(),
        })
}

async fn auth_redirect(
    Path(provider): Path<String>,
    State(app): State<AppState>,
) -> Result<Redirect> {
    let url = routes::redirect_to_provider(&app.socialite, &provider, true)?;
    Ok(Redirect::to(&url))
}

async fn auth_callback(
    Path(provider): Path<String>,
    Query(params): Query<CallbackParams>,
    State(app): State<AppState>,
) -> Result<Redirect> {
    let user = routes::handle_callback(&app.socialite, &provider, params).await?;
    let local_user = find_or_create_user(&user).await?;
    create_session(&local_user);
    Ok(Redirect::to("/dashboard"))
}
```

## Laravel Socialite Compatibility

rf-socialite provides a similar API to Laravel Socialite:

| Laravel Socialite | rf-socialite |
|------------------|--------------|
| `Socialite::driver('github')->redirect()` | `manager.github()?.build()?.redirect()?` |
| `Socialite::driver('github')->user()` | `driver.user_from_code(code).await?` |
| `$user->getId()` | `user.id` |
| `$user->getName()` | `user.name` |
| `$user->getEmail()` | `user.email` |
| `$user->getAvatar()` | `user.avatar` |

## Security Best Practices

1. **Always use HTTPS** in production for redirect URIs
2. **Enable PKCE** for public clients (SPAs, mobile apps)
3. **Verify state tokens** to prevent CSRF attacks
4. **Encrypt tokens** in your database
5. **Use short-lived states** (default: 10 minutes)
6. **Validate redirect URIs** match configured values
7. **Rate limit** OAuth callbacks

## Examples

See the `examples/` directory for more:

- `basic_oauth.rs` - Basic OAuth flow
- `web_integration.rs` - Web framework integration

Run examples:

```bash
cargo run --example basic_oauth
cargo run --example web_integration
```

## Testing

Run tests:

```bash
cargo test --package rf-socialite
```

## Contributing

Contributions welcome! Please check the [contributing guidelines](../../CONTRIBUTING.md).

## License

Licensed under MIT or Apache-2.0.

## Related

- [rf-auth](../rf-auth) - Authentication system
- [rf-session](../rf-session) - Session management
- [Laravel Socialite](https://laravel.com/docs/socialite) - The PHP inspiration
