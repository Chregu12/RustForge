//! Ultra-simple Model macro with Laravel-style relationships
//!
//! Define models with minimal syntax:
//!
//! ```rust,ignore
//! Model!(User {
//!     name: String,
//!     email: String,
//!     hidden password: String,
//!
//!     // Relationships - Laravel style!
//!     hasMany posts: Post,
//!     hasOne profile: Profile,
//! });
//!
//! Model!(Post {
//!     title: String,
//!     body: String,
//!     user_id: i64,
//!
//!     belongsTo user: User,
//!     hasMany comments: Comment,
//!     belongsToMany tags: Tag,
//! });
//!
//! // Or even simpler for basic models:
//! Model!(Comment: body, user_id, post_id);
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    braced, parse::{Parse, ParseStream}, parse_macro_input,
    punctuated::Punctuated, Ident, Token, Type,
};

mod kw {
    syn::custom_keyword!(hidden);
    syn::custom_keyword!(table);
    syn::custom_keyword!(hasMany);
    syn::custom_keyword!(hasOne);
    syn::custom_keyword!(belongsTo);
    syn::custom_keyword!(belongsToMany);
    syn::custom_keyword!(timestamps);
    syn::custom_keyword!(softDeletes);
}

/// A field: `name: String` or `hidden password: String`
struct SimpleField {
    hidden: bool,
    name: Ident,
    ty: Type,
}

impl Parse for SimpleField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let hidden = input.peek(kw::hidden);
        if hidden {
            input.parse::<kw::hidden>()?;
        }

        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;

        Ok(SimpleField { hidden, name, ty })
    }
}

/// Laravel-style relationship types
#[derive(Debug, Clone)]
enum RelationType {
    HasMany,
    HasOne,
    BelongsTo,
    BelongsToMany,
}

/// A relationship definition: `hasMany posts: Post`
struct Relationship {
    rel_type: RelationType,
    name: Ident,
    related: Ident,
    foreign_key: Option<String>,
    pivot_table: Option<String>,
}

impl Relationship {
    fn parse_if_relationship(input: ParseStream) -> Option<syn::Result<Self>> {
        let rel_type = if input.peek(kw::hasMany) {
            input.parse::<kw::hasMany>().ok()?;
            Some(RelationType::HasMany)
        } else if input.peek(kw::hasOne) {
            input.parse::<kw::hasOne>().ok()?;
            Some(RelationType::HasOne)
        } else if input.peek(kw::belongsTo) {
            input.parse::<kw::belongsTo>().ok()?;
            Some(RelationType::BelongsTo)
        } else if input.peek(kw::belongsToMany) {
            input.parse::<kw::belongsToMany>().ok()?;
            Some(RelationType::BelongsToMany)
        } else {
            None
        }?;

        Some((|| {
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let related: Ident = input.parse()?;

            Ok(Relationship {
                rel_type,
                name,
                related,
                foreign_key: None,
                pivot_table: None,
            })
        })())
    }
}

/// Simple field without type (defaults to String): `name, email, password`
struct InferredField {
    hidden: bool,
    name: Ident,
}

impl Parse for InferredField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let hidden = input.peek(kw::hidden);
        if hidden {
            input.parse::<kw::hidden>()?;
        }
        let name: Ident = input.parse()?;
        Ok(InferredField { hidden, name })
    }
}

/// Model definition: `User { name: String, ... }` or `User: name, email`
enum ModelDef {
    Full {
        name: Ident,
        table: Option<String>,
        fields: Vec<SimpleField>,
        relationships: Vec<Relationship>,
        timestamps: bool,
        soft_deletes: bool,
    },
    Simple {
        name: Ident,
        fields: Vec<InferredField>,
    },
}

impl Parse for ModelDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        if input.peek(Token![:]) && !input.peek2(Token![:]) {
            // Simple syntax: Model!(User: name, email)
            input.parse::<Token![:]>()?;
            let fields: Punctuated<InferredField, Token![,]> =
                Punctuated::parse_terminated(input)?;
            Ok(ModelDef::Simple {
                name,
                fields: fields.into_iter().collect(),
            })
        } else {
            // Full syntax: Model!(User { name: String, ... })
            let content;
            braced!(content in input);

            let mut table = None;
            let mut fields = Vec::new();
            let mut relationships = Vec::new();
            let mut timestamps = true; // Default to true like Laravel
            let mut soft_deletes = false;

            while !content.is_empty() {
                // Check for table = "..."
                if content.peek(kw::table) {
                    content.parse::<kw::table>()?;
                    content.parse::<Token![=]>()?;
                    let lit: syn::LitStr = content.parse()?;
                    table = Some(lit.value());
                    let _ = content.parse::<Token![,]>();
                    continue;
                }

                // Check for timestamps = false
                if content.peek(kw::timestamps) {
                    content.parse::<kw::timestamps>()?;
                    content.parse::<Token![=]>()?;
                    let lit: syn::LitBool = content.parse()?;
                    timestamps = lit.value();
                    let _ = content.parse::<Token![,]>();
                    continue;
                }

                // Check for softDeletes
                if content.peek(kw::softDeletes) {
                    content.parse::<kw::softDeletes>()?;
                    soft_deletes = true;
                    let _ = content.parse::<Token![,]>();
                    continue;
                }

                // Check for relationships first
                if let Some(rel_result) = Relationship::parse_if_relationship(&content) {
                    relationships.push(rel_result?);
                    if content.is_empty() {
                        break;
                    }
                    let _ = content.parse::<Token![,]>();
                    continue;
                }

                let field: SimpleField = content.parse()?;
                fields.push(field);

                if content.is_empty() {
                    break;
                }
                let _ = content.parse::<Token![,]>();
            }

            Ok(ModelDef::Full { name, table, fields, relationships, timestamps, soft_deletes })
        }
    }
}

pub fn simple_model_impl(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as ModelDef);

    match model {
        ModelDef::Full { name, table, fields, relationships, timestamps, soft_deletes } => {
            generate_full_model(name, table, fields, relationships, timestamps, soft_deletes)
        }
        ModelDef::Simple { name, fields } => {
            generate_simple_model(name, fields)
        }
    }
}

fn generate_full_model(
    name: Ident,
    table: Option<String>,
    fields: Vec<SimpleField>,
    relationships: Vec<Relationship>,
    timestamps: bool,
    soft_deletes: bool,
) -> TokenStream {
    let table_name = table.unwrap_or_else(|| {
        let s = name.to_string();
        format!("{}s", to_snake_case(&s))
    });

    let field_defs: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        let ftype = &f.ty;
        if f.hidden {
            quote! { #[serde(skip_serializing)] pub #fname: #ftype }
        } else {
            quote! { pub #fname: #ftype }
        }
    }).collect();

    let field_defaults: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        quote! { #fname: Default::default() }
    }).collect();

    let fillable: Vec<String> = fields.iter()
        .filter(|f| !f.hidden)
        .map(|f| f.name.to_string())
        .collect();

    let hidden: Vec<String> = fields.iter()
        .filter(|f| f.hidden)
        .map(|f| f.name.to_string())
        .collect();

    // Infer validation rules from declared field types (convention over
    // configuration). Every declared field (including `hidden` ones like
    // passwords) contributes a `(name, type_keyword, required)` tuple.
    let validation_rules: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = f.name.to_string();
        let (kw, required) = infer_field_rule(&f.ty);
        quote! { (#fname, #kw, #required) }
    }).collect();

    // ---- Companion request DTOs (convention over configuration) ----------
    // A single Model! declaration also emits `Create<Name>` (all declared,
    // non-`id`, non-timestamp fields as-is) and `Update<Name>` (the same fields
    // Option-wrapped for partial updates). Both carry a `VALIDATION_RULES`
    // spec + `validation_rules()` seeded from the declared field TYPES, ready to
    // feed into `rf_validation::rules_from_spec` (the REAL engine). This
    // collapses "entity + input DTO + validation" from three declarations to one.
    let create_name = syn::Ident::new(&format!("Create{}", name), name.span());
    let update_name = syn::Ident::new(&format!("Update{}", name), name.span());

    // Create-DTO fields: every declared field verbatim (hidden fields such as
    // `password` are legitimate *inputs*, so they are kept — `hidden` only
    // affects the model's serialization, not what may be submitted).
    let create_field_defs: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        let ftype = &f.ty;
        quote! { pub #fname: #ftype }
    }).collect();

    // Update-DTO fields: same names, Option-wrapped (partial update semantics).
    let update_field_defs: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        let wrapped = option_wrap(&f.ty);
        quote! { pub #fname: #wrapped }
    }).collect();

    // Create-DTO rules mirror the entity's inferred rules (required as declared).
    let create_validation_rules = validation_rules.clone();
    // Update-DTO rules: every field is optional, so requiredness is dropped but
    // the inferred TYPE keyword is preserved (still type-checked when present).
    let update_validation_rules: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = f.name.to_string();
        let (kw, _required) = infer_field_rule(&f.ty);
        quote! { (#fname, #kw, false) }
    }).collect();

    let dto_defs = quote! {
        /// Request DTO for creating a `#name` (generated from the single Model!
        /// declaration). Fields mirror the declared, non-`id` model fields.
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #create_name {
            #(#create_field_defs,)*
        }

        impl #create_name {
            /// Convention-inferred validation spec `(field, type_keyword, required)`,
            /// seeded from the model's declared field types. Feed into
            /// `rf_validation::rules_from_spec(..)` to drive the real Validator.
            pub const VALIDATION_RULES: &'static [(&'static str, &'static str, bool)] =
                &[#(#create_validation_rules),*];

            /// See [`Self::VALIDATION_RULES`].
            pub fn validation_rules() -> &'static [(&'static str, &'static str, bool)] {
                Self::VALIDATION_RULES
            }
        }

        /// Request DTO for partially updating a `#name` (generated). Every field
        /// is `Option`-wrapped so only the submitted fields are touched.
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        pub struct #update_name {
            #(#update_field_defs,)*
        }

        impl #update_name {
            /// Convention-inferred validation spec `(field, type_keyword, required)`.
            /// All fields optional (`required = false`) but still TYPE-checked when
            /// present. Feed into `rf_validation::rules_from_spec(..)`.
            pub const VALIDATION_RULES: &'static [(&'static str, &'static str, bool)] =
                &[#(#update_validation_rules),*];

            /// See [`Self::VALIDATION_RULES`].
            pub fn validation_rules() -> &'static [(&'static str, &'static str, bool)] {
                Self::VALIDATION_RULES
            }
        }
    };

    // Generate timestamp fields
    let timestamp_fields = if timestamps {
        quote! {
            pub created_at: Option<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
        }
    } else {
        quote! {}
    };

    let timestamp_defaults = if timestamps {
        quote! {
            created_at: None,
            updated_at: None,
        }
    } else {
        quote! {}
    };

    // Generate soft delete field
    let soft_delete_field = if soft_deletes {
        quote! {
            pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
        }
    } else {
        quote! {}
    };

    let soft_delete_default = if soft_deletes {
        quote! { deleted_at: None, }
    } else {
        quote! {}
    };

    // Generate relationship methods
    let relationship_methods: Vec<TokenStream2> = relationships.iter().map(|rel| {
        let method_name = &rel.name;
        let related_type = &rel.related;
        let related_table = format!("{}s", to_snake_case(&related_type.to_string()));
        let model_name_lower = to_snake_case(&name.to_string());
        let foreign_key = rel.foreign_key.clone()
            .unwrap_or_else(|| format!("{}_id", model_name_lower));

        match rel.rel_type {
            RelationType::HasMany => {
                quote! {
                    /// Get all related records (hasMany relationship)
                    pub fn #method_name(&self) -> rf_db_facade::QueryBuilder {
                        rf_db_facade::QueryBuilder::new(#related_table)
                            .r#where(#foreign_key, self.id.unwrap_or(0))
                    }
                }
            }
            RelationType::HasOne => {
                quote! {
                    /// Get the related record (hasOne relationship)
                    pub async fn #method_name(&self) -> Result<Option<serde_json::Value>, String> {
                        rf_db_facade::QueryBuilder::new(#related_table)
                            .r#where(#foreign_key, self.id.unwrap_or(0))
                            .first()
                            .await
                    }
                }
            }
            RelationType::BelongsTo => {
                let related_key = format!("{}_id", to_snake_case(&related_type.to_string()));
                quote! {
                    /// Get the parent record (belongsTo relationship)
                    pub async fn #method_name(&self) -> Result<Option<serde_json::Value>, String> {
                        // Get the foreign key field value - assuming it's named {related}_id
                        rf_db_facade::QueryBuilder::new(#related_table)
                            .find(0) // Would need to get actual FK value
                            .await
                    }
                }
            }
            RelationType::BelongsToMany => {
                let pivot_table = rel.pivot_table.clone()
                    .unwrap_or_else(|| {
                        let mut names = vec![model_name_lower.clone(), to_snake_case(&related_type.to_string())];
                        names.sort();
                        names.join("_")
                    });
                quote! {
                    /// Get all related records via pivot table (belongsToMany relationship)
                    pub fn #method_name(&self) -> rf_db_facade::QueryBuilder {
                        // In real implementation, this would do a JOIN through pivot table
                        rf_db_facade::QueryBuilder::new(#related_table)
                    }
                }
            }
        }
    }).collect();

    // Generate soft delete methods
    let soft_delete_methods = if soft_deletes {
        quote! {
            /// Soft delete this record
            pub async fn soft_delete(&mut self) -> Result<(), String> {
                self.deleted_at = Some(chrono::Utc::now());
                Ok(())
            }

            /// Restore a soft-deleted record
            pub async fn restore(&mut self) -> Result<(), String> {
                self.deleted_at = None;
                Ok(())
            }

            /// Check if record is soft-deleted
            pub fn trashed(&self) -> bool {
                self.deleted_at.is_some()
            }

            /// Query including soft-deleted records
            pub fn with_trashed() -> rf_db_facade::QueryBuilder {
                rf_db_facade::QueryBuilder::new(#table_name)
            }

            /// Query only soft-deleted records
            pub fn only_trashed() -> rf_db_facade::QueryBuilder {
                rf_db_facade::QueryBuilder::new(#table_name)
                    .whereNotNull("deleted_at")
            }

            /// Force delete (permanent)
            pub async fn force_delete(&self) -> Result<u64, String> {
                rf_db_facade::QueryBuilder::new(#table_name)
                    .r#where("id", self.id.unwrap_or(0))
                    .delete()
                    .await
            }
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
            #soft_delete_field
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    id: None,
                    #(#field_defaults,)*
                    #timestamp_defaults
                    #soft_delete_default
                }
            }
        }

        impl rf_db_facade::Model for #name {
            const TABLE: &'static str = #table_name;
        }

        impl #name {
            pub const FILLABLE: &'static [&'static str] = &[#(#fillable),*];
            pub const HIDDEN: &'static [&'static str] = &[#(#hidden),*];
            pub const TIMESTAMPS: bool = #timestamps;
            pub const SOFT_DELETES: bool = #soft_deletes;

            /// Validation rules inferred from the declared field types.
            ///
            /// Each entry is `(field_name, type_keyword, required)`:
            /// `String -> "string"`, `iN/uN -> "integer"`, `f32/f64 -> "numeric"`,
            /// `bool -> "boolean"`, `Option<T>` becomes non-required. An empty
            /// keyword means only requiredness was inferred.
            pub const VALIDATION_RULES: &'static [(&'static str, &'static str, bool)] =
                &[#(#validation_rules),*];

            /// Convention-over-configuration validation rules inferred from the
            /// model's field declarations. See [`Self::VALIDATION_RULES`].
            pub fn validation_rules() -> &'static [(&'static str, &'static str, bool)] {
                Self::VALIDATION_RULES
            }

            #(#relationship_methods)*

            #soft_delete_methods
        }

        #dto_defs
    };

    TokenStream::from(expanded)
}

fn generate_simple_model(name: Ident, fields: Vec<InferredField>) -> TokenStream {
    let table_name = format!("{}s", to_snake_case(&name.to_string()));

    let field_defs: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        if f.hidden {
            quote! { #[serde(skip_serializing)] pub #fname: String }
        } else {
            quote! { pub #fname: String }
        }
    }).collect();

    let field_defaults: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        quote! { #fname: String::new() }
    }).collect();

    let fillable: Vec<String> = fields.iter()
        .filter(|f| !f.hidden)
        .map(|f| f.name.to_string())
        .collect();

    let hidden: Vec<String> = fields.iter()
        .filter(|f| f.hidden)
        .map(|f| f.name.to_string())
        .collect();

    // Simple-syntax fields are all `String`, so every one infers a required
    // string validation rule.
    let validation_rules: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = f.name.to_string();
        quote! { (#fname, "string", true) }
    }).collect();

    // ---- Companion request DTOs (see generate_full_model) ----------------
    // Simple-syntax fields are all `String`, so Create<Name> takes `String`
    // fields and Update<Name> takes `Option<String>` fields.
    let create_name = syn::Ident::new(&format!("Create{}", name), name.span());
    let update_name = syn::Ident::new(&format!("Update{}", name), name.span());

    let create_field_defs: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        quote! { pub #fname: String }
    }).collect();

    let update_field_defs: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = &f.name;
        quote! { pub #fname: ::std::option::Option<String> }
    }).collect();

    let create_validation_rules = validation_rules.clone();
    let update_validation_rules: Vec<TokenStream2> = fields.iter().map(|f| {
        let fname = f.name.to_string();
        quote! { (#fname, "string", false) }
    }).collect();

    let dto_defs = quote! {
        /// Request DTO for creating a `#name` (generated from the single Model!
        /// declaration).
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        pub struct #create_name {
            #(#create_field_defs,)*
        }

        impl #create_name {
            /// Convention-inferred validation spec `(field, type_keyword, required)`.
            /// Feed into `rf_validation::rules_from_spec(..)` for the real Validator.
            pub const VALIDATION_RULES: &'static [(&'static str, &'static str, bool)] =
                &[#(#create_validation_rules),*];

            /// See [`Self::VALIDATION_RULES`].
            pub fn validation_rules() -> &'static [(&'static str, &'static str, bool)] {
                Self::VALIDATION_RULES
            }
        }

        /// Request DTO for partially updating a `#name` (generated); all fields
        /// `Option`-wrapped.
        #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
        pub struct #update_name {
            #(#update_field_defs,)*
        }

        impl #update_name {
            /// Convention-inferred validation spec `(field, type_keyword, required)`
            /// (all optional). Feed into `rf_validation::rules_from_spec(..)`.
            pub const VALIDATION_RULES: &'static [(&'static str, &'static str, bool)] =
                &[#(#update_validation_rules),*];

            /// See [`Self::VALIDATION_RULES`].
            pub fn validation_rules() -> &'static [(&'static str, &'static str, bool)] {
                Self::VALIDATION_RULES
            }
        }
    };

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #name {
            pub id: Option<i64>,
            #(#field_defs,)*
            pub created_at: Option<chrono::DateTime<chrono::Utc>>,
            pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    id: None,
                    #(#field_defaults,)*
                    created_at: None,
                    updated_at: None,
                }
            }
        }

        impl rf_db_facade::Model for #name {
            const TABLE: &'static str = #table_name;
        }

        impl #name {
            pub const FILLABLE: &'static [&'static str] = &[#(#fillable),*];
            pub const HIDDEN: &'static [&'static str] = &[#(#hidden),*];

            /// Validation rules inferred from the declared fields. In the simple
            /// syntax every field is a required `String`. Each entry is
            /// `(field_name, type_keyword, required)`.
            pub const VALIDATION_RULES: &'static [(&'static str, &'static str, bool)] =
                &[#(#validation_rules),*];

            /// Convention-over-configuration validation rules inferred from the
            /// model's field declarations. See [`Self::VALIDATION_RULES`].
            pub fn validation_rules() -> &'static [(&'static str, &'static str, bool)] {
                Self::VALIDATION_RULES
            }
        }

        #dto_defs
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

/// Convention-over-configuration validation inference.
///
/// Maps a declared field type to a `(type_keyword, required)` pair so the
/// generated model can expose inferred validation rules:
///   - `String` / `&str`                 -> ("string",  required)
///   - `iN` / `uN` (any width)           -> ("integer", required)
///   - `f32` / `f64`                     -> ("numeric", required)
///   - `bool`                            -> ("boolean", required)
///   - `Option<T>`                       -> keyword of `T`, but NOT required
///   - anything else                     -> ("", required)  (only requiredness)
///
/// The empty keyword means "no type rule inferred" (still required unless
/// wrapped in `Option`). Consumers translate these into `rf_validation` rules.
fn infer_field_rule(ty: &Type) -> (String, bool) {
    infer_field_rule_inner(ty, true)
}

fn infer_field_rule_inner(ty: &Type, required: bool) -> (String, bool) {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            let ident = seg.ident.to_string();
            if ident == "Option" {
                // Optional field: not required; keyword comes from the inner type.
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let (kw, _) = infer_field_rule_inner(inner, false);
                        return (kw, false);
                    }
                }
                return (String::new(), false);
            }
            return (keyword_for_ident(&ident), required);
        }
    }
    (String::new(), required)
}

/// Returns `true` if `ty` is syntactically an `Option<...>`.
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

/// Wrap `ty` in `Option<...>` for Update-DTO fields, unless it is already an
/// `Option<T>` (avoids double-wrapping declared optional fields).
fn option_wrap(ty: &Type) -> TokenStream2 {
    if is_option_type(ty) {
        quote! { #ty }
    } else {
        quote! { ::std::option::Option<#ty> }
    }
}

fn keyword_for_ident(ident: &str) -> String {
    match ident {
        "String" | "str" => "string",
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
        | "u128" | "usize" => "integer",
        "f32" | "f64" => "numeric",
        "bool" => "boolean",
        _ => "",
    }
    .to_string()
}
