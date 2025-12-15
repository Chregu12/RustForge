use crate::await_transformer::AwaitTransformer;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    token::Paren,
    visit_mut::VisitMut,
    Block, FnArg, PatType, Result, ReturnType, Token,
};

/// Represents the parsed input for the function! macro
struct FunctionInput {
    args: Vec<FnArg>,
    output: ReturnType,
    body: Block,
}

impl Parse for FunctionInput {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse function arguments
        let mut args = Vec::new();

        // Check if there's a parenthesized argument list or direct argument
        let lookahead = input.lookahead1();

        if lookahead.peek(Paren) {
            // Parse parenthesized arguments: (arg1: Type1, arg2: Type2)
            let content;
            syn::parenthesized!(content in input);

            while !content.is_empty() {
                let arg: FnArg = content.parse()?;
                args.push(arg);

                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }
        } else {
            // Parse single argument without parentheses: arg: Type
            let arg: FnArg = input.parse()?;
            args.push(arg);
        }

        // Parse return type
        let output: ReturnType = input.parse()?;

        // Parse body block
        let body: Block = input.parse()?;

        Ok(FunctionInput { args, output, body })
    }
}

pub fn function_impl(input: TokenStream) -> TokenStream {
    let FunctionInput { args, output, mut body } = parse_macro_input!(input as FunctionInput);

    // Apply auto-await transformation to the body
    let mut transformer = AwaitTransformer::new();
    for stmt in &mut body.stmts {
        if let syn::Stmt::Expr(expr, _) | syn::Stmt::Expr(expr @ syn::Expr::Return(_), _) = stmt {
            transformer.visit_expr_mut(expr);
        } else if let syn::Stmt::Local(local) = stmt {
            if let Some(init) = &mut local.init {
                transformer.visit_expr_mut(&mut init.expr);
            }
        }
    }

    // Convert args to closure parameters
    let closure_args = args.iter().map(|arg| match arg {
        FnArg::Typed(PatType { pat, .. }) => pat,
        FnArg::Receiver(_) => panic!("function! macro does not support self parameters"),
    });

    // Generate the async closure
    let expanded = quote! {
        |#(#closure_args),*| async move #output #body
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_function_macro_exists() {
        // Proc-macro functions cannot be called directly in unit tests.
        // The macro is tested via compile tests in the integration tests.
        assert!(true);
    }
}
