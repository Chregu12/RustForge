//! Crate-level RustForge attribute
//!
//! Add `#![rustforge]` at the top of your crate to auto-import everything:
//!
//! ```rust,ignore
//! #![rustforge]
//!
//! // No imports needed! Everything is available.
//! Model!(User {
//!     name: String,
//!     email: String,
//!     hidden password: String,
//! });
//!
//! async fn example() {
//!     let users = User::all().await;
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemMod, Item};

pub fn rustforge_crate_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // This is a crate-level attribute that adds the prelude import
    let input = parse_macro_input!(item as ItemMod);

    let expanded = quote! {
        use rf_prelude::*;
        #input
    };

    TokenStream::from(expanded)
}
