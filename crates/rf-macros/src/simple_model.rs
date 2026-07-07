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
    syn::custom_keyword!(validated);
    syn::custom_keyword!(email);
    syn::custom_keyword!(url);
    syn::custom_keyword!(uuid);
    syn::custom_keyword!(ip);
    syn::custom_keyword!(max);
    syn::custom_keyword!(min);
    syn::custom_keyword!(range);
    syn::custom_keyword!(regex);
}

/// An explicit validation override attached to a field via the `@` DSL, e.g.
/// `title: String @ min(3) max(200)` or `email: String @ email`. These augment
/// the type-inferred rules with real presence/length/email checks that serde's
/// deserialization alone cannot enforce.
#[derive(Debug, Clone)]
enum FieldOverride {
    /// `@ email` — value must be a syntactically valid email address.
    Email,
    /// `@ url` — value must be a syntactically valid URL (String fields only).
    Url,
    /// `@ uuid` — value must be a syntactically valid UUID (String fields only).
    Uuid,
    /// `@ ip` — value must be a valid IPv4 or IPv6 address (String fields only).
    Ip,
    /// `@ max(N)` — string length must be `<= N`.
    Max(usize),
    /// `@ min(N)` — string length must be `>= N`.
    Min(usize),
    /// `@ range(min, max)` — numeric value must satisfy `min <= value <= max`
    /// (numeric `iN`/`uN`/`fN` fields only). Bounds are stored as `f64`.
    Range(f64, f64),
    /// `@ regex("pattern")` — value must match the given regex pattern
    /// (String fields only). Reuses the real `rf_validation` regex validator.
    Regex(String),
}

/// A field: `name: String`, `hidden password: String`, or with validation
/// overrides `name: String @ min(1) max(120) email`.
struct SimpleField {
    hidden: bool,
    name: Ident,
    ty: Type,
    overrides: Vec<FieldOverride>,
}

/// Parse a single numeric bound for `@ range(min, max)`, accepting an optional
/// leading `-` and either an integer or float literal, normalised to `f64`.
fn parse_f64_bound(input: ParseStream) -> syn::Result<f64> {
    let negative = if input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        true
    } else {
        false
    };
    let value: f64 = if input.peek(syn::LitFloat) {
        input.parse::<syn::LitFloat>()?.base10_parse()?
    } else {
        input.parse::<syn::LitInt>()?.base10_parse::<i64>()? as f64
    };
    Ok(if negative { -value } else { value })
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

        // Optional trailing validation overrides: `@ email min(3) max(200)`.
        // `@` cannot appear inside a type path, so the preceding `Type` parse
        // stops right before it — keeping this fully backward-compatible with
        // plain `name: Type` fields.
        let mut overrides = Vec::new();
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            loop {
                if input.peek(kw::email) {
                    input.parse::<kw::email>()?;
                    overrides.push(FieldOverride::Email);
                } else if input.peek(kw::url) {
                    input.parse::<kw::url>()?;
                    overrides.push(FieldOverride::Url);
                } else if input.peek(kw::uuid) {
                    input.parse::<kw::uuid>()?;
                    overrides.push(FieldOverride::Uuid);
                } else if input.peek(kw::ip) {
                    input.parse::<kw::ip>()?;
                    overrides.push(FieldOverride::Ip);
                } else if input.peek(kw::range) {
                    input.parse::<kw::range>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let min = parse_f64_bound(&content)?;
                    content.parse::<Token![,]>()?;
                    let max = parse_f64_bound(&content)?;
                    overrides.push(FieldOverride::Range(min, max));
                } else if input.peek(kw::regex) {
                    input.parse::<kw::regex>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let lit: syn::LitStr = content.parse()?;
                    overrides.push(FieldOverride::Regex(lit.value()));
                } else if input.peek(kw::max) {
                    input.parse::<kw::max>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let lit: syn::LitInt = content.parse()?;
                    overrides.push(FieldOverride::Max(lit.base10_parse()?));
                } else if input.peek(kw::min) {
                    input.parse::<kw::min>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let lit: syn::LitInt = content.parse()?;
                    overrides.push(FieldOverride::Min(lit.base10_parse()?));
                } else {
                    break;
                }
            }
        }

        Ok(SimpleField { hidden, name, ty, overrides })
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
        validated: bool,
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
            let mut validated = false;

            while !content.is_empty() {
                // Check for the `validated` opt-in marker. When present, the
                // generated Create/Update DTOs also get a real
                // `rf_validation::Validate` impl (drives the ValidatedJson
                // extractor). Opt-in keeps existing Model! users that don't
                // depend on rf_validation compiling unchanged.
                if content.peek(kw::validated) {
                    content.parse::<kw::validated>()?;
                    validated = true;
                    let _ = content.parse::<Token![,]>();
                    continue;
                }

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

            Ok(ModelDef::Full { name, table, fields, relationships, timestamps, soft_deletes, validated })
        }
    }
}

pub fn simple_model_impl(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as ModelDef);

    match model {
        ModelDef::Full { name, table, fields, relationships, timestamps, soft_deletes, validated } => {
            generate_full_model(name, table, fields, relationships, timestamps, soft_deletes, validated)
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
    validated: bool,
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

    // ---- Companion DTO `Validate` impls (opt-in) -------------------------
    // When the model opts in (the `validated` marker OR any field carries an
    // `@`-override), also emit a REAL `rf_validation::Validate` (the external
    // `validator` crate trait that the `ValidatedJson` extractor requires) for
    // both DTOs. This closes the loop: `ValidatedJson<Create<Name>>` now
    // deserializes AND validates in one extractor, no manual `validate!()`.
    //
    // Type keywords (string/integer/numeric) are already guaranteed by the
    // strongly-typed DTO fields + serde deserialization, so the emitted impl
    // adds the checks serde CANNOT express: presence (non-empty required
    // strings) plus the explicit `@ email / min(N) / max(N)` overrides.
    let emit_validate = validated || fields.iter().any(|f| !f.overrides.is_empty());
    let dto_validate_impls = if emit_validate {
        // Create DTO: fields verbatim (Option<T> stays optional).
        let create_checks: Vec<TokenStream2> = fields.iter().map(|f| {
            let is_opt = is_option_type(&f.ty);
            let (kw, _) = infer_field_rule(&f.ty);
            dto_field_checks(&f.name, is_opt, &kw, &f.overrides)
        }).collect();
        // Update DTO: every field Option-wrapped -> all optional.
        let update_checks: Vec<TokenStream2> = fields.iter().map(|f| {
            let (kw, _) = infer_field_rule(&f.ty);
            dto_field_checks(&f.name, true, &kw, &f.overrides)
        }).collect();

        quote! {
            #[automatically_derived]
            impl rf_validation::Validate for #create_name {
                fn validate(&self) -> ::std::result::Result<(), rf_validation::ext_validator::ValidationErrors> {
                    let mut errors = rf_validation::ext_validator::ValidationErrors::new();
                    #(#create_checks)*
                    if errors.is_empty() { ::std::result::Result::Ok(()) } else { ::std::result::Result::Err(errors) }
                }
            }

            #[automatically_derived]
            impl rf_validation::Validate for #update_name {
                fn validate(&self) -> ::std::result::Result<(), rf_validation::ext_validator::ValidationErrors> {
                    let mut errors = rf_validation::ext_validator::ValidationErrors::new();
                    #(#update_checks)*
                    if errors.is_empty() { ::std::result::Result::Ok(()) } else { ::std::result::Result::Err(errors) }
                }
            }
        }
    } else {
        quote! {}
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

    // ---- Eager singular-relation FIELDS (belongsTo / hasOne) -----------------
    // The flagship vision line: `post.user` is a *populated struct field*, not a
    // method call. For each singular `belongsTo`/`hasOne` relation we emit an
    // `Option<Related>` field on the struct (skipped when `None` so JSON stays
    // clean and `Default` stays derivable) plus a typed, N+1-free batch loader
    // (`load_<name>_for`) generated further below alongside the plural accessors.
    let model_name_lower = to_snake_case(&name.to_string());

    let relation_field_defs: Vec<TokenStream2> = relationships.iter().filter_map(|rel| {
        let field_name = &rel.name;
        let related_type = &rel.related;
        match rel.rel_type {
            RelationType::BelongsTo | RelationType::HasOne => Some(quote! {
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub #field_name: ::std::option::Option<#related_type>
            }),
            // Eager plural relation FIELD: a populated `Vec<Child>` (default empty
            // vec, `serde(default)` so it deserializes from rows lacking the key
            // AND keeps `Default` derivable). Populated by the `load_<name>_for`
            // grouping batch loader emitted below.
            RelationType::HasMany => Some(quote! {
                #[serde(default)]
                pub #field_name: ::std::vec::Vec<#related_type>
            }),
            _ => None,
        }
    }).collect();

    let relation_field_defaults: Vec<TokenStream2> = relationships.iter().filter_map(|rel| {
        let field_name = &rel.name;
        match rel.rel_type {
            RelationType::BelongsTo | RelationType::HasOne => Some(quote! {
                #field_name: ::std::option::Option::None
            }),
            RelationType::HasMany => Some(quote! {
                #field_name: ::std::vec::Vec::new()
            }),
            _ => None,
        }
    }).collect();

    // Generate relationship methods (plural accessors) + singular batch loaders.
    let relationship_methods: Vec<TokenStream2> = relationships.iter().map(|rel| {
        let method_name = &rel.name;
        let related_type = &rel.related;
        let related_table = format!("{}s", to_snake_case(&related_type.to_string()));
        let foreign_key = rel.foreign_key.clone()
            .unwrap_or_else(|| format!("{}_id", model_name_lower));

        match rel.rel_type {
            RelationType::HasMany => {
                // hasMany: the FK lives on the CHILD row (`<this_model>_id`, e.g.
                // `post.user_id`). The eager `#method_name` Vec field is populated
                // by this batched loader: collect the distinct parent `id`s across
                // the slice, run ONE `WHERE <child_fk> IN (parent ids)` query,
                // GROUP the children into a map keyed by that FK, and assign each
                // parent row its vector of children (empty vec when none).
                let loader = syn::Ident::new(&format!("load_{}_for", method_name), method_name.span());
                let hasmany_fk = foreign_key.clone();
                quote! {
                    /// Eagerly populate the `#method_name` list field on a whole
                    /// slice with ONE batched query (no N+1). hasMany: collects the
                    /// parent ids, fetches all children whose foreign key is in that
                    /// set, groups them by FK, and assigns each parent its Vec
                    /// (empty when the parent has no children).
                    pub async fn #loader(rows: &mut [Self]) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        // Distinct parent ids across the slice.
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            if let Some(id) = row.id {
                                if !ids.contains(&id) { ids.push(id); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        // ONE raw parametrized query for the whole slice (real
                        // rusqlite-backed manager): raw expanded placeholders
                        // (`?, ?, ...`) — NOT a single-array placeholder rusqlite
                        // rejects. SELECT ... WHERE <child_fk> IN (?, ?, ...).
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!("SELECT * FROM {} WHERE {} IN ({})", #related_table, #hasmany_fk, placeholders);
                        let bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        let children = rf_db_facade::DB::select(&sql, &bindings)?;
                        // Group children by their foreign key value.
                        let mut by_key: HashMap<i64, Vec<#related_type>> = HashMap::new();
                        for child in children {
                            if let Some(k) = child.get(#hasmany_fk).and_then(|v| v.as_i64()) {
                                let model: #related_type = ::serde_json::from_value(child)
                                    .map_err(|e| e.to_string())?;
                                by_key.entry(k).or_default().push(model);
                            }
                        }
                        for row in rows.iter_mut() {
                            row.#method_name = row.id
                                .and_then(|id| by_key.get(&id).cloned())
                                .unwrap_or_default();
                        }
                        Ok(())
                    }
                }
            }
            RelationType::HasOne => {
                // hasOne: the FK lives on the CHILD row (`<this_model>_id`). The
                // eager `#method_name` field is populated by this batched loader:
                // ONE `WHERE <fk> IN (parent ids)` query, grouped back by FK.
                let loader = syn::Ident::new(&format!("load_{}_for", method_name), method_name.span());
                let hasone_fk = rel.foreign_key.clone()
                    .unwrap_or_else(|| format!("{}_id", model_name_lower));
                quote! {
                    /// Eagerly populate the `#method_name` field on a whole slice
                    /// with ONE batched query (no N+1). hasOne: matches children
                    /// whose foreign key equals each parent's `id`.
                    pub async fn #loader(rows: &mut [Self]) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            if let Some(id) = row.id {
                                if !ids.contains(&id) { ids.push(id); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        // ONE raw parametrized query for the whole slice (real
                        // rusqlite-backed manager): SELECT ... WHERE fk IN (?, ?, ...).
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!("SELECT * FROM {} WHERE {} IN ({})", #related_table, #hasone_fk, placeholders);
                        let bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        let children = rf_db_facade::DB::select(&sql, &bindings)?;
                        let mut by_key: HashMap<i64, #related_type> = HashMap::new();
                        for child in children {
                            if let Some(k) = child.get(#hasone_fk).and_then(|v| v.as_i64()) {
                                let model: #related_type = ::serde_json::from_value(child)
                                    .map_err(|e| e.to_string())?;
                                by_key.entry(k).or_insert(model);
                            }
                        }
                        for row in rows.iter_mut() {
                            if let Some(id) = row.id {
                                row.#method_name = by_key.get(&id).cloned();
                            }
                        }
                        Ok(())
                    }
                }
            }
            RelationType::BelongsTo => {
                // belongsTo: the FK lives on SELF (`<relation_name>_id`, e.g.
                // `post.user_id`). The eager `#method_name` field is populated by
                // this batched loader: collect the distinct FK values across the
                // slice, run ONE `WHERE id IN (...)` query against the parent
                // table, group by parent `id`, and assign each row's field.
                let loader = syn::Ident::new(&format!("load_{}_for", method_name), method_name.span());
                let fk = rel.foreign_key.clone()
                    .unwrap_or_else(|| format!("{}_id", to_snake_case(&method_name.to_string())));
                quote! {
                    /// Eagerly populate the `#method_name` field on a whole slice
                    /// with ONE batched query (no N+1). belongsTo: reads each
                    /// row's foreign key and resolves the parent by its `id`.
                    pub async fn #loader(rows: &mut [Self]) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        // Collect distinct FK values (read via serde so the loader
                        // works whether the FK field is `i64` or `Option<i64>`).
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            let v = ::serde_json::to_value(&*row).map_err(|e| e.to_string())?;
                            if let Some(fk) = v.get(#fk).and_then(|x| x.as_i64()) {
                                if !ids.contains(&fk) { ids.push(fk); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        // ONE raw parametrized query for the whole slice (real
                        // rusqlite-backed manager): SELECT ... WHERE id IN (?, ?, ...).
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!("SELECT * FROM {} WHERE id IN ({})", #related_table, placeholders);
                        let bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        let parents = rf_db_facade::DB::select(&sql, &bindings)?;
                        let mut by_id: HashMap<i64, #related_type> = HashMap::new();
                        for parent in parents {
                            if let Some(id) = parent.get("id").and_then(|v| v.as_i64()) {
                                let model: #related_type = ::serde_json::from_value(parent)
                                    .map_err(|e| e.to_string())?;
                                by_id.insert(id, model);
                            }
                        }
                        for row in rows.iter_mut() {
                            let v = ::serde_json::to_value(&*row).map_err(|e| e.to_string())?;
                            let fk = v.get(#fk).and_then(|x| x.as_i64());
                            row.#method_name = fk.and_then(|k| by_id.get(&k).cloned());
                        }
                        Ok(())
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

    // ---- One-call multi-relation eager hydration (Laravel `with(...)`) -------
    // The macro knows EVERY declared relation name and its generated loader
    // ident at expansion time, so `with_relations` is a plain compile-time match
    // over the known names dispatching to the SAME per-relation batch loaders
    // (`load_<name>_for`) proved elsewhere — hydrating K relations for N rows is
    // K queries total (each loader runs its own ONE batched WHERE IN query),
    // never N+1. Only relations that actually generate a field-populating loader
    // (belongsTo / hasOne / hasMany) are dispatchable; `belongsToMany` stays
    // method-only and is intentionally excluded (so it falls through to the
    // unknown-name arm). Unknown / non-eager names are a clean error naming the
    // offending relation, matching Laravel's "call to undefined relationship".
    let with_relations_arms: Vec<TokenStream2> = relationships.iter().filter_map(|rel| {
        match rel.rel_type {
            RelationType::BelongsTo | RelationType::HasOne | RelationType::HasMany => {
                let name_str = rel.name.to_string();
                let loader = syn::Ident::new(&format!("load_{}_for", rel.name), rel.name.span());
                Some(quote! {
                    #name_str => { Self::#loader(rows).await?; }
                })
            }
            RelationType::BelongsToMany => None,
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
            #(#relation_field_defs,)*
            #timestamp_fields
            #soft_delete_field
        }

        impl Default for #name {
            fn default() -> Self {
                Self {
                    id: None,
                    #(#field_defaults,)*
                    #(#relation_field_defaults,)*
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

            /// Eagerly hydrate MANY relations on a whole slice with ONE call
            /// (Laravel's `Model::with(...)`). For each requested name this
            /// dispatches — via a compile-time match the macro emits over the
            /// model's declared relations — to that relation's generated
            /// `load_<name>_for` batch loader, so hydrating K relations for N
            /// rows costs K batched queries total (each loader runs its own
            /// single `WHERE ... IN (...)` query): never N+1. Only relations
            /// with a generated field-populating loader (`belongsTo` / `hasOne`
            /// / `hasMany`) are eager-loadable; `belongsToMany` stays
            /// method-only. An unknown or non-eager name is a clean `Err`
            /// naming it (like Laravel's undefined-relationship error) and no
            /// queries are run past the offending name.
            pub async fn with_relations(
                rows: &mut [Self],
                names: &[&str],
            ) -> ::std::result::Result<(), String> {
                for name in names {
                    match *name {
                        #(#with_relations_arms)*
                        other => {
                            return ::std::result::Result::Err(::std::format!(
                                "with_relations: unknown or non-eager relation `{}` on {}",
                                other,
                                #table_name,
                            ));
                        }
                    }
                }
                ::std::result::Result::Ok(())
            }

            #soft_delete_methods
        }

        #dto_defs

        #dto_validate_impls
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

/// Emit the `validator::Validate` check statements for a single DTO field.
///
/// `is_option` selects the access pattern: `Option<T>` fields only validate
/// when `Some` (partial-update semantics), while non-optional string fields
/// additionally get a presence (non-empty) check — the piece serde cannot
/// enforce. `keyword` is the inferred type keyword ("string" gets length/email
/// treatment); `overrides` are the explicit `@ email / min / max` rules. All
/// paths resolve through `rf_validation::…` so the consuming crate only needs an
/// `rf_validation` dependency.
fn dto_field_checks(
    name: &Ident,
    is_option: bool,
    keyword: &str,
    overrides: &[FieldOverride],
) -> TokenStream2 {
    let name_str = name.to_string();
    let is_string = keyword == "string";
    let is_numeric = keyword == "integer" || keyword == "numeric";

    // Checks that operate on a bound `value` (`&String` in scope).
    let mut inner: Vec<TokenStream2> = Vec::new();
    for ov in overrides {
        match ov {
            FieldOverride::Max(n) if is_string => {
                let n = *n;
                let msg = format!("The {} field must be at most {} characters.", name_str, n);
                inner.push(quote! {
                    if value.len() > #n {
                        let mut error = rf_validation::ext_validator::ValidationError::new("length");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            FieldOverride::Min(n) if is_string => {
                let n = *n;
                let msg = format!("The {} field must be at least {} characters.", name_str, n);
                inner.push(quote! {
                    if value.len() < #n {
                        let mut error = rf_validation::ext_validator::ValidationError::new("length");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            FieldOverride::Email => {
                let msg = format!("The {} field must be a valid email address.", name_str);
                inner.push(quote! {
                    if !rf_validation::validators::email::validate_email(value) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("email");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ url` parallels `@ email`, reusing the real rf_validation url
            // validator. Only meaningful on String fields (`value` is `&String`).
            FieldOverride::Url if is_string => {
                let msg = format!("The {} field must be a valid URL.", name_str);
                inner.push(quote! {
                    if !rf_validation::validators::url::validate_url(value) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("url");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ uuid` parallels `@ url`, reusing the real rf_validation uuid
            // validator. Only meaningful on String fields (`value` is `&String`).
            FieldOverride::Uuid if is_string => {
                let msg = format!("The {} field must be a valid UUID.", name_str);
                inner.push(quote! {
                    if !rf_validation::validators::uuid::validate_uuid(value) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("uuid");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ ip` parallels `@ url`, reusing the real rf_validation ip
            // validator (accepts IPv4 OR IPv6). String fields only.
            FieldOverride::Ip if is_string => {
                let msg = format!("The {} field must be a valid IP address.", name_str);
                inner.push(quote! {
                    if !rf_validation::validators::ip::validate_ip(value) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("ip");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ range(min, max)` — numeric between check on `iN`/`uN`/`fN`
            // fields. `value` is `&T`; cast the dereferenced numeric to `f64` and
            // bound-check it (closes the run-9 numeric-range gap).
            FieldOverride::Range(min, max) if is_numeric => {
                let min = *min;
                let max = *max;
                let msg = format!(
                    "The {} field must be between {} and {}.",
                    name_str, min, max
                );
                inner.push(quote! {
                    let __rf_range_value = *value as f64;
                    if __rf_range_value < #min || __rf_range_value > #max {
                        let mut error = rf_validation::ext_validator::ValidationError::new("range");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ regex("pattern")` parallels `@ url`, reusing the real
            // rf_validation regex validator. String fields only (`value` is
            // `&String`). An invalid pattern falls back to never-match.
            FieldOverride::Regex(pattern) if is_string => {
                let msg = format!(
                    "The {} field must match the pattern {}.",
                    name_str, pattern
                );
                inner.push(quote! {
                    if !rf_validation::validators::regex::validate_regex(value, #pattern) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("regex");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // Any override that does not apply to this field's kind (e.g. a
            // string-only rule on a numeric field) is ignored; the inferred type
            // rule still applies via serde.
            _ => {}
        }
    }

    if is_option {
        if inner.is_empty() {
            return quote! {};
        }
        quote! {
            if let ::std::option::Option::Some(ref value) = self.#name {
                #(#inner)*
            }
        }
    } else {
        // Presence: a required, non-optional string must not be empty.
        let presence = if is_string {
            let msg = format!("The {} field is required.", name_str);
            quote! {
                if self.#name.is_empty() {
                    let mut error = rf_validation::ext_validator::ValidationError::new("required");
                    error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                    errors.add(#name_str, error);
                }
            }
        } else {
            quote! {}
        };

        if inner.is_empty() {
            presence
        } else {
            quote! {
                #presence
                {
                    let value = &self.#name;
                    #(#inner)*
                }
            }
        }
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
