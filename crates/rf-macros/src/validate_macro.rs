//! The `validate!` macro: a typed, fluent validation DSL.
//!
//! ```ignore
//! let data = validate! {
//!     title: string.max(255),
//!     email: email,
//!     age:   int.min(18),
//!     bio:   string.optional,
//! }?;
//! ```
//!
//! Unlike the pipe-based `rules!` macro, the leading TYPE disambiguates the
//! length-vs-numeric `min`/`max` collision (`string.max(255)` -> `MaxLengthRule`,
//! `int.max(255)` -> numeric `MaxRule`). Fields are **required by default** unless
//! marked `.optional`/`.nullable`. The macro validates the current request
//! (`rf_request::all()`) and yields `Result<ValidatedData, ValidationErrors>`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Expr, Ident, Result, Token,
};

/// A single `.method` / `.method(args)` modifier.
struct Op {
    name: Ident,
    args: Vec<Expr>,
}

/// One `field: type.mods...` entry.
struct Field {
    name: Ident,
    ty: Ident,
    ops: Vec<Op>,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Ident = input.parse()?;

        let mut ops = Vec::new();
        while input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            let op_name: Ident = input.parse()?;
            let args = if input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in input);
                let parsed: Punctuated<Expr, Token![,]> =
                    content.parse_terminated(Expr::parse, Token![,])?;
                parsed.into_iter().collect()
            } else {
                Vec::new()
            };
            ops.push(Op { name: op_name, args });
        }

        Ok(Field { name, ty, ops })
    }
}

struct ValidateInput {
    fields: Vec<Field>,
}

impl Parse for ValidateInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let fields = Punctuated::<Field, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect();
        Ok(ValidateInput { fields })
    }
}

fn is_numeric(ty: &str) -> bool {
    matches!(ty, "int" | "integer" | "float" | "decimal" | "number")
}

/// The base type rule identifier for a type keyword, if any.
fn base_rule(ty: &str) -> Option<Ident> {
    let name = match ty {
        "string" | "text" => "StringRule",
        "email" => "EmailRule",
        "url" => "UrlRule",
        "uuid" => "UuidRule",
        "ip" => "IpRule",
        "int" | "integer" => "IntegerRule",
        "float" | "decimal" | "number" => "NumericRule",
        "date" | "datetime" => "DateRule",
        "array" => "ArrayRule",
        "bool" | "boolean" => return None, // present-only (no boolean rule yet)
        _ => return None,
    };
    Some(Ident::new(name, proc_macro2::Span::call_site()))
}

fn boxed(rule: TokenStream2) -> TokenStream2 {
    quote! { Box::new(#rule) as Box<dyn rf_validation::Rule> }
}

pub fn validate_impl(input: TokenStream) -> TokenStream {
    let ValidateInput { fields } = parse_macro_input!(input as ValidateInput);

    let mut inserts: Vec<TokenStream2> = Vec::new();

    for field in &fields {
        let field_name = field.name.to_string();
        let ty = field.ty.to_string();
        let numeric = is_numeric(&ty);
        let optional = field
            .ops
            .iter()
            .any(|o| o.name == "optional" || o.name == "nullable");

        // Unknown type keyword -> clear compile error.
        if base_rule(&ty).is_none() && !matches!(ty.as_str(), "bool" | "boolean") {
            let msg = format!(
                "validate!: unknown field type `{}` (expected string/text/email/url/uuid/ip/int/float/decimal/date/array/bool)",
                ty
            );
            return syn::Error::new(field.ty.span(), msg)
                .to_compile_error()
                .into();
        }

        let mut rule_exprs: Vec<TokenStream2> = Vec::new();

        // Required by default unless .optional/.nullable.
        if !optional {
            rule_exprs.push(boxed(quote! { rf_validation::rules::RequiredRule }));
        }
        // Base type rule.
        if let Some(base) = base_rule(&ty) {
            rule_exprs.push(boxed(quote! { rf_validation::rules::#base }));
        }

        // Modifiers.
        for op in &field.ops {
            match op.name.to_string().as_str() {
                "optional" | "nullable" => {}
                "min" => {
                    let arg = &op.args[0];
                    if numeric {
                        rule_exprs.push(boxed(
                            quote! { rf_validation::rules::MinRule::new(#arg as f64) },
                        ));
                    } else {
                        rule_exprs.push(boxed(
                            quote! { rf_validation::rules::MinLengthRule::new(#arg as usize) },
                        ));
                    }
                }
                "max" => {
                    let arg = &op.args[0];
                    if numeric {
                        rule_exprs.push(boxed(
                            quote! { rf_validation::rules::MaxRule::new(#arg as f64) },
                        ));
                    } else {
                        rule_exprs.push(boxed(
                            quote! { rf_validation::rules::MaxLengthRule::new(#arg as usize) },
                        ));
                    }
                }
                "between" => {
                    let lo = &op.args[0];
                    let hi = &op.args[1];
                    rule_exprs.push(boxed(
                        quote! { rf_validation::rules::BetweenRule::new(#lo as f64, #hi as f64) },
                    ));
                }
                other => {
                    let msg = format!("validate!: unknown modifier `.{}`", other);
                    return syn::Error::new(op.name.span(), msg)
                        .to_compile_error()
                        .into();
                }
            }
        }

        inserts.push(quote! {
            __rules.insert(#field_name, vec![ #(#rule_exprs),* ]);
        });
    }

    let expanded = quote! {{
        let mut __rules: std::collections::HashMap<
            &'static str,
            Vec<Box<dyn rf_validation::Rule>>,
        > = std::collections::HashMap::new();
        #(#inserts)*
        let mut __validator = rf_validation::Validator::new(rf_request::all());
        __validator.rules(__rules);
        __validator.validate().await
    }};

    expanded.into()
}
