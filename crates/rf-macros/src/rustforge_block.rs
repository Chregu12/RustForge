//! The `rustforge!` block macro - Ultimate Laravel-like experience
//!
//! Write Rust exactly like Laravel PHP - no imports, no .await, just code!
//!
//! ```rust,ignore
//! rustforge! {
//!     Model!(User: name, email, hidden password);
//!     Model!(Post: title, body, user_id);
//!
//!     // No #[auto_await] needed - it's automatic!
//!     async fn index() -> Response {
//!         let users = User::where("active", true).get();
//!         Response::json(users)
//!     }
//!
//!     async fn show(id: i64) -> Response {
//!         let user = User::findOrFail(id);
//!         Response::json(user)
//!     }
//!
//!     // Use #[sync] to opt-out of auto_await
//!     #[sync]
//!     fn helper() -> String {
//!         "not async".to_string()
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Item,
};

/// Content inside rustforge! { ... }
struct RustforgeBlock {
    items: Vec<Item>,
}

impl Parse for RustforgeBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(RustforgeBlock { items })
    }
}

pub fn rustforge_block_impl(input: TokenStream) -> TokenStream {
    let block = parse_macro_input!(input as RustforgeBlock);

    let mut output_items = Vec::new();

    for item in block.items {
        match item {
            Item::Fn(mut func) => {
                // Check if function has #[sync] / #[no_auto_await] attribute
                let has_sync = func.attrs.iter().any(|attr| {
                    attr.path().is_ident("sync") || attr.path().is_ident("no_auto_await")
                });

                if has_sync {
                    // Remove the opt-out attribute and keep function as-is
                    func.attrs.retain(|attr| {
                        !attr.path().is_ident("sync") && !attr.path().is_ident("no_auto_await")
                    });
                    output_items.push(quote! { #func });
                } else if func.sig.asyncness.is_some() {
                    // Apply auto_await transformation to async functions
                    output_items.push(quote! {
                        #[rf_macros::auto_await]
                        #func
                    });
                } else {
                    output_items.push(quote! { #func });
                }
            }
            Item::Impl(mut impl_block) => {
                // Check for #[sync] / #[no_auto_await] on impl block
                let has_sync = impl_block.attrs.iter().any(|attr| {
                    attr.path().is_ident("sync") || attr.path().is_ident("no_auto_await")
                });

                if has_sync {
                    impl_block.attrs.retain(|attr| {
                        !attr.path().is_ident("sync") && !attr.path().is_ident("no_auto_await")
                    });
                    output_items.push(quote! { #impl_block });
                } else {
                    output_items.push(quote! {
                        #[rf_macros::auto_await]
                        #impl_block
                    });
                }
            }
            Item::Mod(mut module) => {
                // Check for #[sync] / #[no_auto_await] on module
                let has_sync = module.attrs.iter().any(|attr| {
                    attr.path().is_ident("sync") || attr.path().is_ident("no_auto_await")
                });

                if has_sync {
                    module.attrs.retain(|attr| {
                        !attr.path().is_ident("sync") && !attr.path().is_ident("no_auto_await")
                    });
                    output_items.push(quote! { #module });
                } else {
                    output_items.push(quote! {
                        #[rf_macros::auto_await]
                        #module
                    });
                }
            }
            // Keep other items as-is (structs, enums, macros, etc.)
            other => {
                output_items.push(quote! { #other });
            }
        }
    }

    let expanded = quote! {
        // Auto-import everything from rustforge
        use rustforge::*;

        #(#output_items)*
    };

    TokenStream::from(expanded)
}

/// Parse a simpler app! macro for even cleaner syntax
///
/// ```rust,ignore
/// app! {
///     routes {
///         GET "/" => index,
///         GET "/users" => users::index,
///         POST "/users" => users::store,
///         GET "/users/:id" => users::show,
///     }
///
///     middleware ["auth"] {
///         GET "/profile" => profile::show,
///         PUT "/profile" => profile::update,
///     }
/// }
/// ```
pub fn app_impl(input: TokenStream) -> TokenStream {
    // For now, just parse as token stream and generate route calls
    let input2: TokenStream2 = input.into();

    let expanded = quote! {
        {
            use rustforge::*;

            // The actual parsing would go here
            // For now this is a placeholder
            #input2
        }
    };

    TokenStream::from(expanded)
}
