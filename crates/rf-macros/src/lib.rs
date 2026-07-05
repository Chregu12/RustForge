//! # RF Macros
//!
//! Procedural macros for the Rust DX Framework that enable Laravel-style syntax.
//!
//! ## Available Macros
//!
//! - `#[auto_await]`: Automatically adds `.await` to async function calls
//! - `function!`: Converts function syntax to async closures with automatic `.await` insertion
//! - `rules!`: Creates validation rules with pipe syntax
//! - `#[controller]`: Marks structs as controllers and auto-converts methods
//! - `laravel!`: Define models with PHP-like syntax
//!
//! ## Example: Laravel-like syntax
//!
//! ```rust,ignore
//! use rf_macros::laravel;
//!
//! laravel! {
//!     class User extends Model {
//!         protected fillable = [name: String, email: String];
//!         protected hidden = [password: String];
//!     }
//! }
//!
//! // Then use Laravel-style queries:
//! let users = User::where("active", true).get().await?;
//! ```
//!
//! ## Example: auto_await
//!
//! ```rust,ignore
//! use rf_macros::auto_await;
//!
//! #[auto_await]
//! async fn get_users() -> Result<Vec<User>, Error> {
//!     // No .await needed! The macro adds it automatically.
//!     let users = User::filter("active", true).get();
//!     let cached = Cache::get("stats");
//!     Ok(users)
//! }
//! ```

extern crate proc_macro;

mod await_transformer;
mod blade_macro;
mod controller_block_macro;
mod controller_macro;
mod exception_handler;
mod form_request_macro;
mod function_macro;
mod helpers;
mod job_derive;
mod laravel_macros;
mod laravel_syntax;
mod mailable_macro;
mod query_macro;
mod rules_macro;
mod rustforge_block;
mod simple_model;
mod validate_macro;

use await_transformer::AwaitTransformer;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, visit_mut::VisitMut, ItemFn};

/// Converts a function-like syntax into an async closure.
///
/// # Example
///
/// ```ignore
/// use rf_macros::function;
/// use rf_request::Request;
/// use rf_response::Response;
///
/// let handler = function!(request: Request) -> Response {
///     Response::text("Hello, World!")
/// };
/// ```
///
/// This expands to:
///
/// ```ignore
/// |request: Request| async move -> Response {
///     Response::text("Hello, World!")
/// }
/// ```
#[proc_macro]
pub fn function(input: TokenStream) -> TokenStream {
    function_macro::function_impl(input)
}

/// Creates validation rules with pipe syntax.
///
/// # Example
///
/// ```ignore
/// use rf_macros::rules;
///
/// let validation_rules = rules! {
///     name: required | min(3),
///     email: required | email,
/// };
/// ```
///
/// This creates a `ValidationRules` struct with the specified rules.
#[proc_macro]
pub fn rules(input: TokenStream) -> TokenStream {
    rules_macro::rules_impl(input)
}

/// Typed, fluent validation DSL that validates the current request.
///
/// ```ignore
/// let data = validate! {
///     title: string.max(255),
///     email: email,
///     age:   int.min(18),
///     bio:   string.optional,
/// }?;
/// ```
///
/// The leading type disambiguates length-vs-numeric `min`/`max`; fields are
/// required unless `.optional`/`.nullable`. Expands to an `.await`ed validation
/// of `rf_request::all()`, yielding `Result<ValidatedData, ValidationErrors>`.
#[proc_macro]
pub fn validate(input: TokenStream) -> TokenStream {
    validate_macro::validate_impl(input)
}

/// Marks a struct implementation as a controller.
///
/// Automatically converts all public methods to async functions
/// and applies auto-await transformation.
///
/// # Example
///
/// ```ignore
/// use rf_macros::controller;
///
/// struct UserController;
///
/// #[controller]
/// impl UserController {
///     pub fn index(request: Request) -> Response {
///         let users = User::all();
///         Response::json(users)
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    controller_macro::controller_impl(attr, item)
}

/// The vision controller syntax as a function-like macro.
///
/// A bare top-level `controller Name { .. }` keyword is impossible in Rust, so the
/// vision controller is written as `controller_block! { Name { method() { .. } } }`.
/// It generates a unit struct plus an inherent `impl` of `async`, argument-less
/// handler methods that return an `IntoResponse` and read the request through the
/// implicit-request globals (`input()`/`file()`) — so they register directly with
/// the framework router.
///
/// This is additive: the existing `#[controller]` attribute macro (which decorates
/// a hand-written `impl`) is unchanged.
///
/// # Example
///
/// ```ignore
/// use rf::prelude::*;
///
/// controller_block! {
///     PostController {
///         index() { json(Post::all()) }
///         show()  { json(Post::find(input::<i64>("id").unwrap())) }
///         store() { json(Post::create(all())) }
///     }
/// }
///
/// get("/posts", PostController::index);
/// get("/posts/:id", PostController::show);
/// post("/posts", PostController::store);
/// let app = global_router().build_router();
/// ```
///
/// Each method may declare an explicit return type (`show() -> Response { .. }`);
/// otherwise it defaults to `impl IntoResponse`.
#[proc_macro]
pub fn controller_block(input: TokenStream) -> TokenStream {
    controller_block_macro::controller_block_impl(input)
}

/// Automatically adds `.await` to async function calls.
///
/// This macro transforms your code to automatically add `.await` to known
/// async functions like `User::find()`, `Cache::get()`, etc.
///
/// # Example
///
/// ```ignore
/// use rf_macros::auto_await;
///
/// #[auto_await]
/// async fn index() -> Result<Response, Error> {
///     // Write code without .await - the macro adds it!
///     let users = User::filter("active", true).get();
///     let cached = Cache::get("stats");
///     let user = User::find(1);
///
///     Ok(Response::json(users))
/// }
///
/// // The macro transforms it to:
/// async fn index() -> Result<Response, Error> {
///     let users = User::filter("active", true).get().await;
///     let cached = Cache::get("stats").await;
///     let user = User::find(1).await;
///
///     Ok(Response::json(users))
/// }
/// ```
///
/// ## Supported Methods
///
/// The macro automatically adds `.await` to these methods:
///
/// **Model (rf-db-facade):**
/// - `find`, `find_or_fail`, `all`, `first`, `first_or_fail`
/// - `create`, `update`, `delete`, `destroy`, `save`
/// - `get`, `exists`, `count`, `paginate`
///
/// **Cache (rf-cache-facade):**
/// - `get`, `put`, `forget`, `flush`, `has`
/// - `remember`, `forever`, `add`, `pull`
///
/// **Auth (rf-auth-facade):**
/// - `attempt`, `login`, `logout`, `check`, `user`
///
/// **And more:** `send`, `push`, `dispatch`, `execute`, etc.
///
/// ## `where` Keyword Support
///
/// This macro also transforms `where` to `r#where` automatically,
/// so you can write Laravel-style queries without the `query!` macro.
///
/// ## Usage
///
/// **On a module (recommended)** - applies to ALL async functions:
/// ```ignore
/// #[auto_await]
/// mod handlers {
///     async fn index() {
///         let users = User::where("active", true).get();
///     }
///
///     async fn show(id: i64) {
///         let user = User::findOrFail(id);
///     }
/// }
/// ```
///
/// **On a single function:**
/// ```ignore
/// #[auto_await]
/// async fn example() {
///     let users = User::where("active", true).get();
/// }
/// ```
#[proc_macro_attribute]
pub fn auto_await(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Extra async method names to also resolve, from either form:
    //   #[auto_await(fetch_report, charge)]   (identifiers — preferred)
    //   #[auto_await(also("fetch_report"))]   (strings — still accepted)
    auto_await_core(extra_method_names(attr), item)
}

/// `#[await_calls(fetch_report, charge)]` — a clearer, string-free alias for
/// `#[auto_await(...)]` that lists your own async methods as plain identifiers.
#[proc_macro_attribute]
pub fn await_calls(attr: TokenStream, item: TokenStream) -> TokenStream {
    auto_await_core(extra_method_names(attr), item)
}

/// Shared implementation: resolve framework calls (plus `extra`) on a function,
/// impl block, or module — transparently for sync and async calls.
fn auto_await_core(extra: Vec<String>, item: TokenStream) -> TokenStream {
    // First: Transform `where` to `r#where` at token level
    let transformed_tokens = transform_where_tokens(item.clone());

    // Try to parse as module first
    if let Ok(mut module) = syn::parse::<syn::ItemMod>(transformed_tokens.clone()) {
        if let Some((_brace, items)) = &mut module.content {
            for item in items.iter_mut() {
                if let syn::Item::Fn(func) = item {
                    transform_function(func, &extra);
                }
            }
        }
        return TokenStream::from(quote! { #module });
    }

    // Try to parse as impl block
    if let Ok(mut impl_block) = syn::parse::<syn::ItemImpl>(transformed_tokens.clone()) {
        for item in &mut impl_block.items {
            if let syn::ImplItem::Fn(method) = item {
                transform_impl_method(method, &extra);
            }
        }
        return TokenStream::from(quote! { #impl_block });
    }

    // Otherwise parse as function
    let transformed_tokens = transform_where_tokens(item);
    let mut function = parse_macro_input!(transformed_tokens as ItemFn);
    transform_function(&mut function, &extra);

    TokenStream::from(quote! {
        #function
    })
}

/// Extract extra method names from an attribute: bare identifiers (e.g.
/// `fetch_report, charge`) and/or string literals inside `also("...")`. The
/// `also` keyword itself is ignored.
fn extra_method_names(attr: TokenStream) -> Vec<String> {
    use proc_macro2::{TokenStream as TS2, TokenTree};
    fn walk(ts: TS2, out: &mut Vec<String>) {
        for tt in ts {
            match tt {
                TokenTree::Literal(lit) => {
                    let s = lit.to_string();
                    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                        out.push(s[1..s.len() - 1].to_string());
                    }
                }
                TokenTree::Ident(id) => {
                    let s = id.to_string();
                    if s != "also" {
                        out.push(s);
                    }
                }
                TokenTree::Group(g) => walk(g.stream(), out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(attr.into(), &mut out);
    out
}

/// Transform a function so framework calls resolve transparently (sync or async).
fn transform_function(function: &mut ItemFn, extra: &[String]) {
    let mut transformer = AwaitTransformer::with_extra(extra.to_vec());
    for stmt in &mut function.block.stmts {
        transformer.visit_stmt_mut(stmt);
    }
    if transformer.wrapped {
        // The wrapped calls now contain `.await`, so the function must be async.
        // Make a plain `fn` async automatically (it then returns `impl Future`),
        // so a developer can write `#[auto_await] fn handler()` without `async`.
        if function.sig.asyncness.is_none() {
            function.sig.asyncness = Some(syn::token::Async::default());
        }
        let mut stmts = AwaitTransformer::adapter_prelude();
        stmts.append(&mut function.block.stmts);
        function.block.stmts = stmts;
    }
}

/// Transform an impl method so framework calls resolve transparently.
fn transform_impl_method(method: &mut syn::ImplItemFn, extra: &[String]) {
    let mut transformer = AwaitTransformer::with_extra(extra.to_vec());
    for stmt in &mut method.block.stmts {
        transformer.visit_stmt_mut(stmt);
    }
    if transformer.wrapped {
        let mut stmts = AwaitTransformer::adapter_prelude();
        stmts.append(&mut method.block.stmts);
        method.block.stmts = stmts;
    }
}

/// Transform `where` identifiers to `r#where` in token stream
fn transform_where_tokens(input: TokenStream) -> TokenStream {
    use proc_macro2::{TokenStream as TokenStream2, TokenTree, Ident};

    let input2: TokenStream2 = input.into();

    fn transform(stream: TokenStream2) -> TokenStream2 {
        stream.into_iter().map(|token| {
            match token {
                TokenTree::Ident(ident) if ident.to_string() == "where" => {
                    // Check if it's likely a method call (preceded by . or ::)
                    // We transform all `where` to `r#where` - Rust will error if misused
                    TokenTree::Ident(Ident::new_raw("where", ident.span()))
                }
                TokenTree::Group(group) => {
                    let transformed = transform(group.stream());
                    TokenTree::Group(proc_macro2::Group::new(group.delimiter(), transformed))
                }
                other => other
            }
        }).collect()
    }

    TokenStream::from(transform(input2))
}

/// Define models using Laravel-like PHP syntax.
///
/// This macro allows you to write models that look almost identical to Laravel:
///
/// # Example
///
/// ```ignore
/// use rf_macros::laravel;
///
/// laravel! {
///     class User extends Model {
///         protected fillable = [name: String, email: String];
///         protected hidden = [password: String];
///     }
/// }
///
/// laravel! {
///     class Post extends Model {
///         protected table = "blog_posts";
///         protected fillable = [title: String, body: String, author_id: i64];
///         protected timestamps = true;
///     }
/// }
/// ```
///
/// ## Generated Code
///
/// The macro generates:
/// - A struct with the specified fields
/// - `impl Model for YourModel` with the table name
/// - `FILLABLE` and `HIDDEN` constants
/// - Default implementation
///
/// ## Then use Laravel-style queries:
///
/// ```ignore
/// // All these work!
/// let users = User::where("active", true).get().await?;
/// let user = User::find(1).await?;
/// let admins = User::where("role", "admin")
///     .where("active", true)
///     .order_by("name", "asc")
///     .limit(10)
///     .get().await?;
/// ```
#[proc_macro]
pub fn laravel(input: TokenStream) -> TokenStream {
    laravel_syntax::laravel_impl(input)
}

/// Ultra-simple model definition macro.
///
/// # Minimal Syntax
///
/// ```ignore
/// // All fields default to String type
/// Model!(User: name, email, hidden password);
/// ```
///
/// # Full Syntax
///
/// ```ignore
/// Model!(User {
///     name: String,
///     email: String,
///     hidden password: String,
///     age: i32,
/// });
/// ```
///
/// # With Custom Table
///
/// ```ignore
/// Model!(Post {
///     table = "blog_posts",
///     title: String,
///     body: String,
/// });
/// ```
///
/// # Generated Code
///
/// The macro generates:
/// - Struct with `id`, `created_at`, `updated_at` fields
/// - `impl Model` with table name
/// - `FILLABLE` and `HIDDEN` constants
/// - `Default` implementation
#[proc_macro]
pub fn Model(input: TokenStream) -> TokenStream {
    simple_model::simple_model_impl(input)
}

/// Query macro that allows using `where` without the r# prefix.
///
/// In Rust, `where` is a reserved keyword. This macro transforms
/// `where` to `r#where` internally, so you can write Laravel-like code:
///
/// # Example
///
/// ```ignore
/// use rustforge::*;
///
/// // With query! macro - use `where` like Laravel!
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
///
/// This is equivalent to:
/// ```ignore
/// let users = User::r#where("active", true).get().await;
/// ```
#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    query_macro::query_impl(input)
}

// =============================================================================
// Laravel-style Helper Macros
// =============================================================================

/// Create a Laravel-style collection.
///
/// # Example
///
/// ```ignore
/// use rf_macros::collect;
///
/// let numbers = collect![1, 2, 3, 4, 5];
/// let doubled = numbers.map(|x| x * 2);
/// let sum = numbers.sum();
/// let filtered = numbers.filter(|x| x > 2);
/// ```
#[proc_macro]
pub fn collect(input: TokenStream) -> TokenStream {
    helpers::collect_impl(input)
}

/// Get a configuration value.
///
/// # Example
///
/// ```ignore
/// use rf_macros::config;
///
/// let db_host = config!("database.host");
/// let timeout = config!("cache.timeout", 3600);
/// ```
#[proc_macro]
pub fn config(input: TokenStream) -> TokenStream {
    helpers::config_impl(input)
}

/// Get an environment variable with optional default.
///
/// Note: This is named `env_var` to avoid conflict with std::env!
///
/// # Example
///
/// ```ignore
/// use rf_macros::env_var;
///
/// let app_env = env_var!("APP_ENV");
/// let debug = env_var!("APP_DEBUG", "false");
/// ```
#[proc_macro]
pub fn env_var(input: TokenStream) -> TokenStream {
    helpers::env_helper_impl(input)
}

/// Generate a URL for a named route.
///
/// # Example
///
/// ```ignore
/// use rf_macros::route;
///
/// let url = route!("users.show", id = 123);
/// let home = route!("home");
/// ```
#[proc_macro]
pub fn route(input: TokenStream) -> TokenStream {
    helpers::route_impl(input)
}

/// Create various HTTP responses easily.
///
/// # Example
///
/// ```ignore
/// use rf_macros::response;
///
/// // JSON response
/// response!(json: data)
///
/// // Text response
/// response!(text: "Hello World")
///
/// // Redirect
/// response!(redirect: "/home")
///
/// // View with data
/// response!(view: "users.index", users_data)
///
/// // Status code only
/// response!(status: 204)
///
/// // File download
/// response!(download: "/path/to/file.pdf")
/// ```
#[proc_macro]
pub fn response(input: TokenStream) -> TokenStream {
    helpers::response_impl(input)
}

/// Abort with an HTTP error code and optional message.
///
/// # Example
///
/// ```ignore
/// use rf_macros::abort;
///
/// abort!(404);
/// abort!(403, "Forbidden");
/// abort!(500, "Server Error");
/// ```
#[proc_macro]
pub fn abort(input: TokenStream) -> TokenStream {
    helpers::abort_impl(input)
}

/// Dump values and die (for debugging).
///
/// # Example
///
/// ```ignore
/// use rf_macros::dd;
///
/// dd!(user, request, "some value");
/// // Prints debug info and exits
/// ```
#[proc_macro]
pub fn dd(input: TokenStream) -> TokenStream {
    helpers::dd_impl(input)
}

/// Dump values without stopping execution.
///
/// # Example
///
/// ```ignore
/// use rf_macros::dump;
///
/// dump!(user, config);
/// // Prints debug info and continues
/// ```
#[proc_macro]
pub fn dump(input: TokenStream) -> TokenStream {
    helpers::dump_impl(input)
}

/// Get old form input value (for repopulating forms after validation errors).
///
/// # Example
///
/// ```ignore
/// use rf_macros::old;
///
/// let email = old!("email");
/// let name = old!("name", "Default Name");
/// ```
#[proc_macro]
pub fn old(input: TokenStream) -> TokenStream {
    helpers::old_impl(input)
}

/// Generate an asset URL.
///
/// # Example
///
/// ```ignore
/// use rf_macros::asset;
///
/// let css = asset!("css/app.css");
/// let js = asset!("js/app.js");
/// ```
#[proc_macro]
pub fn asset(input: TokenStream) -> TokenStream {
    helpers::asset_impl(input)
}

/// Generate a full URL for a path.
///
/// # Example
///
/// ```ignore
/// use rf_macros::url;
///
/// let full_url = url!("/users/123");
/// ```
#[proc_macro]
pub fn url(input: TokenStream) -> TokenStream {
    helpers::url_impl(input)
}

// =============================================================================
// Additional Laravel Helper Macros
// =============================================================================

/// Get the current datetime
///
/// # Example
///
/// ```ignore
/// use rf_macros::now;
///
/// let current = now!();
/// let formatted = now!("%Y-%m-%d");
/// ```
#[proc_macro]
pub fn now(input: TokenStream) -> TokenStream {
    helpers::now_impl(input)
}

/// Hash or verify passwords with bcrypt
///
/// # Example
///
/// ```ignore
/// use rf_macros::bcrypt;
///
/// let hashed = bcrypt!(password);
/// let hashed = bcrypt!(password, 12);  // custom cost
/// let valid = bcrypt!(verify: password, hash);
/// ```
#[proc_macro]
pub fn bcrypt(input: TokenStream) -> TokenStream {
    helpers::bcrypt_impl(input)
}

/// Redirect back to the previous URL
///
/// # Example
///
/// ```ignore
/// use rf_macros::back;
///
/// return back!();
/// return back!("/fallback");
/// ```
#[proc_macro]
pub fn back(input: TokenStream) -> TokenStream {
    helpers::back_impl(input)
}

/// Render a view template
///
/// # Example
///
/// ```ignore
/// use rf_macros::view;
///
/// return view!("welcome");
/// return view!("users.index", users);
/// ```
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    helpers::view_impl(input)
}

/// Create a redirect response
///
/// # Example
///
/// ```ignore
/// use rf_macros::redirect;
///
/// return redirect!("/home");
/// return redirect!("/users/{}", user_id);
/// return redirect!(route: "users.show", id = 1);
/// ```
#[proc_macro]
pub fn redirect(input: TokenStream) -> TokenStream {
    helpers::redirect_impl(input)
}

/// Session management
///
/// # Example
///
/// ```ignore
/// use rf_macros::session;
///
/// let value = session!("key");
/// session!(set: "key", value);
/// session!(flash: "message", "Success!");
/// ```
#[proc_macro]
pub fn session(input: TokenStream) -> TokenStream {
    helpers::session_impl(input)
}

/// Authentication helpers
///
/// # Example
///
/// ```ignore
/// use rf_macros::auth;
///
/// let user = auth!();
/// if auth!(check) { ... }
/// auth!(logout);
/// ```
#[proc_macro]
pub fn auth(input: TokenStream) -> TokenStream {
    helpers::auth_impl(input)
}

/// CSRF token helper
///
/// # Example
///
/// ```ignore
/// use rf_macros::csrf;
///
/// let token = csrf!();
/// let field = csrf!(field);
/// let meta = csrf!(meta);
/// ```
#[proc_macro]
pub fn csrf(input: TokenStream) -> TokenStream {
    helpers::csrf_impl(input)
}

/// Cache operations
///
/// # Example
///
/// ```ignore
/// use rf_macros::cache;
///
/// let value = cache!("key");
/// cache!(put: "key", value, 3600);
/// cache!(forget: "key");
/// ```
#[proc_macro]
pub fn cache(input: TokenStream) -> TokenStream {
    helpers::cache_impl(input)
}

/// Logging helper
///
/// # Example
///
/// ```ignore
/// use rf_macros::logger;
///
/// logger!(info: "User logged in");
/// logger!(error: "Failed: {}", msg);
/// ```
#[proc_macro]
pub fn logger(input: TokenStream) -> TokenStream {
    helpers::logger_impl(input)
}

/// Event dispatching
///
/// # Example
///
/// ```ignore
/// use rf_macros::event;
///
/// event!(UserCreated { user_id: 123 });
/// ```
#[proc_macro]
pub fn event(input: TokenStream) -> TokenStream {
    helpers::event_impl(input)
}

/// File storage operations
///
/// # Example
///
/// ```ignore
/// use rf_macros::storage;
///
/// let contents = storage!("file.txt");
/// storage!(put: "file.txt", data);
/// storage!(delete: "file.txt");
/// ```
#[proc_macro]
pub fn storage(input: TokenStream) -> TokenStream {
    helpers::storage_impl(input)
}

// =============================================================================
// Laravel-style Eloquent Macros (CRUD without json! or .await!)
// =============================================================================

/// Update a model record - Laravel-style without json! or explicit .await!
///
/// # Example
///
/// ```ignore
/// use rf_macros::update;
///
/// // Clean syntax - no json!, automatic .await!
/// update!(User, 1, name = "John Doe");
/// update!(User, user_id, name = "John", email = "john@example.com");
///
/// // Equivalent to:
/// // User::update_by_id(1, json!({"name": "John Doe"})).await
/// ```
#[proc_macro]
pub fn update(input: TokenStream) -> TokenStream {
    helpers::update_impl(input)
}

/// Create a new model record - Laravel-style without json! or explicit .await!
///
/// # Example
///
/// ```ignore
/// use rf_macros::create;
///
/// // Clean syntax - no json!, automatic .await!
/// let user = create!(User, name = "John", email = "john@example.com");
///
/// // Equivalent to:
/// // User::create(json!({"name": "John", "email": "john@example.com"})).await
/// ```
#[proc_macro]
pub fn create(input: TokenStream) -> TokenStream {
    helpers::create_impl(input)
}

/// Find a model by ID - Laravel-style with automatic .await!
///
/// # Example
///
/// ```ignore
/// use rf_macros::find;
///
/// let user = find!(User, 1);
/// let post = find!(Post, post_id);
///
/// // Equivalent to:
/// // User::find(1).await
/// ```
#[proc_macro]
pub fn find(input: TokenStream) -> TokenStream {
    helpers::find_impl(input)
}

/// Delete a model by ID - Laravel-style with automatic .await!
///
/// # Example
///
/// ```ignore
/// use rf_macros::delete;
///
/// delete!(User, 1);
/// delete!(Post, post_id);
///
/// // Equivalent to:
/// // User::destroy(1).await
/// ```
#[proc_macro]
pub fn delete(input: TokenStream) -> TokenStream {
    helpers::delete_impl(input)
}

// =============================================================================
// Ultimate Laravel Experience - rustforge! block
// =============================================================================

/// The ultimate Laravel-like experience in Rust!
///
/// Write Rust exactly like Laravel PHP:
/// - No `use rustforge::*;` needed - automatic!
/// - No `#[auto_await]` needed - automatic!
/// - No `.await` needed - automatic!
/// - Use `#[sync]` to opt-out for specific functions
///
/// # Example
///
/// ```ignore
/// rustforge! {
///     Model!(User: name, email, hidden password);
///     Model!(Post: title, body, user_id);
///
///     // Routes
///     fn routes() {
///         Route::get("/", index);
///         Route::get("/users", users_index);
///         Route::post("/users", users_store);
///     }
///
///     // Handlers - no .await needed!
///     async fn index() -> Response {
///         Response::text("Welcome to RustForge!")
///     }
///
///     async fn users_index() -> Response {
///         let users = User::where("active", true)
///             .orderBy("name", "asc")
///             .get();  // No .await!
///         Response::json(users)
///     }
///
///     async fn users_store(data: Json<Value>) -> Response {
///         let user = User::create(data.0);  // No .await!
///         Response::json(user).status(201)
///     }
///
///     // Opt-out with #[sync] for non-async helpers
///     #[sync]
///     fn format_name(name: &str) -> String {
///         name.to_uppercase()
///     }
/// }
/// ```
///
/// This expands to:
/// ```ignore
/// use rustforge::*;
///
/// Model!(User: name, email, hidden password);
/// Model!(Post: title, body, user_id);
///
/// fn routes() { ... }
///
/// #[auto_await]
/// async fn index() -> Response { ... }
///
/// #[auto_await]
/// async fn users_index() -> Response { ... }
///
/// fn format_name(name: &str) -> String { ... }
/// ```
#[proc_macro]
pub fn rustforge(input: TokenStream) -> TokenStream {
    rustforge_block::rustforge_block_impl(input)
}

/// Marker attribute for opting out of auto_await inside rustforge! blocks
///
/// Use this when you have a synchronous function that shouldn't have
/// auto_await applied:
///
/// ```ignore
/// rustforge! {
///     #[sync]
///     fn helper() -> String {
///         "I'm synchronous!".to_string()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn sync(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This is just a marker - the actual handling is in rustforge! macro
    item
}

// =============================================================================
// Laravel-style Form Request Validation
// =============================================================================

/// Define a form request with automatic validation - Laravel style!
///
/// This macro creates a validated form request struct with automatic
/// validation rules, custom messages, and authorization checks.
///
/// # Example - Block Syntax
///
/// ```ignore
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
///         // Return true to allow, false to reject with 403
///         Auth::check()
///     }
///
///     fn messages() -> HashMap<&'static str, &'static str> {
///         HashMap::from([
///             ("email.required", "Email address is required"),
///             ("email.email", "Please provide a valid email address"),
///             ("password.min", "Password must be at least 8 characters"),
///         ])
///     }
/// }
///
/// // Use in a handler with automatic validation:
/// async fn create_user(
///     Validated(request): Validated<CreateUserRequest>,
/// ) -> Response {
///     // request is already validated!
///     let user = User::create(json!({
///         "email": request.email,
///         "password": bcrypt!(request.password),
///         "name": request.name,
///     })).await;
///     Response::json(user).status(201)
/// }
/// ```
///
/// # Available Validation Rules
///
/// ## Basic Rules
/// - `required` - Field must not be empty
/// - `nullable` - Field can be null/empty
/// - `string` - Must be a string
/// - `integer` - Must be an integer
/// - `numeric` - Must be numeric
/// - `boolean` - Must be true/false
/// - `array` - Must be an array
///
/// ## String Rules
/// - `email` - Valid email format
/// - `url` - Valid URL format
/// - `ip` - Valid IP address
/// - `uuid` - Valid UUID
/// - `alpha` - Only letters
/// - `alpha_num` - Letters and numbers
/// - `lowercase` - Must be lowercase
/// - `uppercase` - Must be uppercase
/// - `regex("pattern")` - Must match regex
///
/// ## Length/Size Rules
/// - `min(n)` - Minimum value/length
/// - `max(n)` - Maximum value/length
/// - `between(min, max)` - Value between range
/// - `min_length(n)` - Minimum string length
/// - `max_length(n)` - Maximum string length
/// - `size(n)` - Exact size
///
/// ## Date Rules
/// - `date` - Valid date
/// - `date_format("format")` - Specific date format
/// - `before("date")` - Before date
/// - `after("date")` - After date
///
/// ## Database Rules
/// - `unique("table", "column")` - Must be unique in database
/// - `exists("table", "column")` - Must exist in database
///
/// ## Comparison Rules
/// - `same("field")` - Must match another field
/// - `different("field")` - Must differ from another field
/// - `confirmed` - Must have matching {field}_confirmation
///
/// ## Conditional Rules
/// - `required_if("field", "value")` - Required if other field equals value
/// - `required_unless("field", "value")` - Required unless other field equals value
/// - `required_with("field")` - Required if other field is present
/// - `required_without("field")` - Required if other field is absent
#[proc_macro]
pub fn form_request(input: TokenStream) -> TokenStream {
    form_request_macro::form_request_impl(input)
}

/// Attribute macro for simpler form request validation
///
/// Apply to a struct to automatically implement FormRequest trait.
///
/// # Example
///
/// ```ignore
/// #[validated]
/// struct CreatePostRequest {
///     #[validate(required, min_length(5))]
///     title: String,
///
///     #[validate(required)]
///     body: String,
///
///     #[validate(required, exists("categories", "id"))]
///     category_id: i64,
/// }
///
/// async fn create_post(
///     Validated(req): Validated<CreatePostRequest>,
/// ) -> Response {
///     let post = Post::create(json!({
///         "title": req.title,
///         "body": req.body,
///         "category_id": req.category_id,
///     })).await;
///     Response::json(post).status(201)
/// }
/// ```
#[proc_macro_attribute]
pub fn validated(attr: TokenStream, item: TokenStream) -> TokenStream {
    form_request_macro::form_request_attr_impl(attr, item)
}

// =============================================================================
// Laravel-style Exception Handling
// =============================================================================

/// Define a global exception handler - Laravel style!
///
/// This macro creates an exception handler that controls how errors are
/// reported (logged) and rendered (HTTP responses).
///
/// # Example
///
/// ```ignore
/// use rustforge::*;
///
/// exception_handler! {
///     // Exceptions that should not be logged
///     dont_report: [
///         ValidationException,
///         AuthenticationException,
///     ];
///
///     // Form fields not flashed to session
///     dont_flash: [
///         "password",
///         "password_confirmation",
///     ];
///
///     // Custom exception rendering
///     fn render(error: &AppError, request: &Request) -> Response {
///         match error {
///             AppError::NotFound { .. } => {
///                 if request.wants_json() {
///                     Response::json(json!({ "error": "Not found" })).status(404)
///                 } else {
///                     Response::view("errors.404", json!({})).status(404)
///                 }
///             }
///             _ => Response::error(500, "Server Error")
///         }
///     }
///
///     // Custom exception reporting
///     fn report(error: &AppError) {
///         tracing::error!("Application error: {:?}", error);
///         // Send to Sentry, Bugsnag, etc.
///     }
/// }
/// ```
#[proc_macro]
pub fn exception_handler(input: TokenStream) -> TokenStream {
    exception_handler::exception_handler_impl(input)
}

/// Wrap a handler with exception handling
///
/// ```ignore
/// #[handle_exceptions]
/// async fn my_handler(req: Request) -> Response {
///     let user = User::findOrFail(id).await?;
///     Response::json(user)
/// }
/// ```
#[proc_macro_attribute]
pub fn handle_exceptions(attr: TokenStream, item: TokenStream) -> TokenStream {
    exception_handler::handle_exceptions_impl(attr, item)
}

/// Abort if condition is true
///
/// ```ignore
/// abort_if!(user.is_banned(), 403, "Account is banned");
/// abort_if!(post.is_deleted(), 404);
/// ```
#[proc_macro]
pub fn abort_if(input: TokenStream) -> TokenStream {
    exception_handler::abort_if_impl(input)
}

/// Abort unless condition is true
///
/// ```ignore
/// abort_unless!(user.can_edit(&post), 403, "Not authorized");
/// abort_unless!(auth!(check), 401);
/// ```
#[proc_macro]
pub fn abort_unless(input: TokenStream) -> TokenStream {
    exception_handler::abort_unless_impl(input)
}

/// Report an exception without throwing
///
/// ```ignore
/// report!(error);
/// ```
#[proc_macro]
pub fn report(input: TokenStream) -> TokenStream {
    exception_handler::report_impl(input)
}

/// Rescue from errors with a fallback value
///
/// ```ignore
/// let user = rescue!(User::find(id).await, User::default());
/// let config = rescue!(Config::load(), Config::default());
/// ```
#[proc_macro]
pub fn rescue(input: TokenStream) -> TokenStream {
    exception_handler::rescue_impl(input)
}

// =============================================================================
// Blade-like Template Macros
// =============================================================================

/// Blade-like template macro with Laravel directives
///
/// Write HTML templates with familiar Blade syntax:
///
/// # Example
///
/// ```ignore
/// use rustforge::*;
///
/// let user = User::find(1).await;
///
/// let html = blade! {
///     <div class="container">
///         @if let Some(user) = user {
///             <h1>Welcome, {{ user.name }}!</h1>
///
///             @if user.is_admin {
///                 <span class="badge">Admin</span>
///             } @else {
///                 <span class="badge">User</span>
///             }
///
///             <ul>
///             @foreach post in user.posts {
///                 <li>{{ post.title }}</li>
///             }
///             </ul>
///         } @else {
///             <p>Please log in</p>
///         }
///
///         @auth {
///             <a href="/logout">Logout</a>
///         }
///
///         @guest {
///             <a href="/login">Login</a>
///         }
///
///         @csrf
///     </div>
/// };
/// ```
///
/// # Available Directives
///
/// ## Control Flow
/// - `@if condition { ... }` - Conditional rendering
/// - `@else { ... }` - Else branch
/// - `@else if condition { ... }` - Else if branch
/// - `@foreach item in collection { ... }` - Loop iteration
/// - `@for expr { ... }` - For loop
/// - `@while condition { ... }` - While loop
/// - `@match expr { ... }` - Match expression
///
/// ## Authentication
/// - `@auth { ... }` - Content for authenticated users
/// - `@guest { ... }` - Content for guests
///
/// ## Forms
/// - `@csrf` - CSRF token hidden input
/// - `@method("PUT")` - HTTP method spoofing
///
/// ## Content
/// - `{{ expr }}` - Escaped output
/// - `{!! expr !!}` - Unescaped output (raw HTML)
/// - `@json(data)` - JSON output
/// - `@include("partial")` - Include template
///
/// ## Utilities
/// - `@isset(var) { ... }` - Check if set
/// - `@empty(collection) { ... }` - Check if empty
/// - `@env("KEY")` - Environment variable
/// - `@rust { code }` - Execute Rust code
#[proc_macro]
pub fn blade(input: TokenStream) -> TokenStream {
    blade_macro::blade_impl(input)
}

/// Simple HTML template macro
///
/// ```ignore
/// let name = "World";
/// let html = html! {
///     <div>Hello, {name}!</div>
/// };
/// ```
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    blade_macro::html_impl(input)
}

/// Define a template section
///
/// ```ignore
/// section!("content") {
///     <h1>Page Content</h1>
/// }
/// ```
#[proc_macro]
pub fn section(input: TokenStream) -> TokenStream {
    blade_macro::section_impl(input)
}

/// Push content to a stack
///
/// ```ignore
/// push!("scripts") {
///     <script src="/js/app.js"></script>
/// }
/// ```
#[proc_macro]
pub fn push(input: TokenStream) -> TokenStream {
    blade_macro::push_impl(input)
}

/// Render a stack
///
/// ```ignore
/// let scripts = stack!("scripts");
/// ```
#[proc_macro]
pub fn stack(input: TokenStream) -> TokenStream {
    blade_macro::stack_impl(input)
}

// =============================================================================
// Laravel-style Email System (Mailable)
// =============================================================================

/// Define a mailable email - Laravel style!
///
/// Create structured emails with envelope, content, and attachments:
///
/// # Example
///
/// ```ignore
/// use rustforge::*;
///
/// mailable! {
///     pub struct WelcomeEmail {
///         user: User,
///         activation_url: String,
///     }
///
///     fn envelope(&self) -> Envelope {
///         Envelope::new()
///             .subject("Welcome to RustForge!")
///             .from("hello@rustforge.dev")
///             .reply_to("support@rustforge.dev")
///     }
///
///     fn content(&self) -> Content {
///         Content::view("emails.welcome")
///             .with("user", &self.user)
///             .with("url", &self.activation_url)
///     }
///
///     fn attachments(&self) -> Vec<Attachment> {
///         vec![
///             Attachment::from_path("/docs/getting-started.pdf")
///                 .as_("Getting Started Guide.pdf")
///                 .with_mime("application/pdf"),
///         ]
///     }
/// }
///
/// // Send email
/// Mail::to("user@example.com")
///     .send(WelcomeEmail {
///         user,
///         activation_url: "https://rustforge.dev/activate/abc123".into(),
///     })
///     .await?;
///
/// // Queue for later
/// Mail::to("user@example.com")
///     .queue(WelcomeEmail { user, activation_url })
///     .delay(Duration::from_secs(60))
///     .await?;
/// ```
#[proc_macro]
pub fn mailable(input: TokenStream) -> TokenStream {
    mailable_macro::mailable_impl(input)
}

/// Attribute macro for simpler mailable definition
///
/// ```ignore
/// #[mail(subject = "Welcome!", view = "emails.welcome")]
/// pub struct WelcomeEmail {
///     pub user: User,
/// }
/// ```
#[proc_macro_attribute]
pub fn mail(attr: TokenStream, item: TokenStream) -> TokenStream {
    mailable_macro::mailable_attr_impl(attr, item)
}

/// Define a notification - Laravel style!
///
/// Notifications can be sent via multiple channels (mail, database, Slack, etc.)
///
/// # Example
///
/// ```ignore
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
///             .with("order", &self.order)
///     }
///
///     fn to_database(&self) -> Value {
///         json!({
///             "type": "order_shipped",
///             "order_id": self.order.id,
///             "message": format!("Order #{} has shipped!", self.order.id)
///         })
///     }
/// }
///
/// // Send notification to user
/// user.notify(OrderShipped { order }).await?;
///
/// // Send to multiple users
/// Notification::send(users, OrderShipped { order }).await?;
/// ```
#[proc_macro]
pub fn notification(input: TokenStream) -> TokenStream {
    mailable_macro::notification_impl(input)
}

/// Markdown email content helper
///
/// ```ignore
/// let content = markdown! {
///     # Welcome {{ user.name }}!
///
///     Thanks for joining. Here's what to do next:
///
///     - Create your first project
///     - Invite team members
///     - Start building
/// };
/// ```
#[proc_macro]
pub fn markdown(input: TokenStream) -> TokenStream {
    mailable_macro::markdown_impl(input)
}

// =============================================================================
// MEDIUM PRIORITY Laravel Helper Macros
// =============================================================================

/// Send emails with Laravel-style syntax
///
/// Note: Using send_mail! to avoid conflict with the #[mail] attribute macro.
/// In Laravel-style code, you can use this for quick email sending.
///
/// # Example
///
/// ```ignore
/// use rf_macros::send_mail;
///
/// // Simple email
/// send_mail!(to = "user@example.com", subject = "Welcome!", body = "Hello!");
///
/// // With template
/// send_mail!(
///     to = "user@example.com",
///     subject = "Order Confirmation",
///     template = "emails.order",
///     data = order_data,
/// );
///
/// // With attachment and queue
/// send_mail!(
///     to = "user@example.com",
///     subject = "Invoice",
///     template = "emails.invoice",
///     data = invoice_data,
///     attach = "/path/to/invoice.pdf",
///     queue,
/// );
/// ```
#[proc_macro]
pub fn send_mail(input: TokenStream) -> TokenStream {
    helpers::mail_impl(input)
}

/// Dispatch events with Laravel-style syntax
///
/// # Example
///
/// ```ignore
/// use rf_macros::dispatch;
///
/// // Dispatch event with named fields
/// dispatch!(user.registered, user_id = 1, email = "john@example.com");
///
/// // Dispatch event struct
/// dispatch!(UserRegistered { user_id: 1, email: "john@example.com".into() });
///
/// // Delayed dispatch
/// dispatch!(order.shipped, order_id = 123, delay = 3600);
/// dispatch!(delay: 3600, OrderShipped { order_id: 123 });
/// ```
#[proc_macro]
pub fn dispatch(input: TokenStream) -> TokenStream {
    helpers::dispatch_impl(input)
}

/// Define background jobs with Laravel-style syntax
///
/// # Example
///
/// ```ignore
/// use rf_macros::job;
///
/// job! {
///     SendEmail(to: String, subject: String, body: String) {
///         Mail::to(&self.to).subject(&self.subject).body(&self.body).send().await
///     }
///
///     retries = 3,
///     timeout = 300,
///     backoff = [60, 300, 900],
/// }
///
/// job! {
///     ProcessVideo(video_id: i64, resolution: String) {
///         let video = Video::find(self.video_id).await?;
///         video.process(&self.resolution).await?;
///         Ok(())
///     }
///
///     queue = "video-processing",
///     retries = 5,
/// }
///
/// // Then dispatch the job:
/// SendEmail::new("user@example.com".into(), "Hello".into(), "World".into())
///     .dispatch()
///     .await?;
/// ```
#[proc_macro]
pub fn job(input: TokenStream) -> TokenStream {
    helpers::job_impl(input)
}

/// Derive the `rf_queue::Job` trait, generating all of its mechanical wiring so
/// defining a background job is minimal.
///
/// You implement only `rf_queue::JobHandler` (the one `handle` body that
/// matters); the derive supplies `job_type()` (the struct name) and, from an
/// optional `#[job(..)]` attribute, the `queue`/`max_retries`/`timeout`/
/// `priority` accessors — and delegates `Job::handle` to your `JobHandler`.
///
/// # Example
///
/// ```ignore
/// use rf_queue::{Job, JobHandler, QueueError, Jobs, Worker};
/// use async_trait::async_trait;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Job)]
/// #[job(queue = "emails", retries = 5)]
/// struct SendEmail { to: String }
///
/// #[async_trait]
/// impl JobHandler for SendEmail {
///     async fn handle(&self) -> Result<(), QueueError> {
///         println!("emailing {}", self.to);
///         Ok(())
///     }
/// }
///
/// // dispatch_now() / Jobs::dispatch / Worker::register all keep working:
/// SendEmail { to: "a@b.c".into() }.dispatch_now().unwrap();
/// ```
///
/// Supported `#[job(..)]` keys: `job_type` (alias `name`), `queue`, `retries`
/// (alias `max_retries`), `timeout` (seconds), `priority`.
#[proc_macro_derive(Job, attributes(job))]
pub fn derive_job(input: TokenStream) -> TokenStream {
    job_derive::derive_job(input)
}

// =============================================================================
// HIGH PRIORITY Laravel-style Macros (Phase 21 - German Keyboard Fix)
// =============================================================================

/// Define routes using clean Laravel-style syntax (solves German keyboard || problem)
///
/// This macro is the HIGHEST PRIORITY as it eliminates the need for the pipe operator
/// which is difficult to type on German keyboards.
///
/// # Example
///
/// ```ignore
/// use rf_macros::routes;
///
/// routes! {
///     get "/posts" => post_controller::index,
///     post "/posts" => post_controller::store,
///     get "/posts/{id}" => post_controller::show,
///     put "/posts/{id}" => post_controller::update,
///     delete "/posts/{id}" => post_controller::destroy,
///
///     middleware ["auth"] {
///         get "/profile" => profile_controller::show,
///         put "/profile" => profile_controller::update,
///     }
///
///     prefix "/api/v1" {
///         get "/users" => api::users::index,
///         post "/users" => api::users::store,
///     }
/// }
/// ```
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    laravel_macros::routes_impl(input)
}

/// Define RESTful resource routes for a controller
///
/// Automatically generates standard RESTful routes (index, create, store, show, edit, update, destroy).
///
/// # Example
///
/// ```ignore
/// use rf_macros::resource;
///
/// // All routes
/// resource!(posts, PostController);
///
/// // Only specific routes
/// resource!(users, UserController, only: [index, show]);
///
/// // All except specific routes
/// resource!(comments, CommentController, except: [destroy]);
/// ```
#[proc_macro]
pub fn resource(input: TokenStream) -> TokenStream {
    laravel_macros::resource_impl(input)
}

/// Define database migrations with Laravel-style syntax
///
/// # Example
///
/// ```ignore
/// use rf_macros::migration;
///
/// migration! {
///     create_table users {
///         id: primary,
///         email: string unique,
///         name: string,
///         password: string,
///         role: string = "user",
///         timestamps,
///     }
/// }
///
/// migration! {
///     create_table posts {
///         id: primary,
///         user_id: integer,
///         title: string,
///         body: string,
///         published: bool = false,
///         timestamps,
///     }
/// }
/// ```
#[proc_macro]
pub fn migration(input: TokenStream) -> TokenStream {
    laravel_macros::migration_impl(input)
}

/// Define models with relationships using Laravel-style syntax
///
/// # Example
///
/// ```ignore
/// use rf_macros::model;
///
/// model! {
///     Post => "posts" {
///         id: i32 primary,
///         user_id: i32,
///         title: String,
///         content: String,
///         published: bool = false,
///         timestamps,
///
///         belongs_to User via user_id,
///         has_many Comment,
///     }
/// }
///
/// model! {
///     Comment => "comments" {
///         id: i32 primary,
///         post_id: i32,
///         body: String,
///         timestamps,
///
///         belongs_to Post via post_id,
///     }
/// }
/// ```
#[proc_macro]
pub fn model(input: TokenStream) -> TokenStream {
    laravel_macros::model_impl(input)
}

/// Define form request validation with Laravel-style syntax
///
/// # Example
///
/// ```ignore
/// use rf_macros::request;
///
/// request! {
///     CreateUser {
///         email: email,
///         name: length(3, 50),
///         password: length(8) + uppercase + number,
///         age: range(18, 120) | optional,
///     }
/// }
///
/// request! {
///     UpdatePost {
///         title: length(5, 200),
///         body: length(10),
///         published: optional,
///     }
/// }
/// ```
#[proc_macro]
pub fn request(input: TokenStream) -> TokenStream {
    laravel_macros::request_impl(input)
}
