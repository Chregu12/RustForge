//! Laravel-style Model DSL macro
//!
//! Allows defining models with syntax closer to Laravel:
//!
//! ```rust,ignore
//! Model! {
//!     User {
//!         table: "users",
//!         fillable: [name: String, email: String],
//!         hidden: [password: String],
//!         // Optional fields with defaults
//!         guarded: [],
//!         timestamps: true,
//!     }
//! }
//! ```
//!
//! This generates a struct with all the necessary traits and implementations.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, Token, Type, LitStr, LitBool,
};

/// A single field definition: `name: String`
struct FieldDef {
    name: Ident,
    ty: Type,
}

impl Parse for FieldDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        Ok(FieldDef { name, ty })
    }
}

/// Model definition with Laravel-style attributes
struct ModelDef {
    name: Ident,
    table: Option<String>,
    fillable: Vec<FieldDef>,
    hidden: Vec<Ident>,
    guarded: Vec<Ident>,
    timestamps: bool,
}

impl Parse for ModelDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse model name
        let name: Ident = input.parse()?;

        // Parse body
        let content;
        braced!(content in input);

        let mut table = None;
        let mut fillable = Vec::new();
        let mut hidden = Vec::new();
        let mut guarded = Vec::new();
        let mut timestamps = true;

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            content.parse::<Token![:]>()?;

            match key.to_string().as_str() {
                "table" => {
                    let lit: LitStr = content.parse()?;
                    table = Some(lit.value());
                }
                "fillable" => {
                    let fields_content;
                    bracketed!(fields_content in content);
                    let fields: Punctuated<FieldDef, Token![,]> =
                        Punctuated::parse_terminated(&fields_content)?;
                    fillable = fields.into_iter().collect();
                }
                "hidden" => {
                    let fields_content;
                    bracketed!(fields_content in content);
                    let fields: Punctuated<FieldDef, Token![,]> =
                        Punctuated::parse_terminated(&fields_content)?;
                    // For hidden, we only care about the field names
                    hidden = fields.into_iter().map(|f| f.name).collect();
                    // Also add to fillable (hidden fields are still fields)
                    // We need to re-parse... let's handle this differently
                }
                "guarded" => {
                    let fields_content;
                    bracketed!(fields_content in content);
                    let fields: Punctuated<Ident, Token![,]> =
                        Punctuated::parse_terminated(&fields_content)?;
                    guarded = fields.into_iter().collect();
                }
                "timestamps" => {
                    let lit: LitBool = content.parse()?;
                    timestamps = lit.value();
                }
                _ => {
                    return Err(syn::Error::new(key.span(), format!("Unknown attribute: {}", key)));
                }
            }

            // Optional comma
            let _ = content.parse::<Token![,]>();
        }

        Ok(ModelDef {
            name,
            table,
            fillable,
            hidden,
            guarded,
            timestamps,
        })
    }
}

pub fn model_dsl_impl(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as ModelDef);

    let name = &model.name;
    let table_name = model.table.unwrap_or_else(|| {
        // Convert CamelCase to snake_case and pluralize
        let s = name.to_string();
        let snake = to_snake_case(&s);
        format!("{}s", snake)
    });

    // Generate field definitions
    let field_defs: Vec<_> = model.fillable.iter().map(|f| {
        let fname = &f.name;
        let ftype = &f.ty;
        let is_hidden = model.hidden.contains(fname);

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

    // Generate fillable list for runtime
    let fillable_names: Vec<_> = model.fillable.iter()
        .map(|f| f.name.to_string())
        .collect();

    let hidden_names: Vec<_> = model.hidden.iter()
        .map(|h| h.to_string())
        .collect();

    // Add timestamp fields if enabled
    let timestamp_fields = if model.timestamps {
        quote! {
            pub created_at: Option<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
        }
    } else {
        quote! {}
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
            /// List of fillable fields
            pub const FILLABLE: &'static [&'static str] = &[#(#fillable_names),*];

            /// List of hidden fields (not serialized)
            pub const HIDDEN: &'static [&'static str] = &[#(#hidden_names),*];

            /// Create a new instance with default values
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    id: None,
                    #(#field_defs: Default::default(),)*
                    created_at: None,
                    updated_at: None,
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
