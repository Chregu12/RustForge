# Authentication Code Snippets

Common authentication patterns and solutions for RustForge.

---

## User Registration

```rust
use rf_http::{Request, Response};
use rf_hashing::Hash;
use rf_validation::{Required, Email, MinLength, Confirmed};
use crate::models::User;

pub async fn register(req: Request) -> Response {
    // Validate input
    let validated = req.validate(|v| {
        v.rule("name", vec![Required, MinLength(3)])
         .rule("email", vec![Required, Email, Unique("users", "email")])
         .rule("password", vec![Required, MinLength(8), Confirmed])
    }).await?;

    // Hash password
    let hashed = Hash::make(&validated.get::<String>("password"))?;

    // Create user
    let user = User::create(req.db(), UserData {
        name: validated.get("name"),
        email: validated.get("email"),
        password: hashed,
    }).await?;

    // Log user in
    req.auth().login(&user).await?;

    Response::redirect("/dashboard")
}
```

---

## User Login

```rust
pub async fn login(req: Request) -> Response {
    // Validate credentials
    let validated = req.validate(|v| {
        v.rule("email", vec![Required, Email])
         .rule("password", vec![Required])
    }).await?;

    // Find user by email
    let user = User::where_eq("email", validated.get::<String>("email"), req.db())
        .await?
        .first()
        .ok_or(AuthError::InvalidCredentials)?;

    // Verify password
    if !Hash::check(&validated.get::<String>("password"), &user.password)? {
        return Err(AuthError::InvalidCredentials.into());
    }

    // Log user in
    req.auth().login(&user).await?;

    // Redirect to intended page or dashboard
    let intended = req.session().get("url.intended").unwrap_or("/dashboard");
    Response::redirect(intended)
}
```

---

## User Logout

```rust
pub async fn logout(req: Request) -> Response {
    req.auth().logout().await?;
    Response::redirect("/login")
}
```

---

## Password Reset Request

```rust
use rf_mail::Mail;
use crate::mail::PasswordResetMail;

pub async fn forgot_password(req: Request) -> Response {
    let validated = req.validate(|v| {
        v.rule("email", vec![Required, Email])
    }).await?;

    let email = validated.get::<String>("email");

    // Find user
    if let Ok(user) = User::where_eq("email", &email, req.db()).await?.first() {
        // Generate reset token
        let token = user.generate_password_reset_token().await?;

        // Send email
        Mail::to(&user.email)
            .send(PasswordResetMail::new(&user, &token))
            .await?;
    }

    // Always show success (security: don't reveal if email exists)
    Response::ok()
        .with_flash("status", "Password reset link sent!")
        .redirect("/login")
}
```

---

## Password Reset

```rust
pub async fn reset_password(req: Request) -> Response {
    let validated = req.validate(|v| {
        v.rule("token", vec![Required])
         .rule("email", vec![Required, Email])
         .rule("password", vec![Required, MinLength(8), Confirmed])
    }).await?;

    // Find user
    let user = User::where_eq("email", validated.get::<String>("email"), req.db())
        .await?
        .first()
        .ok_or(AuthError::InvalidToken)?;

    // Verify token
    if !user.verify_password_reset_token(&validated.get::<String>("token"))? {
        return Err(AuthError::InvalidToken.into());
    }

    // Update password
    user.update_password(&Hash::make(&validated.get::<String>("password"))?)
        .await?;

    // Invalidate all sessions
    user.invalidate_sessions().await?;

    Response::redirect("/login")
        .with_flash("status", "Password reset successful!")
}
```

---

## Check if Authenticated

```rust
pub async fn dashboard(req: Request) -> Response {
    // Option 1: Using middleware (recommended)
    // Apply auth middleware to route, user is always available

    let user = req.user()?; // Safe to unwrap, middleware ensures user exists

    View::make("dashboard").with("user", user).render()
}

// Option 2: Manual check
pub async fn profile(req: Request) -> Response {
    match req.auth().user().await {
        Some(user) => View::make("profile").with("user", user).render(),
        None => Response::redirect("/login"),
    }
}

// Option 3: Guest check
pub async fn home(req: Request) -> Response {
    if req.auth().guest().await {
        View::make("welcome").render()
    } else {
        Response::redirect("/dashboard")
    }
}
```

---

## Remember Me

```rust
pub async fn login_with_remember(req: Request) -> Response {
    let validated = req.validate(|v| {
        v.rule("email", vec![Required, Email])
         .rule("password", vec![Required])
         .rule("remember", vec![]) // Optional checkbox
    }).await?;

    let user = authenticate_user(&validated).await?;

    // Check if "remember me" was checked
    let remember = validated.get::<bool>("remember").unwrap_or(false);

    // Login with or without remember
    if remember {
        req.auth().login_remember(&user).await?;
    } else {
        req.auth().login(&user).await?;
    }

    Response::redirect("/dashboard")
}
```

---

## Email Verification

```rust
use rf_mail::Mail;
use crate::mail::VerifyEmailMail;

// Send verification email
pub async fn send_verification(user: &User) -> Result<()> {
    let token = user.generate_verification_token().await?;

    Mail::to(&user.email)
        .send(VerifyEmailMail::new(user, &token))
        .await?;

    Ok(())
}

// Verify email endpoint
pub async fn verify_email(req: Request) -> Response {
    let token = req.param("token")?;
    let email = req.query("email")?;

    let user = User::where_eq("email", email, req.db())
        .await?
        .first()
        .ok_or(AuthError::InvalidToken)?;

    if !user.verify_email_token(token)? {
        return Err(AuthError::InvalidToken.into());
    }

    user.mark_email_as_verified().await?;

    Response::redirect("/dashboard")
        .with_flash("status", "Email verified successfully!")
}
```

---

## Two-Factor Authentication

```rust
use totp_rs::{TOTP, Algorithm};
use qrcode::QrCode;

// Enable 2FA
pub async fn enable_2fa(req: Request) -> Response {
    let user = req.user()?;

    // Generate secret
    let secret = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("RustForge".to_string()),
        user.email.clone(),
    )?;

    // Store secret
    user.update_2fa_secret(secret.get_secret_base32()).await?;

    // Generate QR code
    let qr_url = secret.get_url();
    let qr_code = QrCode::new(&qr_url)?;
    let qr_image = qr_code.render::<Luma<u8>>().build();

    View::make("auth.setup-2fa")
        .with("qr_code", qr_image)
        .with("secret", secret.get_secret_base32())
        .render()
}

// Verify 2FA code
pub async fn verify_2fa(req: Request) -> Response {
    let user = req.session().get::<User>("2fa_user")?;
    let code = req.input::<String>("code")?;

    let secret = user.get_2fa_secret()?;
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.as_bytes(),
        Some("RustForge".to_string()),
        user.email.clone(),
    )?;

    if totp.check_current(&code)? {
        req.auth().login(&user).await?;
        req.session().remove("2fa_user");
        Response::redirect("/dashboard")
    } else {
        Response::back().with_error("Invalid 2FA code")
    }
}
```

---

## API Token Authentication

```rust
use uuid::Uuid;
use sha2::{Sha256, Digest};

// Generate API token
pub async fn create_token(req: Request) -> Response {
    let user = req.user()?;

    let validated = req.validate(|v| {
        v.rule("name", vec![Required, MaxLength(255)])
    }).await?;

    // Generate token
    let token = Uuid::new_v4().to_string();
    let hashed = Hash::make(&token)?;

    // Store hashed token
    let api_token = ApiToken::create(req.db(), ApiTokenData {
        user_id: user.id,
        name: validated.get("name"),
        token: hashed,
        abilities: vec!["*"],
        expires_at: None,
    }).await?;

    // Return plain token ONCE (never shown again)
    Response::json(json!({
        "token": token,
        "name": api_token.name,
    }))
}

// Verify API token middleware
pub async fn verify_token(req: Request, next: Next) -> Response {
    let auth_header = req.header("Authorization")?;
    let token = auth_header.strip_prefix("Bearer ")?;

    // Find token
    let hashed = Hash::make(token)?;
    let api_token = ApiToken::where_eq("token", hashed, req.db())
        .await?
        .first()
        .ok_or(AuthError::InvalidToken)?;

    // Check expiration
    if let Some(expires_at) = api_token.expires_at {
        if expires_at < Utc::now() {
            return Err(AuthError::TokenExpired.into());
        }
    }

    // Load user
    let user = User::find(api_token.user_id, req.db()).await?;
    req.set_user(user);

    next.run(req).await
}
```

---

## Social Authentication (OAuth)

```rust
use oauth2::{AuthorizationCode, TokenResponse};

// Redirect to provider
pub async fn redirect_to_provider(req: Request) -> Response {
    let provider = req.param("provider")?; // github, google, etc.

    let oauth_client = get_oauth_client(provider)?;

    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        .url();

    // Store CSRF token in session
    req.session().put("oauth_csrf", csrf_token.secret());

    Response::redirect(auth_url.as_str())
}

// Handle callback
pub async fn handle_callback(req: Request) -> Response {
    let provider = req.param("provider")?;
    let code = req.query("code")?;
    let state = req.query("state")?;

    // Verify CSRF token
    let stored_state = req.session().get::<String>("oauth_csrf")?;
    if state != stored_state {
        return Err(AuthError::InvalidCsrf.into());
    }

    let oauth_client = get_oauth_client(provider)?;

    // Exchange code for token
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await?;

    // Get user info from provider
    let user_info = get_provider_user_info(provider, token.access_token()).await?;

    // Find or create user
    let user = User::find_or_create_from_oauth(provider, &user_info, req.db()).await?;

    // Log user in
    req.auth().login(&user).await?;

    Response::redirect("/dashboard")
}
```

---

## Rate Limiting Login Attempts

```rust
use rf_cache::Cache;

pub async fn login_with_rate_limit(req: Request) -> Response {
    let email = req.input::<String>("email")?;
    let ip = req.ip();

    // Check rate limit
    let cache_key = format!("login_attempts:{}:{}", email, ip);
    let attempts = Cache::get::<u32>(&cache_key).await.unwrap_or(0);

    if attempts >= 5 {
        return Err(AuthError::TooManyAttempts.into());
    }

    // Try to authenticate
    match authenticate_user(&email, req.input("password")?).await {
        Ok(user) => {
            // Clear attempts on success
            Cache::forget(&cache_key).await?;
            req.auth().login(&user).await?;
            Response::redirect("/dashboard")
        }
        Err(_) => {
            // Increment attempts
            Cache::put(&cache_key, attempts + 1, 900).await?; // 15 min TTL
            Err(AuthError::InvalidCredentials.into())
        }
    }
}
```

---

## Auth Middleware

```rust
use rf_http::{Request, Response, Next, middleware::Middleware};

pub struct AuthMiddleware;

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Response {
        // Check if user is authenticated
        if let Some(user) = req.auth().user().await {
            req.set_user(user);
            next.run(req).await
        } else {
            // Store intended URL
            req.session().put("url.intended", req.url());

            Response::redirect("/login")
        }
    }
}

// Guest middleware (opposite of auth)
pub struct GuestMiddleware;

#[async_trait]
impl Middleware for GuestMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Response {
        if req.auth().check().await {
            Response::redirect("/dashboard")
        } else {
            next.run(req).await
        }
    }
}
```

---

These snippets cover the most common authentication patterns in RustForge. Mix and match as needed for your application!
