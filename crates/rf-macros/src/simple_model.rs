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
    braced, ext::IdentExt, parse::{Parse, ParseStream}, parse_macro_input,
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
    syn::custom_keyword!(guarded);
    syn::custom_keyword!(scope);
    syn::custom_keyword!(email);
    syn::custom_keyword!(url);
    syn::custom_keyword!(uuid);
    syn::custom_keyword!(ip);
    syn::custom_keyword!(max);
    syn::custom_keyword!(min);
    syn::custom_keyword!(range);
    syn::custom_keyword!(regex);
    syn::custom_keyword!(alpha);
    syn::custom_keyword!(alphanumeric);
    syn::custom_keyword!(starts_with);
    syn::custom_keyword!(ends_with);
    syn::custom_keyword!(message);
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
    /// `@ alpha` — value must contain only alphabetic characters (String fields
    /// only). Mirrors the real `rf_validation` `AlphaRule` semantics.
    Alpha,
    /// `@ alphanumeric` — value must contain only letters and digits (String
    /// fields only). Mirrors the real `rf_validation` `AlphaNumericRule`.
    AlphaNumeric,
    /// `@ starts_with("prefix")` — value must start with the given prefix
    /// (String fields only). Mirrors the real `rf_validation` `StartsWithRule`.
    StartsWith(String),
    /// `@ ends_with("suffix")` — value must end with the given suffix (String
    /// fields only). Mirrors the real `rf_validation` `EndsWithRule`.
    EndsWith(String),
    /// `@ message("Custom text")` — a per-field custom validation message that
    /// OVERRIDES the auto-generated English message of EVERY rule check emitted
    /// for this field (presence + all `@` modifier checks). Purely cosmetic:
    /// it changes the `ValidationError.message`, never which checks run or their
    /// stable `code`. Fields without it behave exactly as before.
    Message(String),
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
                } else if input.peek(kw::alpha) {
                    input.parse::<kw::alpha>()?;
                    overrides.push(FieldOverride::Alpha);
                } else if input.peek(kw::alphanumeric) {
                    input.parse::<kw::alphanumeric>()?;
                    overrides.push(FieldOverride::AlphaNumeric);
                } else if input.peek(kw::starts_with) {
                    input.parse::<kw::starts_with>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let lit: syn::LitStr = content.parse()?;
                    overrides.push(FieldOverride::StartsWith(lit.value()));
                } else if input.peek(kw::ends_with) {
                    input.parse::<kw::ends_with>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let lit: syn::LitStr = content.parse()?;
                    overrides.push(FieldOverride::EndsWith(lit.value()));
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
                } else if input.peek(kw::message) {
                    // Trailing per-field custom message: `@ ... message("text")`.
                    // Parsed as a parenthesized string literal exactly like
                    // `@ regex`/`@ starts_with`. It overrides the message of the
                    // checks emitted for this field (see `dto_field_checks`).
                    input.parse::<kw::message>()?;
                    let content;
                    syn::parenthesized!(content in input);
                    let lit: syn::LitStr = content.parse()?;
                    overrides.push(FieldOverride::Message(lit.value()));
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

/// A single chainable call inside a `scope` body, e.g. `where("status",
/// "published")` or `limit(10)`. `method` is the DSL method name (parsed with
/// `Ident::parse_any` so the Rust keyword `where` is accepted verbatim) and
/// `args` are its literal arguments. Validated + lowered to a real
/// `QueryBuilder` call in [`emit_scope_call`].
struct ScopeCall {
    method: Ident,
    args: Vec<syn::Lit>,
}

/// A Laravel-style local query scope: `scope active: where("status",
/// "published")` or a chain `scope recent: order_by("created_at", "DESC")
/// limit(10)`. Each generates an inherent associated fn on the model (e.g.
/// `Post::active()`) that builds the real `rf_db_facade::QueryBuilder` for the
/// model's table, applies the declared call-chain, and returns it — so it
/// composes with the existing chainable builder + `.get()`:
/// `Post::active().r#where("user_id", 1).get().await`.
struct Scope {
    name: Ident,
    calls: Vec<ScopeCall>,
}

impl Scope {
    /// Parse a `scope <name>: <call> <call> ...` entry. Assumes the `scope`
    /// keyword has NOT yet been consumed. The call-chain runs until a `,` or the
    /// end of the model body.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::scope>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;

        let mut calls = Vec::new();
        loop {
            if input.is_empty() || input.peek(Token![,]) {
                break;
            }
            // `where` is a reserved keyword, so parse the method name with
            // `parse_any` (accepts keywords as identifiers).
            let method = Ident::parse_any(input)?;
            let content;
            syn::parenthesized!(content in input);
            let args: Punctuated<syn::Lit, Token![,]> =
                Punctuated::parse_terminated(&content)?;
            calls.push(ScopeCall { method, args: args.into_iter().collect() });
        }

        if calls.is_empty() {
            return Err(syn::Error::new(
                name.span(),
                format!("scope `{}` has an empty body; declare at least one call, e.g. `scope {}: where(\"col\", \"val\")`", name, name),
            ));
        }

        Ok(Scope { name, calls })
    }
}

/// Lower one [`ScopeCall`] to the matching real `rf_db_facade::QueryBuilder`
/// chainable method call. ONLY the chainable methods that genuinely exist on
/// `QueryBuilder` (verified in `crates/rf-orm/src/facade/query_builder.rs`) are
/// allowed; an unknown method or a wrong argument count is a clean compile-time
/// error at the offending call's span.
fn emit_scope_call(call: &ScopeCall) -> syn::Result<TokenStream2> {
    let method = call.method.to_string();
    let args = &call.args;
    let span = call.method.span();
    let expect = |n: usize| -> syn::Result<()> {
        if args.len() != n {
            Err(syn::Error::new(
                span,
                format!("scope method `{}` expects {} argument(s), got {}", method, n, args.len()),
            ))
        } else {
            Ok(())
        }
    };
    let ts = match method.as_str() {
        // `where("col", val)` -> the real `r#where` (2-arg equality).
        "where" => { expect(2)?; let a = &args[0]; let b = &args[1]; quote! { .r#where(#a, #b) } }
        // `filter("col", val)` -> alias of `r#where`.
        "filter" => { expect(2)?; let a = &args[0]; let b = &args[1]; quote! { .filter(#a, #b) } }
        // `where_eq("col", val)` -> alias of `r#where`.
        "where_eq" => { expect(2)?; let a = &args[0]; let b = &args[1]; quote! { .where_eq(#a, #b) } }
        // `where_op("col", "op", val)` -> custom operator.
        "where_op" => { expect(3)?; let a = &args[0]; let b = &args[1]; let c = &args[2]; quote! { .where_op(#a, #b, #c) } }
        // `where_like("col", "pattern")`.
        "where_like" => { expect(2)?; let a = &args[0]; let b = &args[1]; quote! { .where_like(#a, #b) } }
        // `where_null("col")` / `where_not_null("col")`.
        "where_null" => { expect(1)?; let a = &args[0]; quote! { .where_null(#a) } }
        "where_not_null" => { expect(1)?; let a = &args[0]; quote! { .where_not_null(#a) } }
        // `order_by("col", "DIR")` + shorthands.
        "order_by" => { expect(2)?; let a = &args[0]; let b = &args[1]; quote! { .order_by(#a, #b) } }
        "order_by_asc" => { expect(1)?; let a = &args[0]; quote! { .order_by_asc(#a) } }
        "order_by_desc" => { expect(1)?; let a = &args[0]; quote! { .order_by_desc(#a) } }
        // `limit(N)` / `offset(N)` — usize literals.
        "limit" => { expect(1)?; let a = &args[0]; quote! { .limit(#a) } }
        "offset" => { expect(1)?; let a = &args[0]; quote! { .offset(#a) } }
        other => {
            return Err(syn::Error::new(
                span,
                format!(
                    "unknown scope method `{}`. Allowed chainable methods: where, filter, where_eq, where_op, where_like, where_null, where_not_null, order_by, order_by_asc, order_by_desc, limit, offset",
                    other
                ),
            ));
        }
    };
    Ok(ts)
}

/// Build the inherent associated fns for all declared scopes. Each scope becomes
/// a `pub fn <name>() -> rf_db_facade::QueryBuilder` that constructs the real
/// builder for `table_name` and applies the declared call-chain, returning the
/// builder so callers can keep chaining (`Post::active().r#where(..).get()`).
fn build_scope_methods(scopes: &[Scope], table_name: &str) -> syn::Result<Vec<TokenStream2>> {
    scopes.iter().map(|scope| {
        let sname = &scope.name;
        let sname_str = sname.to_string();
        let calls: Vec<TokenStream2> = scope
            .calls
            .iter()
            .map(emit_scope_call)
            .collect::<syn::Result<Vec<_>>>()?;
        let doc = format!(
            "Local query scope `{}` (Laravel-style). Returns the real `rf_db_facade::QueryBuilder` for this model's table with the scope's declared constraints already applied, so it composes with further chained calls and `.get().await`.",
            sname_str
        );
        Ok(quote! {
            #[doc = #doc]
            pub fn #sname() -> rf_db_facade::QueryBuilder {
                rf_db_facade::QueryBuilder::new(#table_name)
                    #(#calls)*
            }
        })
    }).collect()
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
        scopes: Vec<Scope>,
        timestamps: bool,
        soft_deletes: bool,
        validated: bool,
        guarded: bool,
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
            let mut scopes = Vec::new();
            let mut timestamps = true; // Default to true like Laravel
            let mut soft_deletes = false;
            let mut validated = false;
            let mut guarded = false;

            while !content.is_empty() {
                // Check for the `guarded` opt-in marker. When present, the
                // DEFAULT `create()` path is flipped to mass-assignment
                // enforcement: the macro emits an inherent
                // `#name::create(data)` that runs `Self::fill_guarded(data)?`
                // (rejecting any field not in MASS_ASSIGNABLE, naming it) then
                // inserts through the real QueryBuilder path directly. Because
                // `create!(Model, ...)` expands to the UFCS `Model::create(...)`
                // and `User::create(...)` call sites resolve to the INHERENT
                // method (inherent wins over the trait), this guards BOTH the
                // `create!` macro and direct `Model::create` calls. Opt-in keeps
                // every existing Model! user byte-for-byte unchanged.
                if content.peek(kw::guarded) {
                    content.parse::<kw::guarded>()?;
                    guarded = true;
                    let _ = content.parse::<Token![,]>();
                    continue;
                }

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

                // Check for a `scope <name>: <call-chain>` declaration
                // (Laravel-style local query scope). Each parsed `Scope` is
                // lowered later into an inherent `Model::<name>()` fn returning
                // the real QueryBuilder. Opt-in: models with no `scope` line are
                // unchanged.
                if content.peek(kw::scope) {
                    scopes.push(Scope::parse(&content)?);
                    if content.is_empty() {
                        break;
                    }
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

            Ok(ModelDef::Full { name, table, fields, relationships, scopes, timestamps, soft_deletes, validated, guarded })
        }
    }
}

pub fn simple_model_impl(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as ModelDef);

    match model {
        ModelDef::Full { name, table, fields, relationships, scopes, timestamps, soft_deletes, validated, guarded } => {
            generate_full_model(name, table, fields, relationships, scopes, timestamps, soft_deletes, validated, guarded)
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
    scopes: Vec<Scope>,
    timestamps: bool,
    soft_deletes: bool,
    validated: bool,
    guarded: bool,
) -> TokenStream {
    let table_name = table.unwrap_or_else(|| {
        let s = name.to_string();
        format!("{}s", to_snake_case(&s))
    });

    // ---- Declarative local query scopes (Laravel local scopes) --------------
    // Each `scope <name>: <chain>` declaration becomes an inherent
    // `Model::<name>() -> rf_db_facade::QueryBuilder` that applies the declared
    // constraints and returns the real builder, so it composes with the existing
    // chainable methods + `.get()`. Unknown methods / bad arg counts are a clean
    // compile error (surfaced here as a `compile_error!` in the caller's crate).
    let scope_methods = match build_scope_methods(&scopes, &table_name) {
        Ok(methods) => methods,
        Err(e) => return e.to_compile_error().into(),
    };

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

    // Mass-assignable (Laravel's `$fillable`): every DECLARED field, EXCLUDING
    // the framework-managed `id`, timestamps, soft-delete column and relations
    // (none of which are declared as plain fields). `hidden` fields such as
    // `password` are legitimate INPUTS (mirroring the Create DTO, which also
    // keeps them), so they stay mass-assignable — `hidden` only affects the
    // model's serialization, not what may be submitted.
    let mass_assignable: Vec<String> = fields.iter()
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
            // Eager MANY-to-many relation FIELD: a populated `Vec<Related>`
            // hydrated THROUGH the pivot table by `load_<name>_for` (below).
            // Same shape as hasMany (default empty vec, `serde(default)` so it
            // deserializes from rows lacking the key AND keeps `Default`
            // derivable) — the difference is purely how it is loaded (via the
            // pivot), not how it is stored.
            RelationType::HasMany | RelationType::BelongsToMany => Some(quote! {
                #[serde(default)]
                pub #field_name: ::std::vec::Vec<#related_type>
            }),
        }
    }).collect();

    let relation_field_defaults: Vec<TokenStream2> = relationships.iter().filter_map(|rel| {
        let field_name = &rel.name;
        match rel.rel_type {
            RelationType::BelongsTo | RelationType::HasOne => Some(quote! {
                #field_name: ::std::option::Option::None
            }),
            RelationType::HasMany | RelationType::BelongsToMany => Some(quote! {
                #field_name: ::std::vec::Vec::new()
            }),
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
                let loader_where = syn::Ident::new(&format!("load_{}_where", method_name), method_name.span());
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

                    /// Constrained sibling of `#loader` (`with_where`): same
                    /// batched hasMany load, but the single `WHERE <fk> IN (...)`
                    /// query gains a parametrized `AND <column> = <value>`, so
                    /// each parent's `#method_name` Vec holds ONLY children
                    /// matching that equality (still one batched query, no N+1).
                    pub async fn #loader_where<V: ::std::convert::Into<::serde_json::Value>>(
                        rows: &mut [Self],
                        column: &str,
                        value: V,
                    ) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        let value: ::serde_json::Value = value.into();
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            if let Some(id) = row.id {
                                if !ids.contains(&id) { ids.push(id); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        // SAME base SELECT + one extra parametrized equality; the
                        // constraint value is bound as the trailing `?`.
                        let sql = format!(
                            "SELECT * FROM {} WHERE {} IN ({}) AND {} = ?",
                            #related_table, #hasmany_fk, placeholders, column,
                        );
                        let mut bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        bindings.push(value);
                        let children = rf_db_facade::DB::select(&sql, &bindings)?;
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
                let loader_where = syn::Ident::new(&format!("load_{}_where", method_name), method_name.span());
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

                    /// Constrained sibling of `#loader` (`with_where`): same
                    /// batched hasOne load, but the `WHERE <fk> IN (...)` query
                    /// gains a parametrized `AND <column> = <value>`, so each
                    /// parent's `#method_name` is set ONLY from a matching child
                    /// (still one batched query, no N+1).
                    pub async fn #loader_where<V: ::std::convert::Into<::serde_json::Value>>(
                        rows: &mut [Self],
                        column: &str,
                        value: V,
                    ) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        let value: ::serde_json::Value = value.into();
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            if let Some(id) = row.id {
                                if !ids.contains(&id) { ids.push(id); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!(
                            "SELECT * FROM {} WHERE {} IN ({}) AND {} = ?",
                            #related_table, #hasone_fk, placeholders, column,
                        );
                        let mut bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        bindings.push(value);
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
                let loader_where = syn::Ident::new(&format!("load_{}_where", method_name), method_name.span());
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

                    /// Constrained sibling of `#loader` (`with_where`): same
                    /// batched belongsTo load, but the parent `WHERE id IN (...)`
                    /// query gains a parametrized `AND <column> = <value>` on the
                    /// parent table, so a row's `#method_name` resolves ONLY to a
                    /// matching parent (still one batched query, no N+1).
                    pub async fn #loader_where<V: ::std::convert::Into<::serde_json::Value>>(
                        rows: &mut [Self],
                        column: &str,
                        value: V,
                    ) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        let value: ::serde_json::Value = value.into();
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            let v = ::serde_json::to_value(&*row).map_err(|e| e.to_string())?;
                            if let Some(fk) = v.get(#fk).and_then(|x| x.as_i64()) {
                                if !ids.contains(&fk) { ids.push(fk); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!(
                            "SELECT * FROM {} WHERE id IN ({}) AND {} = ?",
                            #related_table, placeholders, column,
                        );
                        let mut bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        bindings.push(value);
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
                // belongsToMany: the link lives in a PIVOT table (e.g. `post_tag`)
                // carrying a self FK + a related FK. The eager `#method_name` Vec
                // field is populated by this batched loader with exactly TWO
                // queries (no N+1): (1) SELECT <self_fk>, <related_fk> FROM
                // <pivot> WHERE <self_fk> IN (parent ids) — collecting the
                // distinct related ids and a per-parent map of related ids; then
                // (2) SELECT * FROM <related_table> WHERE id IN (related ids),
                // indexed by id; finally each parent is assigned its Vec of
                // related models by mapping its pivot related-ids through that
                // index (empty vec when it has no pivot rows).
                let loader = syn::Ident::new(&format!("load_{}_for", method_name), method_name.span());
                let loader_where = syn::Ident::new(&format!("load_{}_where", method_name), method_name.span());
                // Pivot table: explicit if given, else the two model names
                // snake-cased, sorted, joined by `_` (Laravel convention).
                let pivot_table = rel.pivot_table.clone()
                    .unwrap_or_else(|| {
                        let mut names = vec![model_name_lower.clone(), to_snake_case(&related_type.to_string())];
                        names.sort();
                        names.join("_")
                    });
                // Pivot FK columns by convention: self side = <parent>_id,
                // related side = <related>_id.
                let self_fk = format!("{}_id", model_name_lower);
                let related_fk = format!("{}_id", to_snake_case(&related_type.to_string()));
                quote! {
                    /// Eagerly populate the `#method_name` list field on a whole
                    /// slice THROUGH the pivot table with exactly TWO batched
                    /// queries (no N+1): first the pivot rows for all parents,
                    /// then the related rows for all collected related ids.
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
                        // ---- Query 1: pivot rows for the whole slice ---------
                        // SELECT <self_fk>, <related_fk> FROM <pivot>
                        //   WHERE <self_fk> IN (?, ?, ...)
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!(
                            "SELECT {}, {} FROM {} WHERE {} IN ({})",
                            #self_fk, #related_fk, #pivot_table, #self_fk, placeholders
                        );
                        let bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        let pivot_rows = rf_db_facade::DB::select(&sql, &bindings)?;
                        // Per-parent list of related ids + the distinct related ids.
                        let mut parent_to_related: HashMap<i64, Vec<i64>> = HashMap::new();
                        let mut related_ids: Vec<i64> = Vec::new();
                        for pr in pivot_rows {
                            let p = pr.get(#self_fk).and_then(|v| v.as_i64());
                            let r = pr.get(#related_fk).and_then(|v| v.as_i64());
                            if let (Some(p), Some(r)) = (p, r) {
                                parent_to_related.entry(p).or_default().push(r);
                                if !related_ids.contains(&r) { related_ids.push(r); }
                            }
                        }
                        // No links at all -> every parent gets an empty vec.
                        if related_ids.is_empty() {
                            for row in rows.iter_mut() { row.#method_name = Vec::new(); }
                            return Ok(());
                        }
                        // ---- Query 2: related rows for all collected ids -----
                        // SELECT * FROM <related_table> WHERE id IN (?, ?, ...)
                        let placeholders2 = related_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql2 = format!(
                            "SELECT * FROM {} WHERE id IN ({})",
                            #related_table, placeholders2
                        );
                        let bindings2: Vec<::serde_json::Value> =
                            related_ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        let related_records = rf_db_facade::DB::select(&sql2, &bindings2)?;
                        // Index related models by id.
                        let mut by_id: HashMap<i64, #related_type> = HashMap::new();
                        for rec in related_records {
                            if let Some(id) = rec.get("id").and_then(|v| v.as_i64()) {
                                let model: #related_type = ::serde_json::from_value(rec)
                                    .map_err(|e| e.to_string())?;
                                by_id.insert(id, model);
                            }
                        }
                        // Assign each parent its vec of related models (mapping
                        // its pivot related-ids through the index; empty when none).
                        for row in rows.iter_mut() {
                            row.#method_name = row.id
                                .and_then(|id| parent_to_related.get(&id))
                                .map(|rids| {
                                    rids.iter()
                                        .filter_map(|rid| by_id.get(rid).cloned())
                                        .collect::<Vec<#related_type>>()
                                })
                                .unwrap_or_default();
                        }
                        Ok(())
                    }

                    /// Constrained sibling of `#loader` (`with_where`): identical
                    /// TWO-query pivot load, except the second query (related
                    /// rows) gains a parametrized `AND <column> = <value>`. Only
                    /// matching related rows are indexed, so each parent's
                    /// `#method_name` Vec keeps ONLY related models satisfying the
                    /// equality (still exactly two batched queries, no N+1).
                    pub async fn #loader_where<V: ::std::convert::Into<::serde_json::Value>>(
                        rows: &mut [Self],
                        column: &str,
                        value: V,
                    ) -> ::std::result::Result<(), String> {
                        use ::std::collections::HashMap;
                        let value: ::serde_json::Value = value.into();
                        let mut ids: Vec<i64> = Vec::new();
                        for row in rows.iter() {
                            if let Some(id) = row.id {
                                if !ids.contains(&id) { ids.push(id); }
                            }
                        }
                        if ids.is_empty() { return Ok(()); }
                        // ---- Query 1: pivot rows (UNCHANGED from base loader) --
                        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql = format!(
                            "SELECT {}, {} FROM {} WHERE {} IN ({})",
                            #self_fk, #related_fk, #pivot_table, #self_fk, placeholders
                        );
                        let bindings: Vec<::serde_json::Value> =
                            ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        let pivot_rows = rf_db_facade::DB::select(&sql, &bindings)?;
                        let mut parent_to_related: HashMap<i64, Vec<i64>> = HashMap::new();
                        let mut related_ids: Vec<i64> = Vec::new();
                        for pr in pivot_rows {
                            let p = pr.get(#self_fk).and_then(|v| v.as_i64());
                            let r = pr.get(#related_fk).and_then(|v| v.as_i64());
                            if let (Some(p), Some(r)) = (p, r) {
                                parent_to_related.entry(p).or_default().push(r);
                                if !related_ids.contains(&r) { related_ids.push(r); }
                            }
                        }
                        if related_ids.is_empty() {
                            for row in rows.iter_mut() { row.#method_name = Vec::new(); }
                            return Ok(());
                        }
                        // ---- Query 2: related rows + the extra equality --------
                        let placeholders2 = related_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                        let sql2 = format!(
                            "SELECT * FROM {} WHERE id IN ({}) AND {} = ?",
                            #related_table, placeholders2, column,
                        );
                        let mut bindings2: Vec<::serde_json::Value> =
                            related_ids.iter().map(|&i| ::serde_json::Value::from(i)).collect();
                        bindings2.push(value);
                        let related_records = rf_db_facade::DB::select(&sql2, &bindings2)?;
                        let mut by_id: HashMap<i64, #related_type> = HashMap::new();
                        for rec in related_records {
                            if let Some(id) = rec.get("id").and_then(|v| v.as_i64()) {
                                let model: #related_type = ::serde_json::from_value(rec)
                                    .map_err(|e| e.to_string())?;
                                by_id.insert(id, model);
                            }
                        }
                        // Related ids filtered out by the constraint simply miss
                        // the index, so `filter_map` drops them per parent.
                        for row in rows.iter_mut() {
                            row.#method_name = row.id
                                .and_then(|id| parent_to_related.get(&id))
                                .map(|rids| {
                                    rids.iter()
                                        .filter_map(|rid| by_id.get(rid).cloned())
                                        .collect::<Vec<#related_type>>()
                                })
                                .unwrap_or_default();
                        }
                        Ok(())
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
    // a bounded number of batched queries total (each direct-FK loader runs ONE
    // `WHERE IN` query, the belongsToMany loader runs TWO through the pivot),
    // never N+1. Every declared relation family (belongsTo / hasOne / hasMany /
    // belongsToMany) generates a field-populating loader, so all are
    // dispatchable. Unknown names are a clean error naming the offending
    // relation, matching Laravel's "call to undefined relationship".
    let with_relations_arms: Vec<TokenStream2> = relationships.iter().filter_map(|rel| {
        match rel.rel_type {
            // Every eager relation FAMILY now generates a field-populating
            // `load_<name>_for` batch loader (belongsTo / hasOne / hasMany
            // direct-FK, belongsToMany through the pivot), so all four are
            // dispatchable here — each still costs its own batched queries, never
            // N+1.
            RelationType::BelongsTo
            | RelationType::HasOne
            | RelationType::HasMany
            | RelationType::BelongsToMany => {
                let name_str = rel.name.to_string();
                let loader = syn::Ident::new(&format!("load_{}_for", rel.name), rel.name.span());
                Some(quote! {
                    #name_str => { Self::#loader(rows).await?; }
                })
            }
        }
    }).collect();

    // ---- Constrained eager-load dispatch (`with_where`) ----------------------
    // One match arm per declared relation, inlined into the fetch-time builder's
    // `get()`: when a requested relation carries a `with_where` constraint, hand
    // it to that relation's constrained `load_<name>_where` loader (the SAME
    // batched query + `AND column = value`), so only matching children hydrate —
    // still one batched query per relation, never N+1. Every relation family
    // (belongsTo / hasOne / hasMany / belongsToMany) generates a `_where` loader,
    // so all are dispatchable; the value is a `serde_json::Value` bound exactly
    // like the base loader's `WHERE IN` params.
    let with_where_arms: Vec<TokenStream2> = relationships.iter().map(|rel| {
        let name_str = rel.name.to_string();
        let loader_where = syn::Ident::new(&format!("load_{}_where", rel.name), rel.name.span());
        quote! {
            #name_str => {
                #name::#loader_where(&mut rows, __rf_col.as_str(), __rf_val.clone()).await?;
            }
        }
    }).collect();

    // ---- Nested dot-notation eager loads (Laravel `with("a.b")`) -------------
    // One level of nesting `a.b`: (1) load the first segment `a` with THIS
    // model's existing `load_a_for` (unchanged signature); (2) collect the
    // children hydrated into the `a` relation field across all rows into ONE
    // flat vec; (3) dispatch the remaining path `b` to the CHILD model's own
    // `with_relations` (the child type ident comes from the relation
    // declaration, and its loaders were generated by the child's own `Model!`)
    // so the child hydration is itself a bounded batched load, never N+1; then
    // (4) regroup the now-hydrated children back into each parent relation field
    // by the child's own `id`. The child call is BOXED to a `dyn Future` so the
    // (possibly cyclic) mutual recursion between models' `with_relations`
    // futures stays a finite, sized type. Because step (3) re-enters
    // `with_relations`, a deeper `rest` (e.g. `a.b.c`) is handled recursively
    // by the child level — the leading arm here only owns the FIRST segment.
    let nested_arms: Vec<TokenStream2> = relationships.iter().map(|rel| {
        let name_str = rel.name.to_string();
        let field_name = &rel.name;
        let child = &rel.related;
        let loader = syn::Ident::new(&format!("load_{}_for", rel.name), rel.name.span());
        let (collect_stmt, regroup_stmt) = match rel.rel_type {
            RelationType::HasMany | RelationType::BelongsToMany => (
                quote! { for c in row.#field_name.iter() { children.push(c.clone()); } },
                quote! {
                    for c in row.#field_name.iter_mut() {
                        if let ::std::option::Option::Some(cid) = c.id {
                            if let ::std::option::Option::Some(h) = by_id.get(&cid) {
                                *c = h.clone();
                            }
                        }
                    }
                },
            ),
            RelationType::BelongsTo | RelationType::HasOne => (
                quote! {
                    if let ::std::option::Option::Some(c) = row.#field_name.as_ref() {
                        children.push(c.clone());
                    }
                },
                quote! {
                    if let ::std::option::Option::Some(c) = row.#field_name.as_mut() {
                        if let ::std::option::Option::Some(cid) = c.id {
                            if let ::std::option::Option::Some(h) = by_id.get(&cid) {
                                *c = h.clone();
                            }
                        }
                    }
                },
            ),
        };
        quote! {
            #name_str => {
                // (1) load the first segment with THIS model's own loader.
                Self::#loader(rows).await?;
                // (2) collect the hydrated children into one flat vec.
                let mut children: ::std::vec::Vec<#child> = ::std::vec::Vec::new();
                for row in rows.iter() { #collect_stmt }
                // (3) hydrate the remaining path on the CHILD type (boxed so the
                // mutual `with_relations` recursion is a finite, sized future).
                // The one-element name slice is bound to a `let` so it outlives
                // the boxed future that borrows it.
                let rest_names = [rest];
                let fut: ::std::pin::Pin<::std::boxed::Box<
                    dyn ::std::future::Future<
                        Output = ::std::result::Result<(), ::std::string::String>
                    > + ::std::marker::Send,
                >> = ::std::boxed::Box::pin(#child::with_relations(&mut children, &rest_names));
                fut.await?;
                // (4) regroup the hydrated children back by their own id.
                let mut by_id: ::std::collections::HashMap<i64, #child> =
                    ::std::collections::HashMap::new();
                for c in children {
                    if let ::std::option::Option::Some(cid) = c.id { by_id.insert(cid, c); }
                }
                for row in rows.iter_mut() { #regroup_stmt }
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

    // ---- Fetch-time chained eager-load builder (Laravel `with(...)->get()`) ---
    // `Post::with(&["user", "comments"]).get().await` returns a typed
    // `Vec<Post>` with the requested relation FIELDS already populated — no
    // manual `serde_json::from_value`, no separate `with_relations` call. The
    // builder wraps the SAME real `rf_db_facade::QueryBuilder` used by every
    // other query entry point and remembers the requested relation names; its
    // chainable methods (`where`/`filter`/`where_op`/`order_by`/`limit`/
    // `offset`) just forward to that inner builder (only methods that REALLY
    // exist on QueryBuilder are delegated). `get(self)` runs ONE fetch,
    // deserializes the resulting `Value`s into the concrete model exactly like
    // the loaders do (`filter_map(serde_json::from_value)`), then calls
    // `Self::with_relations` for the requested names — so K relations + 1 fetch
    // = K+1 batched queries total, never N+1. With no relations it is a plain
    // typed fetch.
    let builder_name = syn::Ident::new(&format!("{}WithBuilder", name), name.span());
    let with_builder = quote! {
        /// Fetch-time chained eager-load builder for `#name` (Laravel's
        /// `with(...)->get()`). Holds the real `rf_db_facade::QueryBuilder`
        /// plus the requested relation names; chain filters/order/limit, then
        /// `get().await` for a typed `Vec<#name>` with those relations
        /// populated (K relations + 1 fetch = K+1 queries, never N+1).
        pub struct #builder_name {
            query: rf_db_facade::QueryBuilder,
            relations: ::std::vec::Vec<::std::string::String>,
            // Constrained eager loads (`with_where`): per-relation ONE equality
            // constraint `(column, value)`. When a requested relation has an
            // entry here, `get()` hydrates it with the constrained
            // `load_<name>_where` loader (same batched `WHERE IN` query + an
            // extra parametrized `AND column = value`) instead of the plain
            // `load_<name>_for`. Empty by default -> identical to the old path.
            constraints: ::std::collections::HashMap<
                ::std::string::String,
                (::std::string::String, ::serde_json::Value),
            >,
        }

        impl #builder_name {
            /// Add a `column = value` filter (delegates to the inner QueryBuilder).
            pub fn r#where<V: ::std::convert::Into<::serde_json::Value>>(
                mut self,
                column: impl ::std::convert::Into<::std::string::String>,
                value: V,
            ) -> Self {
                self.query = self.query.r#where(column, value);
                self
            }

            /// Alias for [`Self::where`] (delegates to the inner QueryBuilder).
            pub fn filter<V: ::std::convert::Into<::serde_json::Value>>(
                mut self,
                column: impl ::std::convert::Into<::std::string::String>,
                value: V,
            ) -> Self {
                self.query = self.query.filter(column, value);
                self
            }

            /// Add a `column <op> value` filter (delegates to the inner QueryBuilder).
            pub fn where_op<V: ::std::convert::Into<::serde_json::Value>>(
                mut self,
                column: impl ::std::convert::Into<::std::string::String>,
                operator: impl ::std::convert::Into<::std::string::String>,
                value: V,
            ) -> Self {
                self.query = self.query.where_op(column, operator, value);
                self
            }

            /// Add an `ORDER BY` clause (delegates to the inner QueryBuilder).
            pub fn order_by(
                mut self,
                column: impl ::std::convert::Into<::std::string::String>,
                direction: impl ::std::convert::Into<::std::string::String>,
            ) -> Self {
                self.query = self.query.order_by(column, direction);
                self
            }

            /// Set a row `LIMIT` (delegates to the inner QueryBuilder).
            pub fn limit(mut self, limit: usize) -> Self {
                self.query = self.query.limit(limit);
                self
            }

            /// Set a row `OFFSET` (delegates to the inner QueryBuilder).
            pub fn offset(mut self, offset: usize) -> Self {
                self.query = self.query.offset(offset);
                self
            }

            /// Constrain a to-be-eager-loaded relation to children matching
            /// `column = value` (Laravel's `with(['comments' => fn ($q) =>
            /// $q->where('approved', true)])`). Records ONE equality constraint
            /// for the named relation; at `get()` time that relation is hydrated
            /// by its constrained `load_<name>_where` loader (the SAME batched
            /// `WHERE <fk> IN (...)` query plus a parametrized `AND column =
            /// value`), so only matching children land in the field — still one
            /// batched query per relation, never N+1. Relations without a
            /// constraint keep the plain unconstrained path. Calling this again
            /// for the same relation replaces its constraint (one per relation
            /// this slice; multiple/operators are a follow-up).
            pub fn with_where<V: ::std::convert::Into<::serde_json::Value>>(
                mut self,
                relation: impl ::std::convert::Into<::std::string::String>,
                column: impl ::std::convert::Into<::std::string::String>,
                value: V,
            ) -> Self {
                self.constraints
                    .insert(relation.into(), (column.into(), value.into()));
                self
            }

            /// Run the fetch and return typed rows with the requested relations
            /// eagerly populated. ONE `SELECT` (this builder's QueryBuilder) is
            /// deserialized into `Vec<#name>` (rows that fail to deserialize are
            /// skipped, mirroring the loaders), then `#name::with_relations`
            /// hydrates each requested relation with its own batched query — so
            /// K relations + 1 fetch = K+1 queries, never N+1. With no requested
            /// relations this is a plain typed fetch.
            pub async fn get(self) -> ::std::result::Result<::std::vec::Vec<#name>, String> {
                let #builder_name { query, relations, constraints } = self;
                let values = query.get().await?;
                let mut rows: ::std::vec::Vec<#name> = values
                    .into_iter()
                    .filter_map(|v| ::serde_json::from_value(v).ok())
                    .collect();
                // Per-relation dispatch: a relation carrying a `with_where`
                // constraint is hydrated by its constrained `load_<name>_where`
                // loader (batched query + `AND column = value`); every other
                // relation keeps the unchanged unconstrained path through the
                // shared `with_relations` (which also handles nested `a.b`).
                for __rf_rel in relations.iter() {
                    match constraints.get(__rf_rel.as_str()) {
                        ::std::option::Option::Some((__rf_col, __rf_val)) => {
                            match __rf_rel.as_str() {
                                #(#with_where_arms)*
                                other => {
                                    return ::std::result::Result::Err(::std::format!(
                                        "with_where: unknown or non-eager relation `{}` on {}",
                                        other,
                                        #table_name,
                                    ));
                                }
                            }
                        }
                        ::std::option::Option::None => {
                            let __rf_one = [__rf_rel.as_str()];
                            #name::with_relations(&mut rows, &__rf_one).await?;
                        }
                    }
                }
                ::std::result::Result::Ok(rows)
            }
        }
    };

    // ---- Typed read fns: all_typed() + paginate() (Laravel parity) ----------
    // The trait `rf_db_facade::Model::all()` returns `Vec<serde_json::Value>`
    // and pagination requires chaining `Model::query().paginate()`. These two
    // inherent fns close that ergonomic gap: they fetch via the SAME real
    // `rf_db_facade::QueryBuilder` and deserialize Values into the concrete
    // `Self` using the exact proven `filter_map(serde_json::from_value)` pattern
    // from the `with(...).get()` builder above.
    //
    // Naming: the typed all-fetch is `all_typed` (NOT `all`) on purpose —
    // an inherent `all` would SHADOW the `Model` trait's `all()`, silently
    // changing what `#name::all()` resolves to (from `Vec<Value>` to
    // `Vec<Self>`) and breaking existing callers. `paginate` is collision-free
    // (the trait has no `paginate`).
    let page_name = syn::Ident::new(&format!("{}Page", name), name.span());
    let typed_read = quote! {
        /// A page of TYPED `#name` rows plus the real pagination metadata the
        /// `rf_db_facade::QueryBuilder::paginate` engine computed (`total`,
        /// `per_page`, `current_page`, `last_page`). Generated companion to
        /// [`#name::paginate`].
        #[derive(Debug, Clone)]
        pub struct #page_name {
            /// The typed rows for this page (already deserialized into `#name`).
            pub data: ::std::vec::Vec<#name>,
            /// Total rows matching the query across ALL pages.
            pub total: usize,
            /// Rows per page (as requested, clamped to >= 1 by the engine).
            pub per_page: usize,
            /// 1-based current page number.
            pub current_page: usize,
            /// Index of the last page (`ceil(total / per_page)`).
            pub last_page: usize,
        }

        impl #name {
            /// Fetch EVERY row of this model's table as a TYPED `Vec<Self>`
            /// (Laravel's `Model::all()`, but concrete rows instead of the
            /// trait's `Vec<serde_json::Value>`). Runs ONE `SELECT *` through the
            /// real `rf_db_facade::QueryBuilder` then deserializes each `Value`
            /// into `Self` (rows that fail to deserialize are skipped, mirroring
            /// the `with(...).get()` builder). Named `all_typed` so it does NOT
            /// shadow the `Model` trait's `all()`.
            pub async fn all_typed() -> ::std::result::Result<::std::vec::Vec<#name>, String> {
                let values = rf_db_facade::QueryBuilder::new(#table_name).get().await?;
                ::std::result::Result::Ok(
                    values
                        .into_iter()
                        .filter_map(|v| ::serde_json::from_value(v).ok())
                        .collect(),
                )
            }

            /// Paginate this model's table into a page of TYPED rows (Laravel's
            /// `Model::paginate($perPage, $page)` — no `query()` chaining needed).
            /// Delegates to the real `rf_db_facade::QueryBuilder::paginate`, which
            /// runs the limited/offset fetch AND a `COUNT(*)`, then deserializes
            /// the page's `Value` items into `Self` (undeserializable rows
            /// skipped, mirroring `all_typed`/`with(...).get()`) while preserving
            /// the engine's real `total` / `per_page` / `current_page` /
            /// `last_page` metadata. `page` is 1-based.
            pub async fn paginate(per_page: usize, page: usize) -> ::std::result::Result<#page_name, String> {
                let result = rf_db_facade::QueryBuilder::new(#table_name)
                    .paginate(per_page, page)
                    .await?;
                let data: ::std::vec::Vec<#name> = result
                    .data
                    .into_iter()
                    .filter_map(|v| ::serde_json::from_value(v).ok())
                    .collect();
                ::std::result::Result::Ok(#page_name {
                    data,
                    total: result.total,
                    per_page: result.per_page,
                    current_page: result.current_page,
                    last_page: result.last_page,
                })
            }
        }
    };

    // ---- `guarded` marker: flip the DEFAULT create() to enforcement ----------
    // With the `guarded` marker present the macro emits an INHERENT
    // `#name::create(data)` alongside the model. Rust resolves `Type::method`
    // by preferring inherent methods over trait methods, so this inherent
    // `create` SHADOWS the permissive `rf_db_facade::Model::create` default for
    // every `#name::create(...)` call site — including `create!(#name, ...)`,
    // which expands to the UFCS `#name::create(...)` (see
    // helpers::create_impl). It runs `Self::fill_guarded(data)?` (rejecting any
    // field not in MASS_ASSIGNABLE, naming the offender) and then performs the
    // SAME insert the trait default does, but by calling the real QueryBuilder
    // path DIRECTLY (`rf_db_facade::QueryBuilder::new(TABLE).create(..)`) so it
    // does NOT recurse into itself. The explicit trait-qualified call
    // `<#name as rf_db_facade::Model>::create(..)` still reaches the permissive
    // default for callers who deliberately want it (e.g. `create_guarded`'s
    // internal insert). Without the marker this is EMPTY, so non-guarded models
    // are byte-for-byte unchanged and keep the permissive trait `create`.
    let guarded_create = if guarded {
        quote! {
            impl #name {
                /// Mass-assignment-enforcing default create (Laravel's guarded
                /// `$fillable`), emitted because this model carries the
                /// `guarded` marker. Rejects any field not in
                /// [`Self::MASS_ASSIGNABLE`] via [`Self::fill_guarded`] (naming
                /// the offender, inserting NO row), then inserts exactly the
                /// permitted fields through the real
                /// `rf_db_facade::QueryBuilder` path directly (the SAME insert
                /// the permissive `Model::create` default performs) — it does
                /// NOT recurse into itself. This INHERENT method shadows the
                /// permissive `Model::create` for `#name::create(...)` and for
                /// `create!(#name, ...)` (which expands to that UFCS call). The
                /// permissive default is still reachable via the explicit
                /// `<#name as rf_db_facade::Model>::create(...)`.
                pub async fn create<D: ::serde::Serialize>(
                    data: D,
                ) -> ::std::result::Result<::serde_json::Value, ::std::string::String> {
                    let filtered = Self::fill_guarded(data)?;
                    rf_db_facade::QueryBuilder::new(#table_name).create(filtered).await
                }
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

            /// The set of fields that may be mass-assigned (Laravel's
            /// `$fillable`). By convention this is every DECLARED model field,
            /// excluding the framework-managed `id`, timestamps, soft-delete
            /// column and relations. Mass-assignment guarding is OPT-IN: the
            /// permissive [`Self::create`] path is unchanged — call
            /// [`Self::create_guarded`] / [`Self::fill_guarded`] to enforce it.
            pub const MASS_ASSIGNABLE: &'static [&'static str] = &[#(#mass_assignable),*];

            /// Filter arbitrary input down to the mass-assignable fields,
            /// REJECTING any field that is not in [`Self::MASS_ASSIGNABLE`] with
            /// a clear error naming the offending field (Laravel's strict
            /// mass-assignment protection). Non-object input passes through
            /// unchanged. This is the OPT-IN guard: the permissive
            /// `Model::create` remains available and its behavior is unchanged.
            pub fn fill_guarded<D: ::serde::Serialize>(
                data: D,
            ) -> ::std::result::Result<::serde_json::Value, ::std::string::String> {
                let value = ::serde_json::to_value(data).map_err(|e| e.to_string())?;
                if let ::serde_json::Value::Object(ref map) = value {
                    for key in map.keys() {
                        if !Self::MASS_ASSIGNABLE.contains(&key.as_str()) {
                            return ::std::result::Result::Err(::std::format!(
                                "mass-assignment denied on `{}`: field `{}` is not fillable (fillable: {:?})",
                                #table_name, key, Self::MASS_ASSIGNABLE,
                            ));
                        }
                    }
                }
                ::std::result::Result::Ok(value)
            }

            /// Guarded counterpart of [`Self::create`]: rejects mass-assignment
            /// of any field not in [`Self::MASS_ASSIGNABLE`] (naming the
            /// offender via [`Self::fill_guarded`]), otherwise inserts exactly
            /// the permitted fields through the same real `Model::create` path.
            /// The permissive `create` remains available and unchanged.
            pub async fn create_guarded<D: ::serde::Serialize>(
                data: D,
            ) -> ::std::result::Result<::serde_json::Value, ::std::string::String> {
                let filtered = Self::fill_guarded(data)?;
                <Self as rf_db_facade::Model>::create(filtered).await
            }

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

            #(#scope_methods)*

            /// Start a fetch-time chained eager-load builder (Laravel's
            /// `Model::with(...)`). Chain `where`/`filter`/`where_op`/`order_by`/
            /// `limit`/`offset`, then `get().await` for a typed `Vec<Self>` with
            /// the named relations already populated:
            ///
            /// ```rust,ignore
            /// let posts = Post::with(&["user", "comments"]) // K relations
            ///     .r#where("published", true)               // delegated filter
            ///     .order_by("created_at", "DESC")
            ///     .limit(20)
            ///     .get()                                     // 1 fetch
            ///     .await?;                                   // = K + 1 queries
            /// ```
            ///
            /// K relations + 1 fetch = K+1 batched queries total, never N+1.
            /// Passing an empty slice is a plain typed fetch.
            pub fn with(names: &[&str]) -> #builder_name {
                #builder_name {
                    query: rf_db_facade::QueryBuilder::new(#table_name),
                    relations: names.iter().map(|s| s.to_string()).collect(),
                    constraints: ::std::collections::HashMap::new(),
                }
            }

            /// Eagerly hydrate MANY relations on a whole slice with ONE call
            /// (Laravel's `Model::with(...)`). For each requested name this
            /// dispatches — via a compile-time match the macro emits over the
            /// model's declared relations — to that relation's generated
            /// `load_<name>_for` batch loader, so hydrating K relations for N
            /// rows costs a bounded number of batched queries total (direct-FK
            /// loaders run one `WHERE ... IN (...)` query each; `belongsToMany`
            /// runs two through the pivot): never N+1. Every declared relation
            /// family (`belongsTo` / `hasOne` / `hasMany` / `belongsToMany`) is
            /// eager-loadable. An unknown name is a clean `Err` naming it (like
            /// Laravel's undefined-relationship error) and no queries are run
            /// past the offending name.
            pub async fn with_relations(
                rows: &mut [Self],
                names: &[&str],
            ) -> ::std::result::Result<(), String> {
                for name in names {
                    // Nested dot-notation path (`a.b`): dispatch the FIRST
                    // segment to a nested arm that loads `a`, then re-enters the
                    // CHILD model's `with_relations` for the rest — one batched
                    // load per level, never N+1. A single-segment name keeps the
                    // original exact-match dispatch untouched.
                    if let ::std::option::Option::Some((first, rest)) = name.split_once('.') {
                        match first {
                            #(#nested_arms)*
                            other => {
                                return ::std::result::Result::Err(::std::format!(
                                    "with_relations: unknown or non-eager relation `{}` in nested path `{}.{}` on {}",
                                    other,
                                    other,
                                    rest,
                                    #table_name,
                                ));
                            }
                        }
                    } else {
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
                }
                ::std::result::Result::Ok(())
            }

            #soft_delete_methods
        }

        #with_builder

        #typed_read

        #guarded_create

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

    // Mass-assignable (Laravel's `$fillable`): every declared field. `hidden`
    // fields are legitimate inputs, so they stay mass-assignable. See
    // `generate_full_model` for the full rationale.
    let mass_assignable: Vec<String> = fields.iter()
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

    // ---- Typed read fns: all_typed() + paginate() (see generate_full_model) --
    // Same Laravel-parity typed read fns for the simple `Model!(Name: a, b)`
    // syntax: `all_typed()` returns a typed `Vec<Self>` and `paginate()` returns
    // a page of typed rows with real metadata, both via the real QueryBuilder +
    // the proven `filter_map(serde_json::from_value)` deserialize.
    let page_name = syn::Ident::new(&format!("{}Page", name), name.span());
    let typed_read = quote! {
        /// A page of TYPED `#name` rows plus the real pagination metadata from
        /// `rf_db_facade::QueryBuilder::paginate`. Generated companion to
        /// [`#name::paginate`].
        #[derive(Debug, Clone)]
        pub struct #page_name {
            /// The typed rows for this page (deserialized into `#name`).
            pub data: ::std::vec::Vec<#name>,
            /// Total rows matching the query across ALL pages.
            pub total: usize,
            /// Rows per page (as requested, clamped to >= 1 by the engine).
            pub per_page: usize,
            /// 1-based current page number.
            pub current_page: usize,
            /// Index of the last page (`ceil(total / per_page)`).
            pub last_page: usize,
        }

        impl #name {
            /// Fetch EVERY row as a TYPED `Vec<Self>` (Laravel's `Model::all()`,
            /// but concrete rows instead of the trait's `Vec<serde_json::Value>`).
            /// Named `all_typed` so it does NOT shadow the `Model` trait `all()`.
            pub async fn all_typed() -> ::std::result::Result<::std::vec::Vec<#name>, String> {
                let values = rf_db_facade::QueryBuilder::new(#table_name).get().await?;
                ::std::result::Result::Ok(
                    values
                        .into_iter()
                        .filter_map(|v| ::serde_json::from_value(v).ok())
                        .collect(),
                )
            }

            /// Paginate into a page of TYPED rows (Laravel's `Model::paginate`).
            /// Delegates to the real `rf_db_facade::QueryBuilder::paginate`,
            /// deserializes the page items into `Self`, and preserves the real
            /// `total` / `per_page` / `current_page` / `last_page`. `page` is
            /// 1-based.
            pub async fn paginate(per_page: usize, page: usize) -> ::std::result::Result<#page_name, String> {
                let result = rf_db_facade::QueryBuilder::new(#table_name)
                    .paginate(per_page, page)
                    .await?;
                let data: ::std::vec::Vec<#name> = result
                    .data
                    .into_iter()
                    .filter_map(|v| ::serde_json::from_value(v).ok())
                    .collect();
                ::std::result::Result::Ok(#page_name {
                    data,
                    total: result.total,
                    per_page: result.per_page,
                    current_page: result.current_page,
                    last_page: result.last_page,
                })
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

            /// The set of fields that may be mass-assigned (Laravel's
            /// `$fillable`) — every declared field. Guarding is OPT-IN via
            /// [`Self::create_guarded`] / [`Self::fill_guarded`]; the permissive
            /// `Model::create` path is unchanged. See `generate_full_model`.
            pub const MASS_ASSIGNABLE: &'static [&'static str] = &[#(#mass_assignable),*];

            /// Filter arbitrary input down to the mass-assignable fields,
            /// REJECTING any field not in [`Self::MASS_ASSIGNABLE`] with a clear
            /// error naming the offender. OPT-IN guard; non-object input passes
            /// through unchanged. The permissive `Model::create` is unchanged.
            pub fn fill_guarded<D: ::serde::Serialize>(
                data: D,
            ) -> ::std::result::Result<::serde_json::Value, ::std::string::String> {
                let value = ::serde_json::to_value(data).map_err(|e| e.to_string())?;
                if let ::serde_json::Value::Object(ref map) = value {
                    for key in map.keys() {
                        if !Self::MASS_ASSIGNABLE.contains(&key.as_str()) {
                            return ::std::result::Result::Err(::std::format!(
                                "mass-assignment denied on `{}`: field `{}` is not fillable (fillable: {:?})",
                                #table_name, key, Self::MASS_ASSIGNABLE,
                            ));
                        }
                    }
                }
                ::std::result::Result::Ok(value)
            }

            /// Guarded counterpart of `create`: rejects mass-assignment of any
            /// field not in [`Self::MASS_ASSIGNABLE`], else inserts the
            /// permitted fields through the real `Model::create` path.
            pub async fn create_guarded<D: ::serde::Serialize>(
                data: D,
            ) -> ::std::result::Result<::serde_json::Value, ::std::string::String> {
                let filtered = Self::fill_guarded(data)?;
                <Self as rf_db_facade::Model>::create(filtered).await
            }

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

            /// Eager-hydrate relations on a whole slice (Laravel's
            /// `Model::with(...)`). The simple `Model!(Name: a, b)` syntax
            /// declares NO relations, so any requested name is a clean `Err`
            /// naming it (mirroring Laravel's undefined-relationship error).
            /// This mirrors the full-syntax `with_relations` signature so a
            /// simple-syntax model can still be the LEAF child of a full-syntax
            /// model's nested dot-notation path (`parent.child`): the parent's
            /// nested arm dispatches the remaining segment here, and a
            /// non-relation segment on a relation-less model is that clean `Err`.
            pub async fn with_relations(
                _rows: &mut [Self],
                names: &[&str],
            ) -> ::std::result::Result<(), ::std::string::String> {
                if let ::std::option::Option::Some(name) = names.first() {
                    return ::std::result::Result::Err(::std::format!(
                        "with_relations: unknown or non-eager relation `{}` on {}",
                        name,
                        #table_name,
                    ));
                }
                ::std::result::Result::Ok(())
            }
        }

        #typed_read

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

    // Per-field custom message override (`@ message("...")`). When present it
    // REPLACES the auto-generated English message of every check emitted for
    // this field (presence + all `@` modifier checks); the stable `code` is
    // untouched. `msg_for(auto)` yields the custom text when set, else `auto`.
    let custom_msg: ::std::option::Option<String> = overrides.iter().find_map(|ov| match ov {
        FieldOverride::Message(m) => Some(m.clone()),
        _ => None,
    });
    let msg_for = |auto: String| -> String { custom_msg.clone().unwrap_or(auto) };

    // Checks that operate on a bound `value` (`&String` in scope).
    let mut inner: Vec<TokenStream2> = Vec::new();
    for ov in overrides {
        match ov {
            FieldOverride::Max(n) if is_string => {
                let n = *n;
                let msg = msg_for(format!("The {} field must be at most {} characters.", name_str, n));
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
                let msg = msg_for(format!("The {} field must be at least {} characters.", name_str, n));
                inner.push(quote! {
                    if value.len() < #n {
                        let mut error = rf_validation::ext_validator::ValidationError::new("length");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            FieldOverride::Email => {
                let msg = msg_for(format!("The {} field must be a valid email address.", name_str));
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
                let msg = msg_for(format!("The {} field must be a valid URL.", name_str));
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
                let msg = msg_for(format!("The {} field must be a valid UUID.", name_str));
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
                let msg = msg_for(format!("The {} field must be a valid IP address.", name_str));
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
                let msg = msg_for(format!(
                    "The {} field must be between {} and {}.",
                    name_str, min, max
                ));
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
                let msg = msg_for(format!(
                    "The {} field must match the pattern {}.",
                    name_str, pattern
                ));
                inner.push(quote! {
                    if !rf_validation::validators::regex::validate_regex(value, #pattern) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("regex");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ alpha` parallels `@ url`: letters-only check inlined exactly as
            // rf_validation `AlphaRule` (non-empty AND all chars alphabetic).
            // String fields only (`value` is `&String`).
            FieldOverride::Alpha if is_string => {
                let msg = msg_for(format!("The {} field must contain only letters.", name_str));
                inner.push(quote! {
                    if value.is_empty() || !value.chars().all(|c| c.is_alphabetic()) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("alpha");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ alphanumeric` parallels `@ alpha`: letters+digits check inlined
            // exactly as rf_validation `AlphaNumericRule`. String fields only.
            FieldOverride::AlphaNumeric if is_string => {
                let msg = msg_for(format!("The {} field must contain only letters and numbers.", name_str));
                inner.push(quote! {
                    if value.is_empty() || !value.chars().all(|c| c.is_alphanumeric()) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("alpha_numeric");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ starts_with("prefix")` parallels `@ regex`: prefix check inlined
            // exactly as rf_validation `StartsWithRule`. String fields only.
            FieldOverride::StartsWith(prefix) if is_string => {
                let msg = msg_for(format!("The {} field must start with '{}'.", name_str, prefix));
                inner.push(quote! {
                    if !value.starts_with(#prefix) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("starts_with");
                        error.message = ::std::option::Option::Some(::std::borrow::Cow::Borrowed(#msg));
                        errors.add(#name_str, error);
                    }
                });
            }
            // `@ ends_with("suffix")` parallels `@ starts_with`: suffix check
            // inlined exactly as rf_validation `EndsWithRule`. String fields only.
            FieldOverride::EndsWith(suffix) if is_string => {
                let msg = msg_for(format!("The {} field must end with '{}'.", name_str, suffix));
                inner.push(quote! {
                    if !value.ends_with(#suffix) {
                        let mut error = rf_validation::ext_validator::ValidationError::new("ends_with");
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
            let msg = msg_for(format!("The {} field is required.", name_str));
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
