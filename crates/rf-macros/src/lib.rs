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
mod controller_macro;
mod function_macro;
mod helpers;
mod laravel_syntax;
mod query_macro;
mod rules_macro;
mod rustforge_block;
mod simple_model;

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
pub fn auto_await(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // First: Transform `where` to `r#where` at token level
    let transformed_tokens = transform_where_tokens(item.clone());

    // Try to parse as module first
    if let Ok(mut module) = syn::parse::<syn::ItemMod>(transformed_tokens.clone()) {
        if let Some((brace, items)) = &mut module.content {
            for item in items.iter_mut() {
                if let syn::Item::Fn(func) = item {
                    transform_function(func);
                }
            }
        }
        return TokenStream::from(quote! { #module });
    }

    // Try to parse as impl block
    if let Ok(mut impl_block) = syn::parse::<syn::ItemImpl>(transformed_tokens.clone()) {
        for item in &mut impl_block.items {
            if let syn::ImplItem::Fn(method) = item {
                transform_impl_method(method);
            }
        }
        return TokenStream::from(quote! { #impl_block });
    }

    // Otherwise parse as function
    let transformed_tokens = transform_where_tokens(item);
    let mut function = parse_macro_input!(transformed_tokens as ItemFn);
    transform_function(&mut function);

    TokenStream::from(quote! {
        #function
    })
}

/// Transform a function by adding .await to async calls
fn transform_function(function: &mut ItemFn) {
    let mut transformer = AwaitTransformer::new();
    for stmt in &mut function.block.stmts {
        transformer.visit_stmt_mut(stmt);
    }
}

/// Transform an impl method by adding .await to async calls
fn transform_impl_method(method: &mut syn::ImplItemFn) {
    let mut transformer = AwaitTransformer::new();
    for stmt in &mut method.block.stmts {
        transformer.visit_stmt_mut(stmt);
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
