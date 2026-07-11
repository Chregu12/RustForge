//! The `validate!` macro: a typed, fluent validation DSL.
//!
//! Validates the current request (`rf_request::all()`) — requires the
//! `capture_request` axum middleware — and yields
//! `Result<ValidatedData, ValidationErrors>`.
//!
//! ```rust,ignore
//! let data = validate! {
//!     title:  string.max(255),           // required String ≤ 255 chars
//!     email:  email,                     // required valid email
//!     age:    int.min(18),               // required integer ≥ 18
//!     bio:    string.optional,           // optional String
//!     ref_id: int.exists("posts", "id"), // must reference a real row
//! }?;
//! ```
//!
//! Unlike the pipe-based `rules!` macro, the leading TYPE disambiguates the
//! length-vs-numeric `min`/`max` collision (`string.max(255)` expands to
//! `MaxLengthRule(255)`, `int.max(255)` to the numeric `MaxRule(255.0)`).
//! Fields are **required by default** unless marked `.optional` or `.nullable`.
//!
//! ## Supported type keywords
//!
//! `string` / `text`, `email`, `url`, `uuid`, `ip`,
//! `int` / `integer`, `float` / `decimal` / `number`,
//! `date` / `datetime`, `array`, `bool` / `boolean`,
//! `image` (validates upload MIME type), `file` (any uploaded file).
//!
//! ## Modifiers
//!
//! `.min(n)`, `.max(n)`, `.between(lo, hi)`, `.optional`, `.nullable`,
//! `.unique("table", "col")`, `.exists("table", "col")`.
//! For `image` / `file`: `.mime("image/png")`, `.min(kb(100))`, `.max(mb(5))`.

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

fn is_file(ty: &str) -> bool {
    matches!(ty, "image" | "file")
}

/// Build the file/image validation for one field. Files are NOT in the JSON
/// field map, so this validates the CURRENT request's upload
/// (`rf_request::file(name)`) directly rather than going through the `Validator`.
///
/// Returns the check block on success, or a compile error for a bad modifier.
fn file_check(field: &Field) -> std::result::Result<TokenStream2, syn::Error> {
    let field_name = field.name.to_string();
    let is_image = field.ty == "image";
    let optional = field
        .ops
        .iter()
        .any(|o| o.name == "optional" || o.name == "nullable");

    let mut min_expr: Option<&Expr> = None;
    let mut max_expr: Option<&Expr> = None;
    let mut mime_exprs: Vec<&Expr> = Vec::new();

    for op in &field.ops {
        match op.name.to_string().as_str() {
            "optional" | "nullable" => {}
            "min" => {
                min_expr = op.args.first();
            }
            "max" => {
                max_expr = op.args.first();
            }
            "mime" | "mimes" => {
                mime_exprs.extend(op.args.iter());
            }
            other => {
                return Err(syn::Error::new(
                    op.name.span(),
                    format!(
                        "validate!: unknown modifier `.{}` for a {} field (expected optional/min/max/mime)",
                        other, field.ty
                    ),
                ));
            }
        }
    }

    let mut checks: Vec<TokenStream2> = Vec::new();

    // `image` types must carry an image MIME type.
    if is_image {
        checks.push(quote! {
            if let Err(__m) =
                rf_validation::rules::ImageRule::new().check(__file.content_type())
            {
                __file_errors.add(
                    #field_name,
                    rf_validation::FieldError::new("image", __m),
                );
            }
        });
    }

    // Explicit MIME allow-list via `.mime("image/png", ...)`.
    if !mime_exprs.is_empty() {
        checks.push(quote! {
            if let Err(__m) =
                rf_validation::rules::MimeRule::new([ #(#mime_exprs),* ])
                    .check(__file.content_type())
            {
                __file_errors.add(
                    #field_name,
                    rf_validation::FieldError::new("mime", __m),
                );
            }
        });
    }

    // Size bounds via `.max(mb(5))` / `.min(kb(1))`.
    if max_expr.is_some() || min_expr.is_some() {
        let mut size_rule = quote! { rf_validation::rules::FileSizeRule::new() };
        if let Some(mx) = max_expr {
            size_rule = quote! { #size_rule.max((#mx) as u64) };
        }
        if let Some(mn) = min_expr {
            size_rule = quote! { #size_rule.min((#mn) as u64) };
        }
        checks.push(quote! {
            if let Err(__m) = #size_rule.check(__file.size()) {
                __file_errors.add(
                    #field_name,
                    rf_validation::FieldError::new("file_size", __m),
                );
            }
        });
    }

    let block = if optional {
        // Optional: only validate when a file was actually uploaded.
        quote! {
            if let Some(__file) = rf_request::file(#field_name) {
                #(#checks)*
            }
        }
    } else {
        // Required: absent upload is an error.
        quote! {
            match rf_request::file(#field_name) {
                Some(__file) => { #(#checks)* }
                None => {
                    __file_errors.add(
                        #field_name,
                        rf_validation::FieldError::new(
                            "required",
                            "This field is required".to_string(),
                        ),
                    );
                }
            }
        }
    };

    Ok(block)
}

pub fn validate_impl(input: TokenStream) -> TokenStream {
    let ValidateInput { fields } = parse_macro_input!(input as ValidateInput);

    let mut inserts: Vec<TokenStream2> = Vec::new();
    let mut file_checks: Vec<TokenStream2> = Vec::new();

    for field in &fields {
        let field_name = field.name.to_string();
        let ty = field.ty.to_string();
        let numeric = is_numeric(&ty);
        let optional = field
            .ops
            .iter()
            .any(|o| o.name == "optional" || o.name == "nullable");

        // File/image fields validate the request's uploads, not the JSON map.
        if is_file(&ty) {
            match file_check(field) {
                Ok(block) => file_checks.push(block),
                Err(e) => return e.to_compile_error().into(),
            }
            continue;
        }

        // Unknown type keyword -> clear compile error.
        if base_rule(&ty).is_none() && !matches!(ty.as_str(), "bool" | "boolean") {
            let msg = format!(
                "validate!: unknown field type `{}` (expected string/text/email/url/uuid/ip/int/float/decimal/date/array/bool/image/file)",
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
                // DB-backed rules run a real COUNT(*) via the rf_orm::DB facade:
                // `.unique(table, column)` passes when 0 rows match the value,
                // `.exists(table, column)` passes when >= 1 row matches.
                "unique" => {
                    if op.args.len() != 2 {
                        return syn::Error::new(
                            op.name.span(),
                            "validate!: `.unique(table, column)` requires exactly two arguments",
                        )
                        .to_compile_error()
                        .into();
                    }
                    let table = &op.args[0];
                    let column = &op.args[1];
                    rule_exprs.push(boxed(
                        quote! { rf_validation::rules::DbUniqueRule::new(#table, #column) },
                    ));
                }
                "exists" => {
                    if op.args.len() != 2 {
                        return syn::Error::new(
                            op.name.span(),
                            "validate!: `.exists(table, column)` requires exactly two arguments",
                        )
                        .to_compile_error()
                        .into();
                    }
                    let table = &op.args[0];
                    let column = &op.args[1];
                    rule_exprs.push(boxed(
                        quote! { rf_validation::rules::DbExistsRule::new(#table, #column) },
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
        // Bring the byte-size helpers into scope so `max(mb(5))` / `min(kb(1))`
        // resolve inside the caller's expression (they are not otherwise imported).
        #[allow(unused_imports)]
        use rf_validation::{kb, mb};

        let mut __rules: std::collections::HashMap<
            &'static str,
            Vec<Box<dyn rf_validation::Rule>>,
        > = std::collections::HashMap::new();
        #(#inserts)*
        let mut __validator = rf_validation::Validator::new(rf_request::all());
        __validator.rules(__rules);
        let __result = __validator.validate().await;

        // File/image fields validate the current request's uploads directly.
        let mut __file_errors = rf_validation::ValidationErrors::new();
        #(#file_checks)*

        match __result {
            Ok(__data) => {
                if __file_errors.is_empty() {
                    Ok(__data)
                } else {
                    Err(__file_errors)
                }
            }
            Err(mut __errs) => {
                for (__field, __ferrs) in __file_errors.errors {
                    for __fe in __ferrs {
                        __errs.add(__field.clone(), __fe);
                    }
                }
                Err(__errs)
            }
        }
    }};

    expanded.into()
}
