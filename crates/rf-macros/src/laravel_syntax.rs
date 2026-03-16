//! Laravel-like syntax macro for RustForge
//!
//! This macro allows you to write models in a syntax very close to Laravel PHP:
//!
//! ```rust,ignore
//! laravel! {
//!     class User extends Model {
//!         protected fillable = [name: String, email: String];
//!         protected hidden = [password: String];
//!     }
//! }
//!
//! // Then use Laravel-style queries:
//! let users = User::where("active", true).get().await?;
//! let user = User::find(1).await?;
//! ```
//!
//! ## How it works
//!
//! The macro parses the PHP-like syntax and generates valid Rust code:
//! - `class User` → `pub struct User`
//! - `extends Model` → `impl Model for User`
//! - `protected fillable` → struct fields
//! - `protected hidden` → `#[serde(skip_serializing)]` attribute

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, Token, Type, LitStr,
};

/// Custom keywords for Laravel-like syntax
mod kw {
    syn::custom_keyword!(class);
    syn::custom_keyword!(extends);
    syn::custom_keyword!(Model);
    syn::custom_keyword!(protected);
    syn::custom_keyword!(fillable);
    syn::custom_keyword!(hidden);
    syn::custom_keyword!(guarded);
    syn::custom_keyword!(table);
    syn::custom_keyword!(timestamps);
    syn::custom_keyword!(casts);
}

/// A field with type: `name: String`
#[derive(Clone)]
struct TypedField {
    name: Ident,
    ty: Type,
}

impl Parse for TypedField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(TypedField { name, ty })
    }
}

/// Laravel-style class definition
struct LaravelClass {
    name: Ident,
    table: Option<String>,
    fillable: Vec<TypedField>,
    hidden: Vec<TypedField>,
    guarded: Vec<Ident>,
    timestamps: bool,
}

impl Parse for LaravelClass {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse: class User extends Model
        input.parse::<kw::class>()?;
        let name: Ident = input.parse()?;
        input.parse::<kw::extends>()?;
        input.parse::<kw::Model>()?;

        // Parse body
        let content;
        braced!(content in input);

        let mut table = None;
        let mut fillable = Vec::new();
        let mut hidden = Vec::new();
        let mut guarded = Vec::new();
        let mut timestamps = true;

        while !content.is_empty() {
            // Each line: protected fillable = [...];
            if content.peek(kw::protected) {
                content.parse::<kw::protected>()?;

                if content.peek(kw::fillable) {
                    content.parse::<kw::fillable>()?;
                    content.parse::<Token![=]>()?;

                    let fields_content;
                    bracketed!(fields_content in content);
                    let fields: Punctuated<TypedField, Token![,]> =
                        Punctuated::parse_terminated(&fields_content)?;
                    fillable = fields.into_iter().collect();
                } else if content.peek(kw::hidden) {
                    content.parse::<kw::hidden>()?;
                    content.parse::<Token![=]>()?;

                    let fields_content;
                    bracketed!(fields_content in content);
                    let fields: Punctuated<TypedField, Token![,]> =
                        Punctuated::parse_terminated(&fields_content)?;
                    hidden = fields.into_iter().collect();
                } else if content.peek(kw::guarded) {
                    content.parse::<kw::guarded>()?;
                    content.parse::<Token![=]>()?;

                    let fields_content;
                    bracketed!(fields_content in content);
                    let fields: Punctuated<Ident, Token![,]> =
                        Punctuated::parse_terminated(&fields_content)?;
                    guarded = fields.into_iter().collect();
                } else if content.peek(kw::table) {
                    content.parse::<kw::table>()?;
                    content.parse::<Token![=]>()?;
                    let lit: LitStr = content.parse()?;
                    table = Some(lit.value());
                } else if content.peek(kw::timestamps) {
                    content.parse::<kw::timestamps>()?;
                    content.parse::<Token![=]>()?;
                    let val: syn::LitBool = content.parse()?;
                    timestamps = val.value();
                } else {
                    return Err(content.error("Expected: fillable, hidden, guarded, table, or timestamps"));
                }

                // Optional semicolon
                let _ = content.parse::<Token![;]>();
            } else {
                // Skip unknown content or break
                break;
            }
        }

        Ok(LaravelClass {
            name,
            table,
            fillable,
            hidden,
            guarded,
            timestamps,
        })
    }
}

pub fn laravel_impl(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as LaravelClass);

    let name = &model.name;
    let table_name = model.table.unwrap_or_else(|| {
        // Convert CamelCase to snake_case and pluralize
        let s = name.to_string();
        let snake = to_snake_case(&s);
        format!("{}s", snake)
    });

    // Collect all fields (fillable + hidden)
    let mut all_fields: Vec<TypedField> = model.fillable.clone();
    for h in &model.hidden {
        if !all_fields.iter().any(|f| f.name == h.name) {
            all_fields.push(h.clone());
        }
    }

    // Generate field definitions
    let field_defs: Vec<TokenStream2> = all_fields.iter().map(|f| {
        let fname = &f.name;
        let ftype = &f.ty;
        let is_hidden = model.hidden.iter().any(|h| h.name == *fname);

        if is_hidden {
            quote! {
                #[serde(skip_serializing)]
                pub #fname: #ftype
            }
        } else {
            quote! {
                pub #fname: #ftype
            }
        }
    }).collect();

    // Generate default field initializers
    let field_defaults: Vec<TokenStream2> = all_fields.iter().map(|f| {
        let fname = &f.name;
        quote! { #fname: Default::default() }
    }).collect();

    // Generate fillable list for runtime
    let fillable_names: Vec<String> = model.fillable.iter()
        .map(|f| f.name.to_string())
        .collect();

    let hidden_names: Vec<String> = model.hidden.iter()
        .map(|h| h.name.to_string())
        .collect();

    // Timestamp fields
    let (timestamp_fields, timestamp_defaults) = if model.timestamps {
        (
            quote! {
                pub created_at: Option<chrono::DateTime<chrono::Utc>>,
                pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
            },
            quote! {
                created_at: None,
                updated_at: None,
            }
        )
    } else {
        (quote! {}, quote! {})
    };

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #name {
            pub id: Option<i64>,
            #(#field_defs,)*
            #timestamp_fields
        }

        impl rf_db_facade::Model for #name {
            const TABLE: &'static str = #table_name;
        }

        impl #name {
            /// List of fillable fields (mass-assignable)
            pub const FILLABLE: &'static [&'static str] = &[#(#fillable_names),*];

            /// List of hidden fields (not in JSON output)
            pub const HIDDEN: &'static [&'static str] = &[#(#hidden_names),*];

            /// Create a new instance
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    id: None,
                    #(#field_defaults,)*
                    #timestamp_defaults
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    // Tests would go here
}
