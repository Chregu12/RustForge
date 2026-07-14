//! Controller attribute macro for automatic route registration

use crate::await_transformer::AwaitTransformer;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, visit_mut::VisitMut, ImplItem, ImplItemFn, ItemImpl,
    Visibility,
};

pub fn controller_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut impl_block = parse_macro_input!(item as ItemImpl);

    // Process each method in the impl block
    for item in &mut impl_block.items {
        if let ImplItem::Fn(method) = item {
            process_controller_method(method);
        }
    }

    let expanded = quote! {
        #impl_block
    };

    TokenStream::from(expanded)
}

fn process_controller_method(method: &mut ImplItemFn) {
    // Check if the method is public
    if !matches!(method.vis, Visibility::Public(_)) {
        return;
    }

    // Make the function async if it isn't already
    if method.sig.asyncness.is_none() {
        method.sig.asyncness = Some(syn::token::Async::default());
    }

    // Apply auto-await transformation to the body
    let mut transformer = AwaitTransformer::new();
    for stmt in &mut method.block.stmts {
        if let syn::Stmt::Expr(expr, _) = stmt {
            transformer.visit_expr_mut(expr);
        } else if let syn::Stmt::Local(local) = stmt {
            if let Some(init) = &mut local.init {
                transformer.visit_expr_mut(&mut init.expr);
            }
        }
    }
    if transformer.wrapped {
        let mut stmts = AwaitTransformer::adapter_prelude();
        stmts.append(&mut method.block.stmts);
        method.block.stmts = stmts;
    }
}

// Proc-macro expansion is verified by compilation; unit tests cannot call
// proc-macro functions directly.  Integration / sandbox probes cover the
// generated code paths.
