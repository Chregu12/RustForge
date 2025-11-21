# Security Features - Quick Reference

Quick reference guide for the critical security features implemented in RustForge Framework.

## CSRF Protection

### Basic Usage
```rust
use rf_web::csrf::{CsrfConfig, CsrfLayer, csrf_token, csrf_field};

// Add to router
let app = Router::new()
    .layer(CsrfLayer::new());

// Generate token
let token = csrf_token();

// In HTML form
let html = format!(r#"
    <form method="POST">
        {}
        <input type="text" name="data">
        <button>Submit</button>
    </form>
"#, csrf_field(&token));
```

### Configuration
```rust
let config = CsrfConfig::new()
    .exempt("/api/webhook")
    .exempt("/health")
    .lifetime_hours(4)
    .header_name("X-CSRF-TOKEN");

let app = Router::new()
    .layer(CsrfLayer::with_config(config));
```

## Session Management

### Basic Usage
```rust
use rf_web::session::{SessionMiddleware, CookieSessionDriver, Session};

// Setup
let driver = Arc::new(CookieSessionDriver::new());
let middleware = SessionMiddleware::new(driver);

// In handler
async fn handler(mut session: Session) {
    // Store data
    session.put("user_id", 123);
    session.put("username", "john");

    // Retrieve data
    let user_id: i32 = session.get_as("user_id").unwrap();

    // Flash message
    session.flash("success", "Saved!");

    // Old input
    session.flash_input(form_data);
    let old_email = session.old("email");

    // Security
    session.regenerate().await?;
    session.invalidate().await?;
}
```

### Drivers
```rust
// Cookie (default)
let driver = Arc::new(CookieSessionDriver::new());

// Database
let driver = Arc::new(DatabaseSessionDriver::new("sessions"));

// Redis
let driver = Arc::new(RedisSessionDriver::new("session:"));
```

### Configuration
```rust
let config = SessionConfig::new()
    .cookie_name("my_session")
    .lifetime(3600)
    .secure(true)
    .http_only(true)
    .same_site(SameSite::Strict);

let middleware = SessionMiddleware::with_config(driver, config);
```

## Middleware Stack

### Basic Usage
```rust
use rf_routing::middleware_stack::MiddlewareStack;

let stack = MiddlewareStack::new();

// Global middleware (all routes)
stack.add_global("cors");
stack.add_global("logging");

// Group middleware
stack.add_group("api", vec![
    "auth".to_string(),
    "throttle".to_string(),
]);

// Route middleware
stack.add_route_middleware("users.create", vec![
    "validate".to_string(),
]);

// Resolve
let middleware = stack.resolve("users.create", &["api".into()]);
// Result: ["cors", "logging", "auth", "throttle", "validate"]
```

### Builder Pattern
```rust
use rf_routing::middleware_stack::MiddlewareStackBuilder;

let stack = MiddlewareStackBuilder::new()
    .global("cors")
    .global("logging")
    .group("api", vec!["auth".into(), "throttle".into()])
    .route("users.create", vec!["validate".into()])
    .build();
```

## Form Request Validation

### Define Form Request
```rust
use rf_validation::{FormRequest, Validated, ValidationRules};

#[derive(Deserialize)]
struct CreateUserRequest {
    email: String,
    password: String,
    name: String,
}

#[async_trait]
impl FormRequest for CreateUserRequest {
    type Validated = Self;

    fn rules(&self) -> ValidationRules {
        let mut rules = HashMap::new();
        rules.insert("email", vec![
            Box::new(RequiredRule),
            Box::new(EmailRule),
        ]);
        rules.insert("password", vec![
            Box::new(RequiredRule),
            Box::new(MinLengthRule::new(8)),
        ]);
        rules
    }

    fn messages(&self) -> ValidationMessages {
        let mut messages = HashMap::new();
        messages.insert("email.required", "Email is required");
        messages
    }

    fn authorize(&self) -> bool {
        true  // or check permissions
    }

    async fn validate(self) -> FormRequestResult<Self::Validated> {
        Ok(self)
    }

    fn prepare_for_validation(&mut self) {
        self.email = self.email.trim().to_lowercase();
    }
}
```

### Use in Handler
```rust
async fn create_user(
    Validated(request): Validated<CreateUserRequest>
) -> Json<User> {
    // Request is validated and authorized
    let user = User {
        email: request.email,
        name: request.name,
    };
    Json(user)
}
```

## Complete Example

```rust
use rf_web::{
    csrf::{CsrfConfig, CsrfLayer},
    session::{SessionMiddleware, RedisSessionDriver},
};
use rf_routing::middleware_stack::MiddlewareStack;
use rf_validation::Validated;

#[tokio::main]
async fn main() {
    // Setup CSRF
    let csrf_config = CsrfConfig::new()
        .exempt("/api/")
        .lifetime_hours(2);

    // Setup Session
    let session_driver = Arc::new(RedisSessionDriver::new("session:"));
    let session_middleware = SessionMiddleware::new(session_driver);

    // Setup Middleware Stack
    let stack = MiddlewareStack::new();
    stack.add_global("cors");
    stack.add_group("web", vec!["session".into(), "csrf".into()]);

    // Build app
    let app = Router::new()
        .route("/users", post(create_user))
        .layer(CsrfLayer::with_config(csrf_config));

    // Serve
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_user(
    session: Session,
    Validated(request): Validated<CreateUserRequest>,
) -> impl IntoResponse {
    // Fully secured endpoint
    Json(create_user_in_db(request).await)
}
```

## Security Best Practices

### CSRF
- ✓ Always protect state-changing operations (POST/PUT/DELETE)
- ✓ Use HTTPS in production
- ✓ Regenerate tokens on each request
- ✓ Validate tokens before processing

### Sessions
- ✓ Regenerate session ID after login
- ✓ Use secure, HTTP-only cookies
- ✓ Set appropriate expiration
- ✓ Clear sensitive data on logout
- ✓ Use Redis in production

### Middleware
- ✓ Security middleware first
- ✓ Use groups for consistency
- ✓ Avoid duplicate middleware

### Validation
- ✓ Always validate user input
- ✓ Check authorization
- ✓ Sanitize before validation
- ✓ Return clear error messages

## Common Patterns

### Login Flow
```rust
async fn login(
    mut session: Session,
    Validated(form): Validated<LoginRequest>,
) -> impl IntoResponse {
    // 1. Verify credentials
    let user = authenticate(&form.email, &form.password).await?;

    // 2. Regenerate session (prevent fixation)
    session.regenerate().await?;

    // 3. Store user data
    session.put("user_id", user.id);
    session.put("authenticated", true);

    // 4. Flash success
    session.flash("success", "Login successful!");

    // 5. Redirect
    Redirect::to("/dashboard")
}
```

### Form with Validation Errors
```rust
async fn show_form(session: Session) -> Html<String> {
    let mut session = session;

    // Get validation errors
    let errors = session.get_flash("errors");

    // Get old input
    let old_email = session.old("email").unwrap_or_default();

    // Render form with errors and old input
    Html(render_form(errors, old_email))
}

async fn submit_form(
    mut session: Session,
    form: Result<Validated<MyRequest>, FormRequestError>,
) -> impl IntoResponse {
    match form {
        Ok(Validated(data)) => {
            // Process valid data
            Redirect::to("/success")
        }
        Err(errors) => {
            // Flash errors and old input
            session.flash("errors", errors);
            session.flash_input(extract_input(&form));
            Redirect::to("/form")
        }
    }
}
```

## Troubleshooting

### CSRF Token Mismatch
- Check token is included in form
- Verify middleware is applied
- Check route is not exempt
- Ensure token hasn't expired

### Session Not Persisting
- Check cookie settings
- Verify driver is configured
- Check session.save() is called
- Ensure middleware is applied

### Middleware Not Executing
- Check middleware is registered
- Verify route is in correct group
- Check resolution order
- Ensure middleware stack is applied

## Performance Tips

1. **CSRF**: Minimal overhead, use default settings
2. **Sessions**: Use Redis in production for best performance
3. **Middleware**: Order security middleware first
4. **Validation**: Cache validation rules if possible

## Additional Resources

- Full Documentation: `SECURITY_FEATURES.md`
- Implementation Details: `SECURITY_IMPLEMENTATION_COMPLETE.md`
- Examples: `crates/*/examples/`
- Tests: `crates/*/tests/`
