//! `#[derive(RfModel)]` procedural macro.
//!
//! Generates Laravel-style model metadata methods from struct attributes.
//!
//! # Supported attributes
//!
//! | Attribute | Example | Description |
//! |-----------|---------|-------------|
//! | `#[rf(table = "...")]` | `#[rf(table = "users")]` | Override the table name |
//! | `#[rf(hidden = [...])]` | `#[rf(hidden = ["password"])]` | Hidden field list |
//! | `#[rf(fillable = [...])]` | `#[rf(fillable = ["name"])]` | Fillable field list |
//! | `#[rf(guarded = [...])]` | `#[rf(guarded = ["id"])]` | Guarded field list |
//! | `#[rf(timestamps)]` | `#[rf(timestamps)]` | Enable timestamp support |
//! | `#[rf(soft_delete)]` | `#[rf(soft_delete)]` | Enable soft-delete support |
//!
//! # Generated methods
//!
//! ```rust,ignore
//! impl User {
//!     pub fn table_name() -> &'static str { "users" }
//!     pub fn hidden_fields() -> &'static [&'static str] { &["password", "remember_token"] }
//!     pub fn fillable_fields() -> &'static [&'static str] { &["name", "email", "password"] }
//!     pub fn guarded_fields() -> &'static [&'static str] { &["id"] }
//!     pub fn uses_timestamps() -> bool { true }
//!     pub fn uses_soft_delete() -> bool { false }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, DeriveInput, Lit, LitStr, Meta, Token,
    parse::ParseStream,
    punctuated::Punctuated,
};

// ---------------------------------------------------------------------------
// Internal data model
// ---------------------------------------------------------------------------

/// All metadata collected from `#[rf(...)]` attributes on a struct.
#[derive(Default)]
struct RfModelAttrs {
    table: Option<String>,
    hidden: Vec<String>,
    fillable: Vec<String>,
    guarded: Vec<String>,
    timestamps: bool,
    soft_delete: bool,
}

// ---------------------------------------------------------------------------
// Attribute parsing helpers
// ---------------------------------------------------------------------------

/// Parse a `#[rf(...)]` attribute and merge it into `attrs`.
fn parse_rf_attr(attr: &Attribute, attrs: &mut RfModelAttrs) -> syn::Result<()> {
    // Only handle `#[rf(...)]`.
    if !attr.path().is_ident("rf") {
        return Ok(());
    }

    attr.parse_args_with(|input: ParseStream| {
        // Each `#[rf(...)]` contains a single key (optionally followed by `=
        // value` or `= [...]`).
        let key: syn::Ident = input.parse()?;
        let key_str = key.to_string();

        match key_str.as_str() {
            "table" => {
                input.parse::<Token![=]>()?;
                let lit: LitStr = input.parse()?;
                attrs.table = Some(lit.value());
            }
            "hidden" => {
                input.parse::<Token![=]>()?;
                let items = parse_string_array(input)?;
                attrs.hidden.extend(items);
            }
            "fillable" => {
                input.parse::<Token![=]>()?;
                let items = parse_string_array(input)?;
                attrs.fillable.extend(items);
            }
            "guarded" => {
                input.parse::<Token![=]>()?;
                let items = parse_string_array(input)?;
                attrs.guarded.extend(items);
            }
            "timestamps" => {
                attrs.timestamps = true;
            }
            "soft_delete" => {
                attrs.soft_delete = true;
            }
            other => {
                return Err(syn::Error::new(
                    key.span(),
                    format!("Unknown rf attribute: `{other}`. Expected one of: table, hidden, fillable, guarded, timestamps, soft_delete"),
                ));
            }
        }
        Ok(())
    })
}

/// Parse a bracketed list of string literals: `["foo", "bar", "baz"]`.
fn parse_string_array(input: ParseStream) -> syn::Result<Vec<String>> {
    let content;
    syn::bracketed!(content in input);
    let items: Punctuated<LitStr, Token![,]> =
        Punctuated::parse_terminated(&content)?;
    Ok(items.into_iter().map(|s| s.value()).collect())
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

fn snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result
}

pub fn rf_model_derive_impl(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;

    let mut rf_attrs = RfModelAttrs::default();

    for attr in &ast.attrs {
        if let Err(e) = parse_rf_attr(attr, &mut rf_attrs) {
            return TokenStream::from(e.to_compile_error());
        }
    }

    // Default table name: snake_case plural of struct name.
    let table_name = rf_attrs
        .table
        .unwrap_or_else(|| format!("{}s", snake_case(&struct_name.to_string())));

    let hidden = &rf_attrs.hidden;
    let fillable = &rf_attrs.fillable;
    let guarded = &rf_attrs.guarded;
    let timestamps = rf_attrs.timestamps;
    let soft_delete = rf_attrs.soft_delete;

    let hidden_len = hidden.len();
    let fillable_len = fillable.len();
    let guarded_len = guarded.len();

    let expanded: TokenStream2 = quote! {
        impl #struct_name {
            /// Returns the database table name for this model.
            pub fn table_name() -> &'static str {
                #table_name
            }

            /// Returns the list of hidden fields (excluded from serialization).
            pub fn hidden_fields() -> &'static [&'static str] {
                static HIDDEN: [&str; #hidden_len] = [#(#hidden),*];
                &HIDDEN
            }

            /// Returns the list of mass-assignable fields.
            pub fn fillable_fields() -> &'static [&'static str] {
                static FILLABLE: [&str; #fillable_len] = [#(#fillable),*];
                &FILLABLE
            }

            /// Returns the list of guarded (non-mass-assignable) fields.
            pub fn guarded_fields() -> &'static [&'static str] {
                static GUARDED: [&str; #guarded_len] = [#(#guarded),*];
                &GUARDED
            }

            /// Returns `true` if this model uses automatic timestamp management.
            pub fn uses_timestamps() -> bool {
                #timestamps
            }

            /// Returns `true` if this model supports soft deletes.
            pub fn uses_soft_delete() -> bool {
                #soft_delete
            }
        }
    };

    TokenStream::from(expanded)
}
