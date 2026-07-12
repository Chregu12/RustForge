//! Query macro that allows using `where` without r# prefix
//!
//! ```rust,ignore
//! use rustforge::*;
//!
//! // Now you can use `where` like in Laravel!
//! let users = query!(User::where("active", true).get()).await;
//!
//! let admins = query! {
//!     User::where("role", "admin")
//!         .where("active", true)
//!         .orderBy("name", "asc")
//!         .limit(10)
//!         .get()
//! }.await;
//! ```

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree, Ident};
use quote::quote;

pub fn query_impl(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.into();

    // Transform all `where` identifiers to `r#where`
    let transformed = transform_where(input2);

    TokenStream::from(quote! {
        #transformed
    })
}

fn transform_where(input: TokenStream2) -> TokenStream2 {
    input.into_iter().map(|token| {
        match token {
            TokenTree::Ident(ident) if ident == "where" => {
                // Replace `where` with `r#where`
                TokenTree::Ident(Ident::new_raw("where", ident.span()))
            }
            TokenTree::Group(group) => {
                // Recursively transform groups (parentheses, braces, brackets)
                let transformed = transform_where(group.stream());
                TokenTree::Group(proc_macro2::Group::new(group.delimiter(), transformed))
            }
            other => other
        }
    }).collect()
}
