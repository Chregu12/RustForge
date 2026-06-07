//! # RustForge - The Laravel of Rust
//!
//! Write Rust exactly like Laravel PHP!
//!
//! ## Ultimate Experience with `rustforge!` block
//!
//! ```rust,ignore
//! // No imports needed! No #[auto_await] needed! No .await needed!
//! rustforge! {
//!     Model!(User: name, email, hidden password);
//!     Model!(Post: title, body, user_id);
//!
//!     async fn index() -> Response {
//!         let users = User::where("active", true)
//!             .orderBy("name", "asc")
//!             .get();  // No .await!
//!         Response::json(users)
//!     }
//!
//!     async fn show(id: i64) -> Response {
//!         let user = User::findOrFail(id);  // No .await!
//!         Response::json(user)
//!     }
//!
//!     async fn store(data: Json<Value>) -> Response {
//!         let user = User::create(data.0);  // No .await!
//!         Response::json(user)
//!     }
//!
//!     // Use #[sync] to opt-out of auto_await
//!     #[sync]
//!     fn helper() -> String {
//!         "synchronous".to_string()
//!     }
//! }
//! ```
//!
//! ## Alternative: Manual imports
//!
//! ```rust,ignore
//! use rustforge::*;
//!
//! Model!(User: name, email, hidden password);
//!
//! #[auto_await]  // <- Once at top, applies to entire module!
//! mod app {
//!     use super::*;
//!
//!     pub async fn index() -> Response {
//!         let users = User::where("active", true)
//!             .orderBy("name", "asc")
//!             .get();
//!         Response::json(users)
//!     }
//! }
//!
//! pub use app::*;
//! ```

// ============================================================================
// Macros - These are automatically available after `use rustforge::*;`
// ============================================================================

/// Model macro - define models like Laravel's Eloquent
///
/// ```rust
/// use rustforge::*;
///
/// #[model]
/// pub struct User {
///     pub name: String,
///     pub email: String,
///     #[hidden]
///     pub password: String,
/// }
/// ```
pub use rf_model_macro::model;

/// Laravel-like class syntax for models
///
/// ```rust
/// use rustforge::*;
///
/// laravel! {
///     class User extends Model {
///         protected fillable = [name: String, email: String];
///         protected hidden = [password: String];
///     }
/// }
/// ```
pub use rf_macros::laravel;

/// Ultra-simple model macro - the cleanest syntax!
///
/// ```rust
/// use rustforge::*;
///
/// // Minimal - all fields are String by default
/// Model!(User: name, email, hidden password);
///
/// // Or with explicit types
/// Model!(Post {
///     title: String,
///     body: String,
///     user_id: i64,
/// });
/// ```
#[allow(non_snake_case)]
pub use rf_macros::Model;

/// Query macro - use `where` like Laravel!
///
/// In Rust, `where` is a reserved keyword. This macro lets you use it anyway:
///
/// ```rust
/// use rustforge::*;
///
/// let users = query!(User::where("active", true).get()).await;
///
/// let admins = query! {
///     User::where("role", "admin")
///         .where("active", true)
///         .orderBy("name", "asc")
///         .limit(10)
///         .get()
/// }.await;
/// ```
pub use rf_macros::query;

/// Auto-await macro - write async code without explicit .await
///
/// ```rust
/// use rustforge::*;
///
/// #[auto_await]
/// async fn handler() -> Result<Response, Error> {
///     let users = User::filter("active", true).get();  // No .await needed!
///     Ok(Response::json(users))
/// }
/// ```
pub use rf_macros::auto_await;

/// Controller macro
pub use rf_macros::controller;

/// Function macro for inline handlers
pub use rf_macros::function;

/// Validation rules macro
pub use rf_macros::rules;

/// The ultimate Laravel experience - everything automatic!
///
/// No imports needed, no #[auto_await] needed, no .await needed!
///
/// ```rust,ignore
/// rustforge! {
///     Model!(User: name, email, hidden password);
///
///     async fn index() -> Response {
///         let users = User::where("active", true).get();
///         Response::json(users)
///     }
/// }
/// ```
pub use rf_macros::rustforge;

/// Opt-out of auto_await inside rustforge! blocks
///
/// ```rust,ignore
/// rustforge! {
///     #[sync]
///     fn helper() -> String {
///         "synchronous".to_string()
///     }
/// }
/// ```
pub use rf_macros::sync;

// ============================================================================
// Laravel Helper Macros
// ============================================================================

/// Create a Laravel-style collection
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let numbers = collect![1, 2, 3, 4, 5];
/// let doubled = numbers.map(|x| x * 2);
/// ```
pub use rf_macros::collect;

/// Get configuration value
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let db_host = config!("database.host");
/// let timeout = config!("cache.timeout", 3600);
/// ```
pub use rf_macros::config;

/// Get environment variable
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let env = env_var!("APP_ENV");
/// let debug = env_var!("DEBUG", "false");
/// ```
pub use rf_macros::env_var;

/// Generate named route URL
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let url = route!("users.show", id = 123);
/// ```
pub use rf_macros::route;

/// Create HTTP responses easily
///
/// ```rust,ignore
/// use rustforge::*;
///
/// response!(json: data)
/// response!(redirect: "/home")
/// response!(view: "welcome", data)
/// ```
pub use rf_macros::response;

/// Abort with HTTP error
///
/// ```rust,ignore
/// use rustforge::*;
///
/// abort!(404);
/// abort!(403, "Forbidden");
/// ```
pub use rf_macros::abort;

/// Dump and die - debug helper
///
/// ```rust,ignore
/// use rustforge::*;
///
/// dd!(user, request);  // Prints and exits
/// ```
pub use rf_macros::dd;

/// Dump without stopping - debug helper
///
/// ```rust,ignore
/// use rustforge::*;
///
/// dump!(user, config);  // Prints and continues
/// ```
pub use rf_macros::dump;

/// Get old form input value
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let email = old!("email");
/// let name = old!("name", "Default");
/// ```
pub use rf_macros::old;

/// Generate asset URL
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let css = asset!("css/app.css");
/// ```
pub use rf_macros::asset;

/// Generate full URL
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let url = url!("/users/123");
/// ```
pub use rf_macros::url;

// ============================================================================
// Additional Laravel Helper Macros
// ============================================================================

/// Get current datetime
///
/// ```rust,ignore
/// let current = now!();
/// let formatted = now!("%Y-%m-%d");
/// ```
pub use rf_macros::now;

/// Hash passwords with bcrypt
///
/// ```rust,ignore
/// let hashed = bcrypt!(password);
/// let valid = bcrypt!(verify: password, hash);
/// ```
pub use rf_macros::bcrypt;

/// Redirect back to previous URL
///
/// ```rust,ignore
/// return back!();
/// return back!("/fallback");
/// ```
pub use rf_macros::back;

/// Render a view
///
/// ```rust,ignore
/// return view!("welcome");
/// return view!("users.index", users);
/// ```
pub use rf_macros::view;

/// Create redirect response
///
/// ```rust,ignore
/// return redirect!("/home");
/// return redirect!(route: "users.show", id = 1);
/// ```
pub use rf_macros::redirect;

/// Session management
///
/// ```rust,ignore
/// let value = session!("key");
/// session!(set: "key", value);
/// session!(flash: "message", "Success!");
/// ```
pub use rf_macros::session;

/// Authentication helpers
///
/// ```rust,ignore
/// let user = auth!();
/// if auth!(check) { ... }
/// ```
pub use rf_macros::auth;

/// CSRF token
///
/// ```rust,ignore
/// let token = csrf!();
/// csrf!(field)  // Hidden input HTML
/// ```
pub use rf_macros::csrf;

/// Cache operations
///
/// ```rust,ignore
/// let value = cache!("key");
/// cache!(put: "key", value, 3600);
/// ```
pub use rf_macros::cache;

/// Logging
///
/// ```rust,ignore
/// logger!(info: "User logged in");
/// logger!(error: "Failed: {}", msg);
/// ```
pub use rf_macros::logger;

/// Event dispatching
///
/// ```rust,ignore
/// event!(UserCreated { user_id: 123 });
/// ```
pub use rf_macros::event;

/// File storage
///
/// ```rust,ignore
/// let contents = storage!("file.txt");
/// storage!(put: "file.txt", data);
/// ```
pub use rf_macros::storage;

// ============================================================================
// Facades - Static API like Laravel
// ============================================================================

/// Authentication facade
///
/// ```rust,no_run
/// use rustforge::Auth;
/// use serde::Deserialize;
/// use serde_json::json;
///
/// #[derive(Deserialize)]
/// struct User { email: String }
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// Auth::attempt(json!({"email": "...", "password": "..."}))?;
/// let user: Option<User> = Auth::user::<User>();
/// Auth::logout();
/// # Ok(())
/// # }
/// ```
pub use rf_auth_facade::Auth;

/// Cache facade
///
/// ```rust,no_run
/// use rustforge::Cache;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// Cache::put("key", "value", 3600)?;
/// let value: Option<String> = Cache::get("key")?;
/// Cache::forget("key")?;
/// # Ok(())
/// # }
/// ```
pub use rf_cache_facade::Cache;

/// Database facade
///
/// ```rust,no_run
/// use rustforge::DB;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let users = DB::table("users").filter("active", true).get().await?;
/// # Ok(())
/// # }
/// ```
pub use rf_db_facade::DB;

/// Model trait for Eloquent-style queries
///
/// ```rust,no_run
/// use rustforge::Model;
///
/// struct User;
/// impl Model for User {
///     const TABLE: &'static str = "users";
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let users = User::filter("active", true).get().await?;
/// let user = User::find(1).await?;
/// # Ok(())
/// # }
/// ```
pub use rf_db_facade::Model;

/// Event facade
pub use rf_event_facade::Event;

/// Storage facade
///
/// ```rust,no_run
/// use rustforge::Storage;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let contents = b"hello".to_vec();
/// Storage::put("file.txt", contents)?;
/// let data = Storage::get("file.txt")?;
/// # Ok(())
/// # }
/// ```
pub use rf_storage_facade::Storage;

/// Route facade
///
/// ```rust,no_run
/// use rustforge::Route;
///
/// Route::get("/users", "UserController@index");
/// Route::post("/users", "UserController@store");
/// ```
pub use rf_route_facade::Route;

/// Sanctum facade - Laravel-style API token authentication
///
/// ```rust,ignore
/// use rustforge::Sanctum;
///
/// // Check token abilities
/// if Sanctum::tokenCan("read:posts").await {
///     // User can read posts
/// }
///
/// // Create token
/// let token = Sanctum::createToken(&user, "mobile", vec!["*"], None).await?;
///
/// // Revoke tokens
/// Sanctum::revokeCurrentToken().await?;
/// Sanctum::revokeAllTokens(&user).await?;
///
/// // Token management
/// let stats = Sanctum::tokenStats(&user).await?;
/// Sanctum::pruneExpiredTokens().await?;
/// ```
pub use rf_sanctum_facade::Sanctum;

/// Passport facade - Laravel-style OAuth2 server
///
/// ```rust,ignore
/// use rustforge::Passport;
/// use chrono::Duration;
///
/// // Configure token lifetimes
/// Passport::tokensExpireIn(Duration::days(15)).await;
/// Passport::refreshTokensExpireIn(Duration::days(30)).await;
///
/// // Define OAuth scopes
/// Passport::tokensCan(&[
///     ("read:posts", "Read blog posts"),
///     ("write:posts", "Create and edit posts"),
/// ]).await;
///
/// // Create personal access token
/// let token = Passport::createToken(&user, "api", vec!["read:posts"]).await?;
///
/// // Check current token scope
/// if Passport::tokenCan("write:posts").await {
///     // User can write posts
/// }
///
/// // Client management
/// let client = Passport::createClient("My App", "https://app.com/callback").await?;
///
/// // Grant control
/// Passport::enablePasswordGrant().await;
/// Passport::requirePkce(true).await;
/// ```
pub use rf_passport_facade::Passport;

// ============================================================================
// Admin Panel & Monitoring - Laravel Nova & Horizon
// ============================================================================

/// Laravel Nova - Admin Panel Builder
///
/// ```rust,ignore
/// use rustforge::nova::*;
///
/// // Define a resource
/// struct UserResource;
/// impl Resource for UserResource {
///     fn fields(&self) -> Vec<Field> {
///         vec![
///             Field::id("id"),
///             Field::text("name").sortable().searchable(),
///             Field::email("email").rules("required|email"),
///             Field::boolean("is_admin"),
///             Field::datetime("created_at"),
///         ]
///     }
/// }
///
/// // Create Nova instance
/// let nova = Nova::new()
///     .with_path("/admin")
///     .register_resource::<UserResource>();
///
/// // Merge routes
/// let app = Router::new().merge(nova.routes());
/// ```
pub mod nova {
    pub use rf_nova::*;
}

/// Laravel Horizon - Queue Monitoring Dashboard
///
/// ```rust,ignore
/// use rustforge::horizon::*;
///
/// // Create and configure Horizon
/// let horizon = Horizon::builder()
///     .queue_manager(queue_manager)
///     .monitor_queue("default")
///     .monitor_queue("emails")
///     .failed_job_retention_days(7)
///     .build();
///
/// // Start dashboard server
/// horizon.serve("0.0.0.0:8080").await?;
///
/// // Or use the facade
/// Horizon::pause().await?;
/// Horizon::status().await;
/// let metrics = Horizon::queueMetrics("default").await;
/// ```
pub mod horizon {
    pub use rf_horizon::*;
}

// Re-export main types at top level for convenience
pub use rf_nova::Nova;
pub use rf_horizon::Horizon;

// ============================================================================
// HTTP Types
// ============================================================================

pub use rf_request::Request;
pub use rf_response::Response;

// ============================================================================
// Validation
// ============================================================================

pub use rf_validation::Validate;

// ============================================================================
// Form Request - Laravel-style Validation
// ============================================================================

/// Define a form request with automatic validation - Laravel style!
///
/// ```rust,ignore
/// use rustforge::*;
///
/// form_request! {
///     pub struct CreateUserRequest {
///         #[required, email, unique("users", "email")]
///         email: String,
///
///         #[required, min(8)]
///         password: String,
///
///         #[required, min(2), max(100)]
///         name: String,
///     }
///
///     fn authorize(&self) -> bool {
///         Auth::check()
///     }
/// }
///
/// async fn create_user(Validated(req): Validated<CreateUserRequest>) -> Response {
///     let user = User::create(json!({
///         "email": req.email,
///         "name": req.name,
///     })).await;
///     Response::json(user)
/// }
/// ```
pub use rf_macros::form_request;

/// Attribute macro for simpler form request validation
///
/// ```rust,ignore
/// #[validated]
/// struct CreatePostRequest {
///     #[validate(required, min_length(5))]
///     title: String,
///
///     #[validate(required)]
///     body: String,
/// }
/// ```
pub use rf_macros::validated;

/// Validated extractor - automatically validates form requests
///
/// ```rust,ignore
/// async fn store(Validated(request): Validated<CreateUserRequest>) -> Response {
///     // request is already validated!
///     Response::json(request)
/// }
/// ```
pub use rf_validation::Validated;

/// Form request trait
pub use rf_validation::FormRequest;

/// Form request error type
pub use rf_validation::FormRequestError;

/// Form request result type
pub use rf_validation::FormRequestResult;

// ============================================================================
// Exception Handling
// ============================================================================

/// Define a global exception handler
///
/// ```rust,ignore
/// exception_handler! {
///     dont_report: [ValidationException];
///     dont_flash: ["password"];
///
///     fn render(error: &AppError, request: &Request) -> Response {
///         Response::error(500, "Server Error")
///     }
/// }
/// ```
pub use rf_macros::exception_handler;

/// Wrap a handler with exception handling
pub use rf_macros::handle_exceptions;

/// Abort if condition is true
///
/// ```rust,ignore
/// abort_if!(user.is_banned(), 403, "Account banned");
/// ```
pub use rf_macros::abort_if;

/// Abort unless condition is true
///
/// ```rust,ignore
/// abort_unless!(user.has_permission(), 403);
/// ```
pub use rf_macros::abort_unless;

/// Report an exception without throwing
pub use rf_macros::report;

/// Rescue from errors with a fallback value
///
/// ```rust,ignore
/// let user = rescue!(User::find(id).await, User::default());
/// ```
pub use rf_macros::rescue;

// ============================================================================
// Blade-like Template Macros
// ============================================================================

/// Blade-like template macro with Laravel directives
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let html = blade! {
///     <div class="container">
///         @if let Some(user) = user {
///             <h1>Welcome, {{ user.name }}!</h1>
///         } @else {
///             <p>Please log in</p>
///         }
///
///         @foreach post in posts {
///             <li>{{ post.title }}</li>
///         }
///
///         @auth {
///             <a href="/logout">Logout</a>
///         }
///
///         @csrf
///     </div>
/// };
/// ```
pub use rf_macros::blade;

/// Simple HTML template macro
///
/// ```rust,ignore
/// let name = "World";
/// let html = html! {
///     <div>Hello, {name}!</div>
/// };
/// ```
pub use rf_macros::html;

/// Define a template section
///
/// ```rust,ignore
/// section!("content") {
///     <h1>Page Content</h1>
/// }
/// ```
pub use rf_macros::section;

/// Push content to a stack
///
/// ```rust,ignore
/// push!("scripts") {
///     <script src="/js/app.js"></script>
/// }
/// ```
pub use rf_macros::push;

/// Render a stack
///
/// ```rust,ignore
/// let scripts = stack!("scripts");
/// ```
pub use rf_macros::stack;

// ============================================================================
// Laravel-style Email System (Mailable)
// ============================================================================

/// Define a mailable email - Laravel style!
///
/// ```rust,ignore
/// use rustforge::*;
///
/// mailable! {
///     pub struct WelcomeEmail {
///         user: User,
///     }
///
///     fn envelope(&self) -> Envelope {
///         Envelope::new()
///             .subject("Welcome to RustForge!")
///             .from("hello@rustforge.dev")
///     }
///
///     fn content(&self) -> Content {
///         Content::view("emails.welcome")
///             .with("user", &self.user)
///     }
/// }
///
/// // Send email
/// Mail::to("user@example.com")
///     .send(WelcomeEmail { user })
///     .await?;
/// ```
pub use rf_macros::mailable;

/// Attribute macro for simpler mailable definition
///
/// ```rust,ignore
/// #[mail(subject = "Welcome!", view = "emails.welcome")]
/// pub struct WelcomeEmail {
///     pub user: User,
/// }
/// ```
pub use rf_macros::mail;

/// Define a notification - Laravel style!
///
/// ```rust,ignore
/// notification! {
///     pub struct OrderShipped {
///         order: Order,
///     }
///
///     fn via(&self) -> Vec<Channel> {
///         vec![Channel::Mail, Channel::Database]
///     }
///
///     fn to_mail(&self) -> Mailable {
///         Mailable::new()
///             .subject("Your order has shipped!")
///             .view("emails.order_shipped")
///     }
/// }
///
/// // Send notification
/// user.notify(OrderShipped { order }).await?;
/// ```
pub use rf_macros::notification;

/// Markdown email content helper
///
/// ```rust,ignore
/// let content = markdown! {
///     # Welcome {{ user.name }}!
///
///     Thanks for joining us.
/// };
/// ```
pub use rf_macros::markdown;

// ============================================================================
// Common Re-exports
// ============================================================================

/// JSON macro for creating JSON values
pub use serde_json::json;

/// JSON Value type
pub use serde_json::Value;

/// Serde derives
pub use serde::{Deserialize, Serialize};

/// Async trait for implementing async traits
pub use async_trait::async_trait;

// ============================================================================
// Authentication - Sanctum & Passport (Laravel-style)
// ============================================================================

/// Laravel Sanctum - Personal Access Token Authentication
///
/// ```rust,ignore
/// use rustforge::sanctum::*;
///
/// // Create a token
/// let token = user.create_token("mobile-app", vec!["read:posts"], None, &db).await?;
///
/// // Protect routes
/// async fn protected(SanctumAuth(user, token): SanctumAuth<User>) -> Response {
///     Response::json(user)
/// }
/// ```
pub mod sanctum {
    pub use rf_sanctum::*;
}

/// Laravel Passport - OAuth2 Server
///
/// ```rust,ignore
/// use rustforge::passport::*;
///
/// // Configure OAuth2
/// let config = PassportConfig::new()
///     .access_token_lifetime(3600)
///     .enforce_pkce(true);
///
/// // Personal Access Tokens
/// let token = user.create_token("mobile-app", vec!["read:posts"], &db, &config).await?;
///
/// // Protect routes with OAuth
/// async fn protected(PassportAuth(user_id, token): PassportAuth) -> Response {
///     if token.has_scope("read:posts") {
///         Response::json(data)
///     } else {
///         Response::forbidden()
///     }
/// }
/// ```
pub mod passport {
    pub use rf_passport::*;
}

// ============================================================================
// Sanctum & Passport Re-exports (for convenience)
// ============================================================================

// Sanctum - Most common types at top level
pub use rf_sanctum::{
    Tokenable, SanctumAuth, SanctumError, PersonalAccessToken, NewToken,
    TokenRepository as SanctumTokenRepository,
};

// Passport - Most common types at top level
pub use rf_passport::{
    PassportAuth, PassportConfig, PassportError, PassportResult,
    HasApiTokens, OAuthClient, OAuthAccessToken, OAuthRefreshToken,
    Scope, ScopeRepository,
    // Grant types
    AuthorizationCodeGrant, PasswordGrant, ClientCredentialsGrant,
    RefreshTokenGrant, ImplicitGrant,
    // Requests
    AuthorizationRequest, AuthorizationResponse, TokenResponse,
    // PKCE
    generate_code_verifier, generate_code_challenge, verify_code_challenge,
};

// ============================================================================
// Type Aliases for cleaner code
// ============================================================================

/// Standard Result type with String error
pub type Result<T> = std::result::Result<T, String>;

/// Standard Error type
pub type Error = String;
