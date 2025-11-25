//! Exception Handler Macro - Laravel-style Exception Handling
//!
//! Define exception handlers with automatic error reporting and rendering:
//!
//! ```rust,ignore
//! exception_handler! {
//!     // Exceptions that should not be reported (logged)
//!     dont_report: [
//!         ValidationException,
//!         AuthenticationException,
//!     ];
//!
//!     // Form fields that should not be flashed to session
//!     dont_flash: [
//!         "password",
//!         "password_confirmation",
//!     ];
//!
//!     // Custom exception rendering
//!     fn render(error: &AppError, request: &Request) -> Response {
//!         match error {
//!             AppError::NotFound { .. } => {
//!                 if request.wants_json() {
//!                     Response::json(json!({ "error": "Not found" })).status(404)
//!                 } else {
//!                     Response::view("errors.404", json!({})).status(404)
//!                 }
//!             }
//!             _ => Response::error(500, "Server Error")
//!         }
//!     }
//!
//!     // Custom exception reporting
//!     fn report(error: &AppError) {
//!         tracing::error!("Application error: {:?}", error);
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Ident, ItemFn, LitStr, Token,
    punctuated::Punctuated,
    braced, bracketed,
};

/// Parsed exception handler definition
struct ExceptionHandlerDef {
    dont_report: Vec<Ident>,
    dont_flash: Vec<LitStr>,
    render_fn: Option<ItemFn>,
    report_fn: Option<ItemFn>,
    register_fn: Option<ItemFn>,
}

impl Parse for ExceptionHandlerDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut dont_report = Vec::new();
        let mut dont_flash = Vec::new();
        let mut render_fn = None;
        let mut report_fn = None;
        let mut register_fn = None;

        while !input.is_empty() {
            if input.peek(Ident) {
                let lookahead: Ident = input.fork().parse()?;
                let name = lookahead.to_string();

                match name.as_str() {
                    "dont_report" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let content;
                        bracketed!(content in input);
                        dont_report = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
                            .into_iter()
                            .collect();
                        input.parse::<Token![;]>()?;
                    }
                    "dont_flash" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let content;
                        bracketed!(content in input);
                        dont_flash = Punctuated::<LitStr, Token![,]>::parse_terminated(&content)?
                            .into_iter()
                            .collect();
                        input.parse::<Token![;]>()?;
                    }
                    "fn" => {
                        let func: ItemFn = input.parse()?;
                        let func_name = func.sig.ident.to_string();
                        match func_name.as_str() {
                            "render" => render_fn = Some(func),
                            "report" => report_fn = Some(func),
                            "register" => register_fn = Some(func),
                            _ => {}
                        }
                    }
                    _ => {
                        // Skip unknown tokens
                        let _: proc_macro2::TokenTree = input.parse()?;
                    }
                }
            } else if input.peek(Token![fn]) {
                let func: ItemFn = input.parse()?;
                let func_name = func.sig.ident.to_string();
                match func_name.as_str() {
                    "render" => render_fn = Some(func),
                    "report" => report_fn = Some(func),
                    "register" => register_fn = Some(func),
                    _ => {}
                }
            } else {
                // Skip unknown tokens
                let _: proc_macro2::TokenTree = input.parse()?;
            }
        }

        Ok(ExceptionHandlerDef {
            dont_report,
            dont_flash,
            render_fn,
            report_fn,
            register_fn,
        })
    }
}

pub fn exception_handler_impl(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as ExceptionHandlerDef);

    let dont_report_check = if def.dont_report.is_empty() {
        quote! { false }
    } else {
        let checks: Vec<TokenStream2> = def.dont_report.iter().map(|ex| {
            quote! { error.is::<#ex>() }
        }).collect();
        quote! { #(#checks)||* }
    };

    let dont_flash_list = &def.dont_flash;

    let render_impl = if let Some(func) = &def.render_fn {
        let block = &func.block;
        quote! {
            fn render(&self, error: &rf_core::error::AppError, request: &rf_request::Request) -> rf_response::Response {
                #block
            }
        }
    } else {
        quote! {
            fn render(&self, error: &rf_core::error::AppError, _request: &rf_request::Request) -> rf_response::Response {
                use rf_core::error::AppError;

                match error {
                    AppError::NotFound { message } => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Not Found",
                            "message": message
                        })).status(404)
                    }
                    AppError::Unauthorized { message } => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Unauthorized",
                            "message": message
                        })).status(401)
                    }
                    AppError::Forbidden { message } => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Forbidden",
                            "message": message
                        })).status(403)
                    }
                    AppError::BadRequest { message } => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Bad Request",
                            "message": message
                        })).status(400)
                    }
                    AppError::ValidationError { errors } => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Validation Failed",
                            "errors": errors
                        })).status(422)
                    }
                    AppError::InternalError { message } => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Internal Server Error",
                            "message": if cfg!(debug_assertions) { message.clone() } else { "An error occurred".to_string() }
                        })).status(500)
                    }
                    _ => {
                        rf_response::Response::json(serde_json::json!({
                            "error": "Internal Server Error"
                        })).status(500)
                    }
                }
            }
        }
    };

    let report_impl = if let Some(func) = &def.report_fn {
        let block = &func.block;
        quote! {
            fn report(&self, error: &rf_core::error::AppError) {
                #block
            }
        }
    } else {
        quote! {
            fn report(&self, error: &rf_core::error::AppError) {
                tracing::error!(error = ?error, "Application error occurred");
            }
        }
    };

    let expanded = quote! {
        pub struct ExceptionHandler;

        impl ExceptionHandler {
            pub fn new() -> Self {
                Self
            }

            /// Check if the error should be reported (logged)
            pub fn should_report(&self, error: &rf_core::error::AppError) -> bool {
                !(#dont_report_check)
            }

            /// Get fields that should not be flashed to session on validation errors
            pub fn dont_flash(&self) -> &'static [&'static str] {
                &[#(#dont_flash_list),*]
            }

            #render_impl

            #report_impl

            /// Handle an exception: report and render
            pub fn handle(&self, error: &rf_core::error::AppError, request: &rf_request::Request) -> rf_response::Response {
                if self.should_report(error) {
                    self.report(error);
                }
                self.render(error, request)
            }
        }

        impl Default for ExceptionHandler {
            fn default() -> Self {
                Self::new()
            }
        }

        /// Global exception handler instance
        pub static EXCEPTION_HANDLER: std::sync::LazyLock<ExceptionHandler> =
            std::sync::LazyLock::new(|| ExceptionHandler::new());
    };

    TokenStream::from(expanded)
}

/// Attribute macro to mark functions with exception handling
///
/// ```rust,ignore
/// #[handle_exceptions]
/// async fn my_handler(req: Request) -> Response {
///     // If this throws, it will be handled by the exception handler
///     let user = User::findOrFail(id).await?;
///     Response::json(user)
/// }
/// ```
pub fn handle_exceptions_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);

    let vis = &func.vis;
    let sig = &func.sig;
    let block = &func.block;
    let attrs = &func.attrs;

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            let result: std::result::Result<rf_response::Response, rf_core::error::AppError> = (|| async {
                #block
            })().await;

            match result {
                Ok(response) => response,
                Err(error) => {
                    // Use global exception handler if available
                    if let Some(handler) = std::option_env!("RUSTFORGE_EXCEPTION_HANDLER") {
                        // Custom handling
                    }

                    // Default error response
                    rf_response::Response::json(serde_json::json!({
                        "error": format!("{:?}", error)
                    })).status(500)
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Abort macro - throw an HTTP exception
///
/// ```rust,ignore
/// abort!(404);
/// abort!(403, "Access denied");
/// abort_if!(condition, 404);
/// abort_unless!(has_permission, 403);
/// ```
pub fn abort_if_impl(input: TokenStream) -> TokenStream {
    struct AbortIfArgs {
        condition: syn::Expr,
        _comma: Token![,],
        code: syn::Expr,
        message: Option<syn::Expr>,
    }

    impl Parse for AbortIfArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let condition = input.parse()?;
            let _comma = input.parse()?;
            let code = input.parse()?;
            let message = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok(AbortIfArgs { condition, _comma, code, message })
        }
    }

    let args = parse_macro_input!(input as AbortIfArgs);
    let condition = &args.condition;
    let code = &args.code;

    let expanded = if let Some(message) = args.message {
        quote! {
            if #condition {
                return rf_response::Response::error(#code, #message);
            }
        }
    } else {
        quote! {
            if #condition {
                return rf_response::Response::status(#code);
            }
        }
    };

    TokenStream::from(expanded)
}

/// Abort unless condition is true
pub fn abort_unless_impl(input: TokenStream) -> TokenStream {
    struct AbortUnlessArgs {
        condition: syn::Expr,
        _comma: Token![,],
        code: syn::Expr,
        message: Option<syn::Expr>,
    }

    impl Parse for AbortUnlessArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let condition = input.parse()?;
            let _comma = input.parse()?;
            let code = input.parse()?;
            let message = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok(AbortUnlessArgs { condition, _comma, code, message })
        }
    }

    let args = parse_macro_input!(input as AbortUnlessArgs);
    let condition = &args.condition;
    let code = &args.code;

    let expanded = if let Some(message) = args.message {
        quote! {
            if !(#condition) {
                return rf_response::Response::error(#code, #message);
            }
        }
    } else {
        quote! {
            if !(#condition) {
                return rf_response::Response::status(#code);
            }
        }
    };

    TokenStream::from(expanded)
}

/// Try-catch like error handling
///
/// ```rust,ignore
/// try_catch! {
///     try {
///         let user = User::findOrFail(id).await;
///         Response::json(user)
///     } catch ValidationError as e {
///         Response::json(e.errors()).status(422)
///     } catch NotFoundError {
///         Response::status(404)
///     } catch {
///         Response::status(500)
///     }
/// }
/// ```
pub fn try_catch_impl(input: TokenStream) -> TokenStream {
    // For now, just pass through - full implementation would require more complex parsing
    let input2: TokenStream2 = input.into();

    let expanded = quote! {
        {
            let __result = (|| -> std::result::Result<rf_response::Response, Box<dyn std::error::Error>> {
                #input2
            })();

            match __result {
                Ok(response) => response,
                Err(error) => {
                    rf_response::Response::json(serde_json::json!({
                        "error": error.to_string()
                    })).status(500)
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Report an exception without throwing
///
/// ```rust,ignore
/// report!(error);
/// ```
pub fn report_impl(input: TokenStream) -> TokenStream {
    let error: syn::Expr = parse_macro_input!(input as syn::Expr);

    let expanded = quote! {
        {
            tracing::error!(error = ?#error, "Exception reported");
        }
    };

    TokenStream::from(expanded)
}

/// Rescue from errors with a fallback
///
/// ```rust,ignore
/// let user = rescue!(User::find(id).await, User::default());
/// ```
pub fn rescue_impl(input: TokenStream) -> TokenStream {
    struct RescueArgs {
        expr: syn::Expr,
        _comma: Token![,],
        fallback: syn::Expr,
    }

    impl Parse for RescueArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(RescueArgs {
                expr: input.parse()?,
                _comma: input.parse()?,
                fallback: input.parse()?,
            })
        }
    }

    let args = parse_macro_input!(input as RescueArgs);
    let expr = &args.expr;
    let fallback = &args.fallback;

    let expanded = quote! {
        {
            match #expr {
                Ok(value) => value,
                Err(_) => #fallback,
            }
        }
    };

    TokenStream::from(expanded)
}
