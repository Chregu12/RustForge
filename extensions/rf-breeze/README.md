# rf-breeze

[![Crates.io](https://img.shields.io/crates/v/rf-breeze.svg)](https://crates.io/crates/rf-breeze)
[![Documentation](https://docs.rs/rf-breeze/badge.svg)](https://docs.rs/rf-breeze)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](README.md)

Laravel Breeze-equivalent authentication scaffolding for RustForge. Provides complete, ready-to-use authentication system with views, controllers, routes, and middleware.

## Features

- **Complete Authentication System**: Login, registration, password reset, and email verification
- **Blade-Compatible Views**: Beautiful, pre-built templates using rf-blade
- **Controller Generation**: Ready-to-use authentication controllers with TODOs for database integration
- **Route Setup**: Automatic route registration with protected and guest routes
- **Middleware Templates**: Auth, guest, verified, and role-based middleware
- **API Support**: Optional API authentication routes
- **Customizable**: Flexible installation options for different authentication flows

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-breeze = "0.1"
rf-auth = { version = "0.1", features = ["mail"] }
rf-blade = "0.1"
```

## Quick Start

```rust
use rf_breeze::{BreezeScaffold, InstallOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create scaffold
    let breeze = BreezeScaffold::new(".")?;

    // Install complete authentication system
    breeze.install(&InstallOptions {
        with_api: false,
        with_email_verification: true,
        with_password_reset: true,
        output_dir: None,
    }).await?;

    println!("Authentication scaffolding installed successfully!");

    Ok(())
}
```

## Installation Options

### Full Installation

Install everything: login, register, password reset, and email verification:

```rust
use rf_breeze::{BreezeScaffold, InstallOptions};

let breeze = BreezeScaffold::new(".")?;

breeze.install(&InstallOptions {
    with_api: false,
    with_email_verification: true,
    with_password_reset: true,
    output_dir: None,
}).await?;
```

### Minimal Installation

Install only login and registration:

```rust
breeze.install(&InstallOptions {
    with_api: false,
    with_email_verification: false,
    with_password_reset: false,
    output_dir: None,
}).await?;
```

### API Authentication

Include API authentication routes:

```rust
breeze.install(&InstallOptions {
    with_api: true,
    with_email_verification: true,
    with_password_reset: true,
    output_dir: None,
}).await?;
```

### Selective Installation

Install specific components:

```rust
// Install only views
breeze.install_views().await?;

// Install only controllers
breeze.install_controllers().await?;

// Install only routes
breeze.install_routes().await?;

// Install only middleware
breeze.install_middleware().await?;
```

## Generated Structure

After installation, you'll have:

```
project/
├── resources/
│   └── views/
│       ├── layouts/
│       │   └── app.blade.html
│       ├── auth/
│       │   ├── login.blade.html
│       │   ├── register.blade.html
│       │   ├── forgot-password.blade.html
│       │   ├── reset-password.blade.html
│       │   └── verify-email.blade.html
│       └── dashboard.blade.html
├── src/
│   ├── controllers/
│   │   ├── auth/
│   │   │   ├── mod.rs
│   │   │   ├── login.rs
│   │   │   ├── register.rs
│   │   │   ├── password_reset.rs
│   │   │   └── email_verification.rs
│   │   └── dashboard.rs
│   ├── routes/
│   │   ├── auth.rs
│   │   └── api.rs (if with_api is true)
│   └── middleware/
│       ├── mod.rs
│       ├── auth.rs
│       ├── guest.rs
│       ├── verified.rs
│       └── role.rs
```

## View Templates

All views use Tailwind CSS for styling and Blade syntax for templating:

### Login View
- Email and password fields
- Remember me checkbox
- Forgot password link
- Register link
- Error message display

### Register View
- Name, email, and password fields
- Password confirmation
- Validation error display
- Login link

### Forgot Password View
- Email input
- Success/error messages
- Back to login link

### Reset Password View
- Email and new password fields
- Password confirmation
- Token verification (hidden field)

### Email Verification View
- Verification notice
- Resend verification email button
- Logout link

### Dashboard View
- Welcome message
- User name display
- Logout button

## Controller Templates

All controllers include TODO comments for database integration:

```rust
// Example: Login Controller
pub async fn login(
    Extension(hasher): Extension<Arc<PasswordHasher>>,
    Extension(jwt): Extension<Arc<JwtManager>>,
    Form(request): Form<LoginRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // TODO: Fetch user from database
    // TODO: Verify password
    // TODO: Generate JWT token
    Ok(Redirect::to("/dashboard"))
}
```

## Route Templates

Routes are organized into guest and protected sections:

```rust
// Guest routes (no authentication required)
.route("/login", get(auth::login::show_login_form))
.route("/login", post(auth::login::login))
.route("/register", get(auth::register::show_register_form))
.route("/register", post(auth::register::register))

// Protected routes (authentication required)
.route("/dashboard", get(dashboard::show_dashboard))
.route("/logout", post(auth::login::logout))
.layer(axum::middleware::from_fn(auth_layer))
```

## Middleware Templates

### Auth Middleware
Ensures user is authenticated before accessing protected routes.

### Guest Middleware
Redirects authenticated users away from guest-only pages (login/register).

### Verified Middleware
Ensures user's email is verified before accessing certain routes.

### Role Middleware
Checks if user has required role(s).

## Integration with rf-auth

The generated controllers integrate seamlessly with rf-auth:

```rust
use rf_auth::{
    JwtManager,
    PasswordHasher,
    PasswordReset,
    EmailVerification,
};

// Password hashing
let hasher = PasswordHasher::bcrypt(12)?;
let hash = hasher.hash(&password)?;

// JWT tokens
let jwt = JwtManager::new("your-secret")?;
let token = jwt.generate_token(&claims)?;

// Password reset
let reset = PasswordReset::new(jwt.clone());
reset.send_reset_link(&email).await?;

// Email verification
let verification = EmailVerification::new(jwt.clone());
verification.send_verification_email(&user).await?;
```

## Integration with rf-blade

Views are rendered using rf-blade:

```rust
use rf_blade::BladeEngine;

let blade = BladeEngine::new("resources/views")?;

let html = blade.render("auth.login", json!({
    "app_name": "My App",
    "old_email": email,
    "errors": error_message
})).await?;
```

## Customization

All generated files are templates that you can customize:

1. **Views**: Modify Blade templates to match your design
2. **Controllers**: Implement database queries and business logic
3. **Routes**: Add or remove routes as needed
4. **Middleware**: Customize authentication logic

## Testing

The crate includes comprehensive tests:

```bash
cargo test -p rf-breeze
```

Test coverage includes:
- Template content verification
- Installation process
- Directory creation
- File generation
- Option handling

## Examples

### Basic Usage

```rust
use rf_breeze::{BreezeScaffold, InstallOptions};

let breeze = BreezeScaffold::new(".")?;
breeze.install(&InstallOptions::default()).await?;
```

### Custom Output Directory

```rust
use std::path::PathBuf;

breeze.install(&InstallOptions {
    output_dir: Some(PathBuf::from("./custom/path")),
    ..Default::default()
}).await?;
```

### Programmatic Installation

```rust
// Install components separately for more control
let breeze = BreezeScaffold::new(".")?;

// Install views first
breeze.install_views().await?;

// Then controllers
breeze.install_controllers().await?;

// Then routes
breeze.install_routes().await?;

// Finally middleware
breeze.install_middleware().await?;
```

## Best Practices

1. **Review Generated Code**: Always review and customize generated code before deployment
2. **Implement Database Logic**: Replace TODO comments with actual database queries
3. **Secure Secrets**: Use environment variables for JWT secrets and other sensitive data
4. **Test Thoroughly**: Test all authentication flows before production use
5. **Customize Views**: Adapt the Tailwind CSS styling to match your brand

## License

This project is licensed under either of:

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
