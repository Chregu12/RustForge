//! Function-like `controller_block!` macro — the vision controller syntax.
//!
//! A top-level bare `controller Name { .. }` keyword is impossible in Rust, so
//! the vision controller is expressed as a function-like macro instead:
//!
//! ```ignore
//! controller_block! {
//!     PostController {
//!         index() { json(Post::all()) }
//!         show()  { json(Post::find(input::<i64>("id").unwrap())) }
//!         store() { json(Post::create(all())) }
//!     }
//! }
//! ```
//!
//! It expands to a unit struct plus an inherent `impl` of `async`, argument-less
//! handler functions that use the implicit-request globals (`input()`/`file()`)
//! and return an `IntoResponse`, so they register with the real framework router:
//!
//! ```ignore
//! get("/posts", PostController::index);
//! post("/posts", PostController::store);
//! let app = global_router().build_router();
//! ```
//!
//! This is additive: the existing `#[controller]` attribute macro (which decorates
//! a hand-written `impl` block) is untouched. `controller_block!` differs only in
//! that it *generates* the struct + impl from the block syntax, and — like
//! `#[controller]` — applies the auto-await transformation to each body so
//! framework calls resolve without spelling out `.await`.

use crate::await_transformer::AwaitTransformer;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    visit_mut::VisitMut,
    Block, Ident, Token, Type,
};

struct ControllerBlock {
    name: Ident,
    methods: Vec<ControllerMethod>,
}

struct ControllerMethod {
    name: Ident,
    ret: Option<Type>,
    body: Block,
}

impl Parse for ControllerBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Tolerate an optional leading `pub` / `struct` so both
        // `PostController { .. }` and `pub struct PostController { .. }` parse.
        if input.peek(Token![pub]) {
            input.parse::<syn::Visibility>()?;
        }
        if input.peek(Token![struct]) {
            input.parse::<Token![struct]>()?;
        }

        let name: Ident = input.parse()?;

        let content;
        syn::braced!(content in input);

        let mut methods = Vec::new();
        while !content.is_empty() {
            methods.push(content.parse::<ControllerMethod>()?);
            // Allow (but do not require) separators between methods.
            while content.peek(Token![,]) || content.peek(Token![;]) {
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                } else {
                    content.parse::<Token![;]>()?;
                }
            }
        }

        Ok(ControllerBlock { name, methods })
    }
}

impl Parse for ControllerMethod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Optional `pub` / `async` / `fn` decorations — the generated method is
        // always `pub async fn`, so these are accepted and normalised away.
        if input.peek(Token![pub]) {
            input.parse::<syn::Visibility>()?;
        }
        if input.peek(Token![async]) {
            input.parse::<Token![async]>()?;
        }
        if input.peek(Token![fn]) {
            input.parse::<Token![fn]>()?;
        }

        let name: Ident = input.parse()?;

        // Argument list must be empty: handlers read the request via the
        // implicit-request globals (`input()`/`file()`), not parameters.
        let args_content;
        syn::parenthesized!(args_content in input);
        if !args_content.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                "controller_block! handlers take no arguments; read the request via the \
                 implicit-request globals input()/file()",
            ));
        }

        // Optional explicit return type; defaults to `impl IntoResponse`.
        let ret = if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            Some(input.parse::<Type>()?)
        } else {
            None
        };

        let body: Block = input.parse()?;

        Ok(ControllerMethod { name, ret, body })
    }
}

pub fn controller_block_impl(input: TokenStream) -> TokenStream {
    let ControllerBlock { name, methods } = parse_macro_input!(input as ControllerBlock);

    let method_tokens: Vec<TokenStream2> = methods
        .into_iter()
        .map(|ControllerMethod { name: mname, ret, mut body }| {
            // Parity with `#[controller]`: resolve framework calls transparently so
            // handler bodies can call `Post::all()` etc. without writing `.await`.
            let mut transformer = AwaitTransformer::new();
            for stmt in &mut body.stmts {
                transformer.visit_stmt_mut(stmt);
            }
            if transformer.wrapped {
                let mut stmts = AwaitTransformer::adapter_prelude();
                stmts.append(&mut body.stmts);
                body.stmts = stmts;
            }

            let ret_ty = match ret {
                Some(t) => quote! { #t },
                // `rf_response` re-exports axum's `IntoResponse` and is re-exported
                // by the `rf` prelude, so this path resolves for an `rf`-only user.
                None => quote! { impl rf_response::IntoResponse },
            };

            quote! {
                pub async fn #mname() -> #ret_ty #body
            }
        })
        .collect();

    let expanded = quote! {
        pub struct #name;

        impl #name {
            #(#method_tokens)*
        }
    };

    TokenStream::from(expanded)
}

// Proc-macro expansion is verified by compilation; unit tests cannot call
// proc-macro functions directly.  The sandbox probe (controller_macro) drives
// real requests through the generated handlers.
