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
mod laravel_syntax;
mod rules_macro;

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
#[proc_macro_attribute]
pub fn auto_await(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(item as ItemFn);

    // Create the transformer
    let mut transformer = AwaitTransformer::new();

    // Transform all statements in the function body
    for stmt in &mut function.block.stmts {
        transformer.visit_stmt_mut(stmt);
    }

    // Return the transformed function
    TokenStream::from(quote! {
        #function
    })
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
