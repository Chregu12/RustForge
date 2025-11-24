//! # RF Macros
//!
//! Procedural macros for the Rust DX Framework that enable Laravel-style syntax.
//!
//! ## Available Macros
//!
//! - `function!`: Converts function syntax to async closures with automatic `.await` insertion
//! - `rules!`: Creates validation rules with pipe syntax
//! - `#[controller]`: Marks structs as controllers and auto-converts methods

extern crate proc_macro;

mod await_transformer;
mod controller_macro;
mod function_macro;
mod rules_macro;

use proc_macro::TokenStream;

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
