# OAuth Setup Guide

## Overview

Foundry provides OAuth 2.0 integration for social authentication with Google, GitHub, Facebook, and other providers.

## Quick Start

```rust
use foundry_oauth::{OAuthClient, GoogleProvider, GithubProvider};

// Create OAuth client
let mut oauth_client = OAuthClient::new();

// Register providers
oauth_client.register_provider(Box::new(
    GoogleProvider::new(
        env::var("GOOGLE_CLIENT_ID")?,
        env::var("GOOGLE_CLIENT_SECRET")?,
        "http://localhost:8000/auth/google/callback".to_string()
    )
));

oauth_client.register_provider(Box::new(
    GithubProvider::new(
        env::var("GITHUB_CLIENT_ID")?,
        env::var("GITHUB_CLIENT_SECRET")?,
        "http://localhost:8000/auth/github/callback".to_string()
    )
));
```

---

## Provider Setup

### Google OAuth

1. **Create OAuth Credentials**
   - Go to [Google Cloud Console](https://console.cloud.google.com/)
   - Create a new project or select existing
   - Navigate to APIs & Services > Credentials
   - Click "Create Credentials" > "OAuth 2.0 Client ID"
   - Select "Web application"
   - Add authorized redirect URIs:
     - Development: `http://localhost:8000/auth/google/callback`
     - Production: `https://yourdomain.com/auth/google/callback`

2. **Configuration**
   ```env
   GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
   GOOGLE_CLIENT_SECRET=your-client-secret
   GOOGLE_REDIRECT_URI=http://localhost:8000/auth/google/callback
   ```

3. **Scopes**
   - `openid`: User ID
   - `email`: Email address
   - `profile`: Name, avatar

### GitHub OAuth

1. **Create OAuth App**
   - Go to GitHub Settings > Developer settings > OAuth Apps
   - Click "New OAuth App"
   - Fill in:
     - Application name: Your app name
     - Homepage URL: `http://localhost:8000`
     - Authorization callback URL: `http://localhost:8000/auth/github/callback`

2. **Configuration**
   ```env
   GITHUB_CLIENT_ID=your-github-client-id
   GITHUB_CLIENT_SECRET=your-github-client-secret
   GITHUB_REDIRECT_URI=http://localhost:8000/auth/github/callback
   ```

3. **Scopes**
   - `user:email`: Email addresses

### Facebook OAuth

1. **Create Facebook App**
   - Go to [Facebook Developers](https://developers.facebook.com/)
   - Create a new app
   - Add "Facebook Login" product
   - Configure Valid OAuth Redirect URIs:
     - `http://localhost:8000/auth/facebook/callback`

2. **Configuration**
   ```env
   FACEBOOK_CLIENT_ID=your-facebook-app-id
   FACEBOOK_CLIENT_SECRET=your-facebook-app-secret
   FACEBOOK_REDIRECT_URI=http://localhost:8000/auth/facebook/callback
   ```

3. **Permissions**
   - `email`
   - `public_profile`

---

## Implementation

### Route Handlers

```rust
use axum::{Router, response::Redirect, extract::{Query, State}};
use foundry_oauth::OAuthClient;
use serde::Deserialize;

#[derive(Deserialize)]
struct AuthCallback {
    code: String,
    state: String,
}

// Redirect to provider
async fn oauth_redirect(
    State(oauth): State<Arc<OAuthClient>>,
    Path(provider): Path<String>,
) -> Result<Redirect, AppError> {
    let (auth_url, state) = oauth.get_authorize_url(&provider).await?;

    // Store state in session for validation
    // session.set("oauth_state", state);

    Ok(Redirect::to(&auth_url))
}

// Handle callback
async fn oauth_callback(
    State(oauth): State<Arc<OAuthClient>>,
    Path(provider): Path<String>,
    Query(params): Query<AuthCallback>,
) -> Result<Redirect, AppError> {
    // Validate state parameter (CSRF protection)
    // let stored_state = session.get("oauth_state")?;
    // if params.state != stored_state {
    //     return Err(AppError::InvalidState);
    // }

    // Exchange code for tokens
    let (user, tokens) = oauth
        .authenticate(&provider, &params.code, &params.state)
        .await?;

    // Find or create user
    let app_user = find_or_create_oauth_user(&user).await?;

    // Log in user
    session.login(app_user).await?;

    // Store tokens for later use
    session.set("oauth_access_token", tokens.access_token);
    if let Some(refresh_token) = tokens.refresh_token {
        session.set("oauth_refresh_token", refresh_token);
    }

    Ok(Redirect::to("/dashboard"))
}

// Setup routes
fn oauth_routes() -> Router {
    Router::new()
        .route("/auth/:provider", get(oauth_redirect))
        .route("/auth/:provider/callback", get(oauth_callback))
}
```

### User Management

```rust
use foundry_oauth::OAuthUser;

async fn find_or_create_oauth_user(oauth_user: &OAuthUser) -> Result<User, AppError> {
    // Try to find existing user by OAuth provider and ID
    if let Some(user) = User::find_by_oauth(
        &oauth_user.provider,
        &oauth_user.provider_id
    ).await? {
        return Ok(user);
    }

    // Try to find by email
    if let Some(email) = &oauth_user.email {
        if let Some(user) = User::find_by_email(email).await? {
            // Link OAuth account to existing user
            user.link_oauth_account(
                &oauth_user.provider,
                &oauth_user.provider_id
            ).await?;
            return Ok(user);
        }
    }

    // Create new user
    let user = User::create_from_oauth(oauth_user).await?;
    Ok(user)
}
```

### Database Schema

```sql
-- OAuth accounts table
CREATE TABLE oauth_accounts (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    provider_id VARCHAR(255) NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    expires_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(provider, provider_id)
);

-- Index for lookups
CREATE INDEX idx_oauth_accounts_user_id ON oauth_accounts(user_id);
CREATE INDEX idx_oauth_accounts_provider ON oauth_accounts(provider, provider_id);
```

---

## Security Best Practices

### 1. State Parameter Validation

The state parameter prevents CSRF attacks:

```rust
// State is automatically generated and validated
let (auth_url, state) = oauth.get_authorize_url("google").await?;

// Store state in session
session.set("oauth_state", state);

// Validate on callback
let stored_state = session.get("oauth_state")?;
if params.state != stored_state {
    return Err(OAuthError::InvalidState);
}
```

### 2. Use HTTPS in Production

```rust
let redirect_uri = if cfg!(debug_assertions) {
    "http://localhost:8000/auth/google/callback"
} else {
    "https://yourdomain.com/auth/google/callback"
};
```

### 3. Store Tokens Securely

```rust
// Encrypt sensitive tokens before storing
let encrypted_token = encrypt(&tokens.access_token)?;
session.set("oauth_access_token", encrypted_token);

// Or use dedicated secrets storage
vault.store(&format!("oauth_token:{}", user.id), &tokens.access_token)?;
```

### 4. Token Refresh

```rust
async fn refresh_oauth_token(
    oauth: &OAuthClient,
    provider: &str,
    refresh_token: &str,
) -> Result<OAuthTokens, AppError> {
    let new_tokens = oauth
        .refresh_token(provider, refresh_token)
        .await?;

    // Update stored tokens
    update_user_oauth_tokens(&new_tokens).await?;

    Ok(new_tokens)
}
```

### 5. Revoke Tokens on Logout

```rust
async fn logout(
    State(oauth): State<Arc<OAuthClient>>,
    session: Session,
) -> Result<Redirect, AppError> {
    // Revoke OAuth token
    if let Some(token) = session.get("oauth_access_token") {
        if let Some(provider) = session.get("oauth_provider") {
            let _ = oauth.revoke_token(&provider, &token).await;
        }
    }

    // Clear session
    session.destroy().await?;

    Ok(Redirect::to("/"))
}
```

---

## UI Integration

### Login Buttons

```html
<div class="oauth-buttons">
    <a href="/auth/google" class="btn btn-google">
        <img src="/icons/google.svg" alt="Google">
        Sign in with Google
    </a>

    <a href="/auth/github" class="btn btn-github">
        <img src="/icons/github.svg" alt="GitHub">
        Sign in with GitHub
    </a>

    <a href="/auth/facebook" class="btn btn-facebook">
        <img src="/icons/facebook.svg" alt="Facebook">
        Sign in with Facebook
    </a>
</div>
```

### Account Linking

```html
<!-- In user settings -->
<div class="oauth-accounts">
    <h3>Connected Accounts</h3>

    {% if user.google_connected %}
        <div class="connected">
            ✓ Google connected
            <button onclick="disconnect('google')">Disconnect</button>
        </div>
    {% else %}
        <a href="/settings/oauth/connect/google">Connect Google</a>
    {% endif %}

    <!-- Similar for other providers -->
</div>
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("Invalid state parameter")]
    InvalidState,

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("User denied access")]
    AccessDenied,

    #[error("Token expired")]
    TokenExpired,
}

// Handle errors in route
async fn oauth_callback(...) -> Result<Redirect, OAuthError> {
    match oauth.authenticate(&provider, &code, &state).await {
        Ok((user, tokens)) => {
            // Success
        }
        Err(OAuthError::InvalidState) => {
            // CSRF attack detected
            return Err(OAuthError::InvalidState);
        }
        Err(OAuthError::AccessDenied) => {
            // User clicked "Cancel" on OAuth screen
            return Ok(Redirect::to("/login?error=cancelled"));
        }
        Err(e) => {
            // Other errors
            log::error!("OAuth error: {:?}", e);
            return Ok(Redirect::to("/login?error=oauth_failed"));
        }
    }
}
```

---

## Testing

### Mock Provider

```rust
struct MockOAuthProvider;

impl OAuthProvider for MockOAuthProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn authorize_url(&self, state: &str) -> String {
        format!("http://mock.com/auth?state={}", state)
    }

    async fn exchange_code(&self, _code: &str) -> Result<OAuthTokens> {
        Ok(OAuthTokens {
            access_token: "mock_token".to_string(),
            refresh_token: None,
            expires_in: Some(3600),
            token_type: "Bearer".to_string(),
        })
    }

    async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
        Ok(OAuthUser {
            provider: "mock".to_string(),
            provider_id: "123".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            avatar: None,
        })
    }
}
```

### Integration Test

```rust
#[tokio::test]
async fn test_oauth_flow() {
    let mut oauth = OAuthClient::new();
    oauth.register_provider(Box::new(MockOAuthProvider));

    // Get auth URL
    let (url, state) = oauth.get_authorize_url("mock").await.unwrap();
    assert!(url.contains("http://mock.com/auth"));

    // Simulate callback
    let (user, tokens) = oauth
        .authenticate("mock", "code123", &state)
        .await
        .unwrap();

    assert_eq!(user.email, Some("test@example.com".to_string()));
    assert_eq!(tokens.access_token, "mock_token");
}
```

---

## Common Issues

### Redirect URI Mismatch

**Error:** `redirect_uri_mismatch`

**Solution:** Ensure redirect URI in code matches exactly what's registered with provider:
```rust
// Must match exactly (including trailing slash)
"http://localhost:8000/auth/google/callback"  // ✓
"http://localhost:8000/auth/google/callback/" // ✗
```

### Invalid State

**Error:** `Invalid state parameter`

**Solution:** Check session storage is working correctly and state hasn't expired.

### Token Expired

**Error:** `Token expired`

**Solution:** Implement token refresh:
```rust
if tokens.is_expired() {
    tokens = oauth.refresh_token(&provider, &refresh_token).await?;
}
```

---

## Related Documentation

- [Authentication](./AUTHENTICATION.md)
- [Session Management](./SESSION_MANAGEMENT.md)
- [Security Best Practices](./SECURITY_BEST_PRACTICES.md)
