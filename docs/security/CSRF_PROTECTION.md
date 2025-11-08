# CSRF Protection Guide

## Overview

Cross-Site Request Forgery (CSRF) protection prevents malicious websites from making unauthorized requests on behalf of authenticated users.

## How It Works

CSRF protection in Foundry uses a token-based approach:
1. Server generates a unique token for each session
2. Token is embedded in forms or sent via headers
3. Server validates the token on state-changing requests (POST, PUT, DELETE, PATCH)

## Basic Usage

### 1. Enable CSRF Middleware

Add the CSRF middleware to your application:

```rust
use foundry_application::middleware::csrf::CsrfMiddleware;

let csrf = CsrfMiddleware::new()
    .exempt("/api/*")        // Exempt API endpoints
    .exempt("/webhooks/*");  // Exempt webhook endpoints

// Add to your Axum router
app = app.layer(axum::middleware::from_fn(move |req, next| {
    csrf.handle(req, next)
}));
```

### 2. Generate and Use Tokens

#### In Templates

```html
<form method="POST" action="/posts">
    <input type="hidden" name="_csrf_token" value="{{ csrf_token }}">
    <!-- form fields -->
</form>
```

#### With AJAX/JavaScript

```javascript
// Get token from meta tag or API
const csrfToken = document.querySelector('meta[name="csrf-token"]').content;

// Include in request headers
fetch('/api/posts', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
        'X-CSRF-Token': csrfToken
    },
    body: JSON.stringify({ title: 'New Post' })
});
```

### 3. Generate Tokens in Handlers

```rust
use foundry_application::middleware::csrf::CsrfMiddleware;
use axum::extract::State;

async fn show_form(
    State(csrf): State<Arc<CsrfMiddleware>>,
) -> Html<String> {
    let token = csrf.store().generate().await;

    Html(format!(
        r#"<form method="POST">
            <input type="hidden" name="_csrf_token" value="{}">
            <button type="submit">Submit</button>
        </form>"#,
        token
    ))
}
```

## Configuration

### Custom Error Messages

```rust
use foundry_application::middleware::csrf::CsrfConfig;

let config = CsrfConfig::new()
    .error_message("CSRF validation failed. Please refresh and try again.")
    .exempt("/api/*");

let csrf = CsrfMiddleware::with_config(config);
```

### One-Time Tokens

For extra security, enable one-time use tokens:

```rust
let config = CsrfConfig::new()
    .one_time_tokens(true);
```

**Note:** One-time tokens are removed after validation, preventing replay attacks but requiring a new token for each form submission.

## Exempting Routes

Some routes should be exempt from CSRF protection:

### API Endpoints
```rust
let csrf = CsrfMiddleware::new()
    .exempt("/api/*");  // All API routes
```

### Webhooks
```rust
let csrf = CsrfMiddleware::new()
    .exempt("/webhooks/stripe")
    .exempt("/webhooks/github");
```

### Public Endpoints
```rust
let csrf = CsrfMiddleware::new()
    .exempt("/public/*")
    .exempt("/health")
    .exempt("/metrics");
```

## Integration with SPA/React

For Single Page Applications:

### 1. Store token in meta tag
```html
<meta name="csrf-token" content="{{ csrf_token }}">
```

### 2. Configure Axios (or fetch)
```javascript
import axios from 'axios';

// Get token from meta tag
const token = document.querySelector('meta[name="csrf-token"]').content;

// Set default header for all requests
axios.defaults.headers.common['X-CSRF-Token'] = token;
```

### 3. Handle token rotation
```javascript
// After successful request, update token if server sends a new one
axios.interceptors.response.use(response => {
    const newToken = response.headers['x-csrf-token'];
    if (newToken) {
        document.querySelector('meta[name="csrf-token"]').content = newToken;
        axios.defaults.headers.common['X-CSRF-Token'] = newToken;
    }
    return response;
});
```

## Best Practices

### ✅ DO
- Enable CSRF protection for all state-changing operations
- Use unique tokens per session
- Validate tokens before processing requests
- Exempt only truly stateless endpoints (webhooks, public APIs)
- Use HTTPS to prevent token interception

### ❌ DON'T
- Disable CSRF protection globally
- Store tokens in URLs or GET parameters
- Reuse tokens across different users
- Exempt authenticated endpoints without good reason
- Send tokens in response bodies (use headers or hidden fields)

## Testing

### Test CSRF Protection

```rust
#[tokio::test]
async fn test_csrf_protection() {
    let app = create_test_app().await;

    // Request without token should fail
    let response = app
        .post("/posts")
        .json(&json!({ "title": "Test" }))
        .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Request with valid token should succeed
    let csrf_token = get_csrf_token(&app).await;
    let response = app
        .post("/posts")
        .header("X-CSRF-Token", csrf_token)
        .json(&json!({ "title": "Test" }))
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}
```

## Troubleshooting

### Token Validation Fails

**Problem:** All POST requests return 403 Forbidden

**Solutions:**
1. Verify token is being sent (check request headers/body)
2. Check token format (should be 32 characters)
3. Ensure middleware is properly configured
4. Check if route is accidentally exempt

### Token Not Persisting

**Problem:** Token changes between requests

**Solution:** Ensure session storage is properly configured:

```rust
// Use persistent session storage
let session_store = RedisSessionStore::new("redis://localhost");
```

### AJAX Requests Failing

**Problem:** JavaScript requests fail CSRF validation

**Solutions:**
1. Include token in `X-CSRF-Token` header
2. Verify token is correctly extracted from DOM
3. Check CORS configuration allows custom headers

## Security Considerations

1. **Token Length**: Use at least 32 random characters
2. **Token Storage**: Store tokens server-side, never client-side only
3. **HTTPS**: Always use HTTPS to prevent token interception
4. **SameSite Cookies**: Use `SameSite=Strict` for session cookies
5. **Token Rotation**: Consider rotating tokens periodically
6. **Rate Limiting**: Combine with rate limiting to prevent brute force

## Related Documentation

- [Session Management](./SESSION_MANAGEMENT.md)
- [Rate Limiting](./RATE_LIMITING.md)
- [Security Best Practices](./SECURITY_BEST_PRACTICES.md)
