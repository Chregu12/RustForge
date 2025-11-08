# Security Best Practices

## Overview

This guide covers essential security practices for building secure applications with the Foundry framework.

---

## 1. Authentication & Passwords

### Password Requirements

✅ **Enforce strong passwords:**
```rust
fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 12 {
        return Err(ValidationError::TooShort);
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_uppercase || !has_lowercase || !has_digit || !has_special {
        return Err(ValidationError::TooWeak);
    }

    Ok(())
}
```

✅ **Use Argon2 or BCrypt for hashing:**
```rust
use foundry_application::auth::PasswordHash;

// Foundry uses Argon2 by default
let hash = PasswordHash::hash("user_password")?;
```

❌ **Never:**
- Store passwords in plaintext
- Use MD5 or SHA-1 for password hashing
- Use weak salts or no salts
- Log passwords

### Multi-Factor Authentication (MFA)

Implement MFA for sensitive accounts:

```rust
// TODO: MFA implementation example
// Consider TOTP (Time-based One-Time Password)
```

---

## 2. Session Management

### Secure Session Configuration

```rust
use foundry_application::auth::{SessionStore, Session};
use std::time::Duration;

// Production session configuration
let session_store = RedisSessionStore::new("redis://localhost:6379")
    .ttl(Duration::from_secs(3600))        // 1 hour
    .cookie_name("__Host-session")         // Secure prefix
    .cookie_secure(true)                   // HTTPS only
    .cookie_http_only(true)                // No JavaScript access
    .cookie_same_site(SameSite::Strict);   // CSRF protection
```

### Session Security Best Practices

✅ **Do:**
- Use `HttpOnly` cookies
- Use `Secure` flag (HTTPS only)
- Set `SameSite=Strict` or `Lax`
- Implement session timeout
- Regenerate session ID on login
- Destroy session on logout

❌ **Don't:**
- Store sessions in localStorage
- Use predictable session IDs
- Keep sessions indefinitely
- Share sessions across subdomains unnecessarily

### Session Fixation Protection

```rust
async fn login(session: &mut Session, user: User) -> Result<()> {
    // Regenerate session ID to prevent fixation
    let old_id = session.id.clone();
    session.regenerate_id().await?;

    // Transfer old session data
    session.migrate_from(&old_id).await?;

    // Set user
    session.set_user(user).await?;

    Ok(())
}
```

---

## 3. CSRF Protection

### Enable CSRF Middleware

```rust
use foundry_application::middleware::csrf::CsrfMiddleware;

let csrf = CsrfMiddleware::new()
    .exempt("/api/*")        // API endpoints with token auth
    .exempt("/webhooks/*");  // External webhooks

app = app.layer(csrf.into_layer());
```

### In Forms

```html
<form method="POST" action="/posts">
    <input type="hidden" name="_csrf_token" value="{{ csrf_token }}">
    <!-- form fields -->
</form>
```

### In AJAX

```javascript
const token = document.querySelector('meta[name="csrf-token"]').content;

fetch('/api/posts', {
    method: 'POST',
    headers: {
        'X-CSRF-Token': token,
        'Content-Type': 'application/json'
    },
    body: JSON.stringify(data)
});
```

---

## 4. SQL Injection Prevention

### Use ORM (SeaORM)

✅ **Good - Parameterized queries:**
```rust
// SeaORM automatically uses parameterized queries
let user = User::find()
    .filter(user::Column::Email.eq(email))
    .one(&db)
    .await?;
```

❌ **Bad - Raw SQL with concatenation:**
```rust
// NEVER DO THIS
let query = format!("SELECT * FROM users WHERE email = '{}'", email);
db.execute(query).await?;
```

✅ **If using raw SQL, use parameters:**
```rust
let user = sqlx::query_as::<_, User>(
    "SELECT * FROM users WHERE email = $1"
)
.bind(email)
.fetch_one(&db)
.await?;
```

---

## 5. XSS Prevention

### Template Auto-Escaping

Ensure your template engine auto-escapes by default:

```rust
// With Tera templates
let mut tera = Tera::new("templates/**/*")?;
tera.autoescape_on(vec![".html", ".xml"]);  // Enable auto-escape
```

### Manual Escaping

```rust
use html_escape::encode_text;

let safe_output = encode_text(&user_input);
```

### Content Security Policy

```rust
use axum::response::Response;

fn add_security_headers(mut response: Response) -> Response {
    response.headers_mut().insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
            .parse()
            .unwrap()
    );
    response
}
```

---

## 6. Rate Limiting

### Implement Rate Limiting

```rust
use foundry_application::middleware::rate_limit::{RateLimitMiddleware, RateLimitConfig};

// Prevent brute force on login
let login_limiter = RateLimitMiddleware::in_memory(
    RateLimitConfig::per_ip(5)  // 5 attempts per minute
);

// API rate limiting
let api_limiter = RateLimitMiddleware::in_memory(
    RateLimitConfig::per_user(100)  // 100 requests per minute
);
```

---

## 7. Input Validation

### Validate All Input

```rust
use validator::{Validate, ValidationError};

#[derive(Validate)]
struct CreateUserRequest {
    #[validate(email)]
    email: String,

    #[validate(length(min = 12, max = 128))]
    password: String,

    #[validate(length(min = 1, max = 100))]
    name: String,
}

async fn create_user(
    Json(data): Json<CreateUserRequest>
) -> Result<Json<User>, AppError> {
    // Validate input
    data.validate()?;

    // Process...
}
```

### Sanitize Input

```rust
fn sanitize_html(input: &str) -> String {
    ammonia::clean(input)
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect()
}
```

---

## 8. File Upload Security

### Validate File Types

```rust
async fn upload_file(
    mut multipart: Multipart,
) -> Result<(), AppError> {
    while let Some(field) = multipart.next_field().await? {
        let content_type = field.content_type()
            .ok_or(AppError::InvalidFileType)?;

        // Whitelist allowed types
        match content_type.as_ref() {
            "image/jpeg" | "image/png" | "image/gif" => {
                // OK
            }
            _ => return Err(AppError::InvalidFileType),
        }

        // Validate file size
        const MAX_SIZE: usize = 5 * 1024 * 1024; // 5MB
        let data = field.bytes().await?;
        if data.len() > MAX_SIZE {
            return Err(AppError::FileTooLarge);
        }

        // Validate file signature (magic bytes)
        if !is_valid_image(&data) {
            return Err(AppError::InvalidFile);
        }
    }

    Ok(())
}
```

### Store Files Securely

```rust
// Generate random filename
use uuid::Uuid;

let filename = format!("{}.jpg", Uuid::new_v4());
let path = format!("uploads/{}", filename);

// Store outside webroot
std::fs::write(&path, data)?;

// Serve through controlled endpoint
app.route("/files/:id", get(serve_file));
```

---

## 9. API Security

### API Authentication

```rust
use foundry_application::auth::middleware::jwt_auth_middleware;

// Protect API routes with JWT
let api_routes = Router::new()
    .route("/api/posts", get(list_posts))
    .layer(axum::middleware::from_fn_with_state(
        jwt_service,
        jwt_auth_middleware
    ));
```

### API Rate Limiting

```rust
// Different tiers
let free_tier = RateLimitConfig::per_user(100);   // 100/min
let pro_tier = RateLimitConfig::per_user(1000);   // 1000/min
```

### CORS Configuration

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin("https://yourdomain.com".parse::<HeaderValue>()?)
    .allow_methods([Method::GET, Method::POST])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

app = app.layer(cors);
```

---

## 10. Error Handling

### Don't Leak Sensitive Information

❌ **Bad:**
```rust
// Exposes database details
Err(format!("Database error: {}", e))
```

✅ **Good:**
```rust
// Generic error to user
log::error!("Database error: {}", e);
Err("An error occurred. Please try again later.")
```

### Prevent User Enumeration

❌ **Bad:**
```rust
if !user_exists(email) {
    return Err("User not found");
}
if !password_valid(password) {
    return Err("Invalid password");
}
```

✅ **Good:**
```rust
// Same error for both cases
if !authenticate(email, password) {
    return Err("Invalid email or password");
}
```

---

## 11. Secrets Management

### Environment Variables

```env
# .env
DATABASE_URL=postgresql://user:pass@localhost/db
JWT_SECRET=your-very-long-random-secret-here
ENCRYPTION_KEY=another-long-random-key
```

### Never Commit Secrets

```gitignore
# .gitignore
.env
.env.*
!.env.example
secrets/
*.pem
*.key
```

### Validate Production Secrets

```rust
fn validate_production_config() -> Result<()> {
    if cfg!(not(debug_assertions)) {
        let jwt_secret = env::var("JWT_SECRET")?;

        // Ensure not using default/weak secrets
        if jwt_secret == "default_secret" || jwt_secret.len() < 32 {
            panic!("Production requires strong JWT_SECRET!");
        }
    }

    Ok(())
}
```

---

## 12. HTTPS/TLS

### Force HTTPS in Production

```rust
use axum::middleware::Next;
use axum::extract::Request;

async fn redirect_to_https(req: Request, next: Next) -> Response {
    if !cfg!(debug_assertions) {
        if req.headers().get("x-forwarded-proto") != Some(&HeaderValue::from_static("https")) {
            let host = req.headers().get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost");

            let redirect_url = format!("https://{}{}", host, req.uri());
            return Redirect::permanent(&redirect_url).into_response();
        }
    }

    next.run(req).await
}
```

### HSTS Header

```rust
response.headers_mut().insert(
    "Strict-Transport-Security",
    "max-age=31536000; includeSubDomains; preload"
        .parse()
        .unwrap()
);
```

---

## 13. Security Headers

```rust
fn add_security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();

    // Prevent XSS
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap()
    );

    // Prevent clickjacking
    headers.insert(
        "X-Frame-Options",
        "DENY".parse().unwrap()
    );

    // Prevent MIME sniffing
    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().unwrap()
    );

    // Referrer policy
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap()
    );

    // Content Security Policy
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
            .parse()
            .unwrap()
    );

    response
}
```

---

## 14. Dependency Security

### Regular Audits

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit

# Update dependencies
cargo update
```

### Automated Scanning

Add to CI/CD:

```yaml
# .github/workflows/security.yml
name: Security Audit
on: [push, pull_request]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 15. Logging & Monitoring

### Secure Logging

```rust
// Log security events
tracing::warn!(
    user_id = user.id,
    ip = client_ip,
    "Failed login attempt"
);

// Never log sensitive data
// ❌ log!("Password: {}", password);
// ✅ log!("Authentication failed");
```

### Monitor Security Events

- Failed login attempts
- Rate limit violations
- Authorization failures
- Unusual access patterns
- Token expiration events

---

## Security Checklist

Before deploying to production:

- [ ] All secrets rotated from defaults
- [ ] HTTPS enforced
- [ ] CSRF protection enabled
- [ ] Rate limiting configured
- [ ] Input validation implemented
- [ ] SQL injection prevention verified
- [ ] XSS prevention in templates
- [ ] Security headers configured
- [ ] Session security configured
- [ ] Password policy enforced
- [ ] Error messages don't leak info
- [ ] Logging configured (no sensitive data)
- [ ] Dependencies audited
- [ ] Authorization checks in place
- [ ] File upload validation
- [ ] API authentication working

---

## Related Documentation

- [CSRF Protection](./CSRF_PROTECTION.md)
- [Rate Limiting](./RATE_LIMITING.md)
- [Authorization](./AUTHORIZATION.md)
- [OAuth Setup](./OAUTH_SETUP.md)
