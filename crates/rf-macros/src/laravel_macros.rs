//! High-priority Laravel-style macros for RustForge framework
//!
//! This module implements:
//! - routes! macro - Solves German keyboard || problem with clean routing syntax
//! - resource! macro - RESTful resource routing
//! - migration! macro - Database migration DSL
//! - model! macro - Enhanced model with relationships
//! - request! macro - Form validation

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, Ident, LitStr, Token,
};

// =============================================================================
// ROUTES! MACRO - HIGHEST PRIORITY (solves || keyboard problem)
// =============================================================================

/// Implements the routes! macro for Laravel-style routing
///
/// This macro solves the German keyboard || problem by providing clean syntax
/// without pipes.
///
/// # Example
///
/// ```rust,ignore
/// routes! {
///     get "/posts" => post_controller::index,
///     post "/posts" => post_controller::store,
///
///     middleware ["auth"] {
///         get "/profile" => profile_controller::show,
///         put "/profile" => profile_controller::update,
///     }
///
///     prefix "/api/v1" {
///         get "/users" => api::users::index,
///         post "/users" => api::users::store,
///     }
/// }
/// ```
pub fn routes_impl(input: TokenStream) -> TokenStream {
    let routes_input = parse_macro_input!(input as RoutesInput);

    let route_codes: Vec<TokenStream2> = routes_input
        .routes
        .iter()
        .map(|r| generate_route_code(r, "", &[]))
        .collect();

    let expanded = quote! {
        {
            #(#route_codes)*
        }
    };

    TokenStream::from(expanded)
}

struct RoutesInput {
    routes: Vec<RouteDefinition>,
}

enum RouteDefinition {
    Route {
        method: Ident,
        path: LitStr,
        handler: syn::Path,
    },
    Middleware {
        middlewares: Vec<LitStr>,
        nested: Vec<RouteDefinition>,
    },
    Prefix {
        prefix: LitStr,
        nested: Vec<RouteDefinition>,
    },
}

impl Parse for RoutesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut routes = Vec::new();

        while !input.is_empty() {
            routes.push(parse_route_item(input)?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(RoutesInput { routes })
    }
}

fn parse_route_item(input: ParseStream) -> syn::Result<RouteDefinition> {
    // Check for middleware or prefix block
    if input.peek(Ident) {
        let fork = input.fork();
        if let Ok(ident) = fork.parse::<Ident>() {
            let ident_str = ident.to_string();

            if ident_str == "middleware" {
                input.parse::<Ident>()?; // consume "middleware"

                let middlewares_content;
                syn::bracketed!(middlewares_content in input);
                let middleware_list = syn::punctuated::Punctuated::<LitStr, Token![,]>::parse_terminated(&middlewares_content)?;
                let middlewares: Vec<LitStr> = middleware_list.into_iter().collect();

                let nested_content;
                syn::braced!(nested_content in input);
                let mut nested = Vec::new();
                while !nested_content.is_empty() {
                    nested.push(parse_route_item(&nested_content)?);
                    if nested_content.peek(Token![,]) {
                        nested_content.parse::<Token![,]>()?;
                    }
                }

                return Ok(RouteDefinition::Middleware {
                    middlewares,
                    nested,
                });
            } else if ident_str == "prefix" {
                input.parse::<Ident>()?; // consume "prefix"
                let prefix: LitStr = input.parse()?;

                let nested_content;
                syn::braced!(nested_content in input);
                let mut nested = Vec::new();
                while !nested_content.is_empty() {
                    nested.push(parse_route_item(&nested_content)?);
                    if nested_content.peek(Token![,]) {
                        nested_content.parse::<Token![,]>()?;
                    }
                }

                return Ok(RouteDefinition::Prefix { prefix, nested });
            }
        }
    }

    // Parse regular route: method "path" => handler
    let method: Ident = input.parse()?;
    let path: LitStr = input.parse()?;
    input.parse::<Token![=>]>()?;
    let handler: syn::Path = input.parse()?;

    Ok(RouteDefinition::Route {
        method,
        path,
        handler,
    })
}

fn generate_route_code(
    route: &RouteDefinition,
    prefix: &str,
    middlewares: &[String],
) -> TokenStream2 {
    match route {
        RouteDefinition::Route {
            method,
            path,
            handler,
        } => {
            let full_path = if prefix.is_empty() {
                path.value()
            } else {
                format!("{}{}", prefix, path.value())
            };

            let method_str = method.to_string();
            let route_call = match method_str.as_str() {
                "get" => quote! { rf_route_facade::Route::get },
                "post" => quote! { rf_route_facade::Route::post },
                "put" => quote! { rf_route_facade::Route::put },
                "delete" => quote! { rf_route_facade::Route::delete },
                "patch" => quote! { rf_route_facade::Route::patch },
                "options" => quote! { rf_route_facade::Route::options },
                "any" => quote! { rf_route_facade::Route::any },
                _ => quote! { rf_route_facade::Route::get },
            };

            if middlewares.is_empty() {
                quote! {
                    #route_call(#full_path, #handler);
                }
            } else {
                quote! {
                    #route_call(#full_path, #handler)
                        .middleware(&[#(#middlewares),*]);
                }
            }
        }
        RouteDefinition::Middleware {
            middlewares: mw,
            nested,
        } => {
            let mw_strings: Vec<String> = mw.iter().map(|m| m.value()).collect();
            let mut combined = middlewares.to_vec();
            combined.extend(mw_strings);

            let nested_code: Vec<TokenStream2> = nested
                .iter()
                .map(|r| generate_route_code(r, prefix, &combined))
                .collect();

            quote! {
                #(#nested_code)*
            }
        }
        RouteDefinition::Prefix { prefix: pfx, nested } => {
            let new_prefix = if prefix.is_empty() {
                pfx.value()
            } else {
                format!("{}{}", prefix, pfx.value())
            };

            let nested_code: Vec<TokenStream2> = nested
                .iter()
                .map(|r| generate_route_code(r, &new_prefix, middlewares))
                .collect();

            quote! {
                #(#nested_code)*
            }
        }
    }
}

// =============================================================================
// RESOURCE! MACRO
// =============================================================================

/// Implements the resource! macro for RESTful resource routing
///
/// # Example
///
/// ```rust,ignore
/// resource!(posts, PostController);
/// resource!(users, UserController, only: [index, show]);
/// resource!(comments, CommentController, except: [destroy]);
/// ```
pub fn resource_impl(input: TokenStream) -> TokenStream {
    let resource = parse_macro_input!(input as ResourceInput);

    let name_str = resource.name.to_string();
    let controller = &resource.controller;

    // Standard RESTful routes
    let all_routes = vec![
        ("index", "get", format!("/{}", name_str)),
        ("create", "get", format!("/{}/create", name_str)),
        ("store", "post", format!("/{}", name_str)),
        ("show", "get", format!("/{}/{{id}}", name_str)),
        ("edit", "get", format!("/{}/{{id}}/edit", name_str)),
        ("update", "put", format!("/{}/{{id}}", name_str)),
        ("destroy", "delete", format!("/{}/{{id}}", name_str)),
    ];

    let routes_to_generate: Vec<_> = match &resource.filter {
        Some(ResourceFilter::Only(methods)) => {
            let method_names: Vec<String> = methods.iter().map(|m| m.to_string()).collect();
            all_routes
                .into_iter()
                .filter(|(name, _, _)| method_names.contains(&name.to_string()))
                .collect()
        }
        Some(ResourceFilter::Except(methods)) => {
            let method_names: Vec<String> = methods.iter().map(|m| m.to_string()).collect();
            all_routes
                .into_iter()
                .filter(|(name, _, _)| !method_names.contains(&name.to_string()))
                .collect()
        }
        None => all_routes,
    };

    let route_calls: Vec<TokenStream2> = routes_to_generate
        .iter()
        .map(|(method_name, http_method, path)| {
            let method_ident = Ident::new(method_name, proc_macro2::Span::call_site());
            let route_fn = match *http_method {
                "get" => quote! { rf_route_facade::Route::get },
                "post" => quote! { rf_route_facade::Route::post },
                "put" => quote! { rf_route_facade::Route::put },
                "delete" => quote! { rf_route_facade::Route::delete },
                _ => quote! { rf_route_facade::Route::get },
            };

            quote! {
                #route_fn(#path, #controller::#method_ident);
            }
        })
        .collect();

    let expanded = quote! {
        {
            #(#route_calls)*
        }
    };

    TokenStream::from(expanded)
}

struct ResourceInput {
    name: Ident,
    controller: syn::Path,
    filter: Option<ResourceFilter>,
}

enum ResourceFilter {
    Only(Vec<Ident>),
    Except(Vec<Ident>),
}

impl Parse for ResourceInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let controller: syn::Path = input.parse()?;

        let filter = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let filter_type: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            let content;
            syn::bracketed!(content in input);
            let methods = content.parse_terminated(Ident::parse, Token![,])?;
            let methods_vec: Vec<Ident> = methods.into_iter().collect();

            match filter_type.to_string().as_str() {
                "only" => Some(ResourceFilter::Only(methods_vec)),
                "except" => Some(ResourceFilter::Except(methods_vec)),
                _ => {
                    return Err(syn::Error::new(
                        filter_type.span(),
                        "Expected 'only' or 'except'",
                    ))
                }
            }
        } else {
            None
        };

        Ok(ResourceInput {
            name,
            controller,
            filter,
        })
    }
}

// =============================================================================
// MIGRATION! MACRO
// =============================================================================

/// Implements the migration! macro for database migrations
///
/// # Example
///
/// ```rust,ignore
/// migration! {
///     create_table users {
///         id: primary,
///         email: string unique,
///         name: string,
///         password: string,
///         role: string = "user",
///         timestamps,
///     }
/// }
/// ```
pub fn migration_impl(input: TokenStream) -> TokenStream {
    let migration = parse_macro_input!(input as MigrationInput);

    let code = migration
        .operations
        .iter()
        .map(|op| match op {
            MigrationOperation::CreateTable { name, columns } => {
                let table_name = name.to_string();
                let column_code: Vec<TokenStream2> = columns
                    .iter()
                    .map(|col| {
                        let col_name_str = col.name.to_string();
                        match &col.column_type {
                            ColumnType::Primary => quote! {
                                schema.integer(#col_name_str).primary_key().auto_increment();
                            },
                            ColumnType::String {
                                unique,
                                nullable,
                                default,
                            } => {
                                let mut chain = quote! { schema.string(#col_name_str) };
                                if *unique {
                                    chain = quote! { #chain.unique() };
                                }
                                if *nullable {
                                    chain = quote! { #chain.nullable() };
                                }
                                if let Some(def) = default {
                                    chain = quote! { #chain.default(#def) };
                                }
                                quote! { #chain; }
                            }
                            ColumnType::Integer { nullable, default } => {
                                let mut chain = quote! { schema.integer(#col_name_str) };
                                if *nullable {
                                    chain = quote! { #chain.nullable() };
                                }
                                if let Some(def) = default {
                                    chain = quote! { #chain.default(#def) };
                                }
                                quote! { #chain; }
                            }
                            ColumnType::Bool { default } => {
                                let mut chain = quote! { schema.boolean(#col_name_str) };
                                if let Some(def) = default {
                                    chain = quote! { #chain.default(#def) };
                                }
                                quote! { #chain; }
                            }
                            ColumnType::Timestamps => quote! {
                                schema.timestamps();
                            },
                        }
                    })
                    .collect();

                quote! {
                    schema.create(#table_name, |table| {
                        #(#column_code)*
                    });
                }
            }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        {
            #(#code)*
        }
    };

    TokenStream::from(expanded)
}

struct MigrationInput {
    operations: Vec<MigrationOperation>,
}

enum MigrationOperation {
    CreateTable {
        name: Ident,
        columns: Vec<ColumnDefinition>,
    },
}

struct ColumnDefinition {
    name: Ident,
    column_type: ColumnType,
}

#[derive(Clone)]
enum ColumnType {
    Primary,
    String {
        unique: bool,
        nullable: bool,
        default: Option<String>,
    },
    Integer {
        nullable: bool,
        default: Option<i32>,
    },
    Bool {
        default: Option<bool>,
    },
    Timestamps,
}

impl Parse for MigrationInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut operations = Vec::new();

        if input.peek(Ident) {
            let lookahead: Ident = input.fork().parse()?;
            if lookahead == "create_table" {
                input.parse::<Ident>()?; // consume "create_table"
                let table_name: Ident = input.parse()?;

                let content;
                syn::braced!(content in input);

                let mut columns = Vec::new();
                while !content.is_empty() {
                    let col_name: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;

                    let col_type = parse_column_type(&content)?;
                    columns.push(ColumnDefinition {
                        name: col_name,
                        column_type: col_type,
                    });

                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    }
                }

                operations.push(MigrationOperation::CreateTable {
                    name: table_name,
                    columns,
                });
            }
        }

        Ok(MigrationInput { operations })
    }
}

fn parse_column_type(input: ParseStream) -> syn::Result<ColumnType> {
    let type_ident: Ident = input.parse()?;
    let type_str = type_ident.to_string();

    match type_str.as_str() {
        "primary" => Ok(ColumnType::Primary),
        "timestamps" => Ok(ColumnType::Timestamps),
        "string" => {
            let mut unique = false;
            let mut nullable = false;
            let mut default = None;

            while input.peek(Ident) {
                let modifier: Ident = input.parse()?;
                match modifier.to_string().as_str() {
                    "unique" => unique = true,
                    "nullable" => nullable = true,
                    _ => {}
                }
            }

            if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                let default_lit: LitStr = input.parse()?;
                default = Some(default_lit.value());
            }

            Ok(ColumnType::String {
                unique,
                nullable,
                default,
            })
        }
        "integer" | "i32" | "i64" => Ok(ColumnType::Integer {
            nullable: false,
            default: None,
        }),
        "bool" | "boolean" => {
            let mut default = None;
            if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                let default_lit: syn::LitBool = input.parse()?;
                default = Some(default_lit.value);
            }
            Ok(ColumnType::Bool { default })
        }
        _ => Ok(ColumnType::String {
            unique: false,
            nullable: false,
            default: None,
        }),
    }
}

// =============================================================================
// MODEL! MACRO WITH RELATIONSHIPS
// =============================================================================

/// Implements the model! macro with relationships
///
/// # Example
///
/// ```rust,ignore
/// model! {
///     Post => "posts" {
///         id: i32 primary,
///         user_id: i32,
///         title: String,
///         content: String,
///         published: bool = false,
///         timestamps,
///
///         belongs_to User via user_id,
///         has_many Comment,
///     }
/// }
/// ```
pub fn model_impl(input: TokenStream) -> TokenStream {
    let model = parse_macro_input!(input as ModelInput);

    let name = &model.name;
    let table = &model.table;

    let field_defs: Vec<TokenStream2> = model
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.name;
            let field_type = &f.field_type;
            quote! {
                pub #field_name: #field_type
            }
        })
        .collect();

    let relationship_methods: Vec<TokenStream2> = model
        .relationships
        .iter()
        .map(|rel| match rel {
            Relationship::BelongsTo {
                model: rel_model,
                foreign_key,
            } => {
                let method_name = Ident::new(
                    &rel_model.to_string().to_lowercase().to_string(),
                    proc_macro2::Span::call_site(),
                );
                quote! {
                    pub async fn #method_name(&self) -> Result<#rel_model, Box<dyn std::error::Error>> {
                        #rel_model::find(self.#foreign_key).await
                    }
                }
            }
            Relationship::HasMany { model: rel_model } => {
                let method_name = Ident::new(
                    &format!("{}s", rel_model.to_string().to_lowercase()),
                    proc_macro2::Span::call_site(),
                );
                let foreign_key = Ident::new(
                    &format!("{}_id", name.to_string().to_lowercase()),
                    proc_macro2::Span::call_site(),
                );
                quote! {
                    pub async fn #method_name(&self) -> Result<Vec<#rel_model>, Box<dyn std::error::Error>> {
                        #rel_model::r#where(stringify!(#foreign_key), &self.id).get().await
                    }
                }
            }
        })
        .collect();

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #name {
            #(#field_defs,)*
        }

        impl #name {
            pub fn table_name() -> &'static str {
                #table
            }

            #(#relationship_methods)*
        }
    };

    TokenStream::from(expanded)
}

struct ModelInput {
    name: Ident,
    table: LitStr,
    fields: Vec<ModelField>,
    relationships: Vec<Relationship>,
}

struct ModelField {
    name: Ident,
    field_type: syn::Type,
}

enum Relationship {
    BelongsTo {
        model: Ident,
        foreign_key: Ident,
    },
    HasMany {
        model: Ident,
    },
}

impl Parse for ModelInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        let table: LitStr = input.parse()?;

        let content;
        syn::braced!(content in input);

        let mut fields = Vec::new();
        let mut relationships = Vec::new();

        while !content.is_empty() {
            let first: Ident = content.fork().parse()?;
            let first_str = first.to_string();

            if first_str == "belongs_to" {
                content.parse::<Ident>()?; // consume "belongs_to"
                let model: Ident = content.parse()?;
                content.parse::<Ident>()?; // "via"
                let foreign_key: Ident = content.parse()?;
                relationships.push(Relationship::BelongsTo { model, foreign_key });
            } else if first_str == "has_many" {
                content.parse::<Ident>()?; // consume "has_many"
                let model: Ident = content.parse()?;
                relationships.push(Relationship::HasMany { model });
            } else if first_str == "timestamps" {
                content.parse::<Ident>()?; // consume "timestamps"
                fields.push(ModelField {
                    name: Ident::new("created_at", proc_macro2::Span::call_site()),
                    field_type: syn::parse_quote!(Option<chrono::DateTime<chrono::Utc>>),
                });
                fields.push(ModelField {
                    name: Ident::new("updated_at", proc_macro2::Span::call_site()),
                    field_type: syn::parse_quote!(Option<chrono::DateTime<chrono::Utc>>),
                });
            } else {
                // Parse field: name: Type
                let field_name: Ident = content.parse()?;
                content.parse::<Token![:]>()?;
                let field_type: syn::Type = content.parse()?;

                // Skip "primary" modifier if present
                if content.peek(Ident) {
                    let _ = content.parse::<Ident>();
                }

                // Skip default value if present
                if content.peek(Token![=]) {
                    content.parse::<Token![=]>()?;
                    let _: Expr = content.parse()?;
                }

                fields.push(ModelField {
                    name: field_name,
                    field_type,
                });
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(ModelInput {
            name,
            table,
            fields,
            relationships,
        })
    }
}

// =============================================================================
// REQUEST! MACRO FOR VALIDATION
// =============================================================================

/// Implements the request! macro for form validation
///
/// # Example
///
/// ```rust,ignore
/// request! {
///     CreateUser {
///         email: email,
///         name: length(3, 50),
///         password: length(8) + uppercase + number,
///         age: range(18, 120) | optional,
///     }
/// }
/// ```
pub fn request_impl(input: TokenStream) -> TokenStream {
    let request = parse_macro_input!(input as RequestInput);

    let name = &request.name;

    let field_defs: Vec<TokenStream2> = request
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.name;
            let is_optional = f
                .validations
                .iter()
                .any(|v| matches!(v, ValidationRule::Optional));

            if is_optional {
                quote! {
                    pub #field_name: Option<String>
                }
            } else {
                quote! {
                    pub #field_name: String
                }
            }
        })
        .collect();

    let validation_code: Vec<TokenStream2> = request
        .fields
        .iter()
        .map(|f| {
            let field_name = &f.name;
            let field_str = field_name.to_string();

            let rules: Vec<TokenStream2> = f
                .validations
                .iter()
                .filter_map(|rule| match rule {
                    ValidationRule::Email => Some(quote! {
                        if !self.#field_name.contains('@') {
                            errors.push(format!("{} must be a valid email", #field_str));
                        }
                    }),
                    ValidationRule::Length { min, max } => {
                        if let Some(min_val) = min {
                            if let Some(max_val) = max {
                                Some(quote! {
                                    let len = self.#field_name.len();
                                    if len < #min_val || len > #max_val {
                                        errors.push(format!("{} must be between {} and {} characters", #field_str, #min_val, #max_val));
                                    }
                                })
                            } else {
                                Some(quote! {
                                    if self.#field_name.len() < #min_val {
                                        errors.push(format!("{} must be at least {} characters", #field_str, #min_val));
                                    }
                                })
                            }
                        } else {
                            None
                        }
                    }
                    ValidationRule::Range { min, max } => Some(quote! {
                        if let Ok(val) = self.#field_name.parse::<i32>() {
                            if val < #min || val > #max {
                                errors.push(format!("{} must be between {} and {}", #field_str, #min, #max));
                            }
                        }
                    }),
                    ValidationRule::Uppercase => Some(quote! {
                        if !self.#field_name.chars().any(|c| c.is_uppercase()) {
                            errors.push(format!("{} must contain at least one uppercase letter", #field_str));
                        }
                    }),
                    ValidationRule::Number => Some(quote! {
                        if !self.#field_name.chars().any(|c| c.is_numeric()) {
                            errors.push(format!("{} must contain at least one number", #field_str));
                        }
                    }),
                    ValidationRule::Optional => None,
                })
                .collect();

            quote! {
                #(#rules)*
            }
        })
        .collect();

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #name {
            #(#field_defs,)*
        }

        impl #name {
            pub fn validate(&self) -> Result<(), Vec<String>> {
                let mut errors = Vec::new();
                #(#validation_code)*
                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        }
    };

    TokenStream::from(expanded)
}

struct RequestInput {
    name: Ident,
    fields: Vec<RequestField>,
}

struct RequestField {
    name: Ident,
    validations: Vec<ValidationRule>,
}

enum ValidationRule {
    Email,
    Length { min: Option<usize>, max: Option<usize> },
    Range { min: i32, max: i32 },
    Uppercase,
    Number,
    Optional,
}

impl Parse for RequestInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;

        let content;
        syn::braced!(content in input);

        let mut fields = Vec::new();

        while !content.is_empty() {
            let field_name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;

            let mut validations = Vec::new();

            // Parse validation rules separated by +
            loop {
                let rule_name: Ident = content.parse()?;
                let rule_str = rule_name.to_string();

                let rule = match rule_str.as_str() {
                    "email" => ValidationRule::Email,
                    "uppercase" => ValidationRule::Uppercase,
                    "number" => ValidationRule::Number,
                    "optional" => ValidationRule::Optional,
                    "length" => {
                        let args_content;
                        syn::parenthesized!(args_content in content);
                        let first: syn::LitInt = args_content.parse()?;
                        let min = Some(first.base10_parse()?);
                        let max = if args_content.peek(Token![,]) {
                            args_content.parse::<Token![,]>()?;
                            let second: syn::LitInt = args_content.parse()?;
                            Some(second.base10_parse()?)
                        } else {
                            None
                        };
                        ValidationRule::Length { min, max }
                    }
                    "range" => {
                        let args_content;
                        syn::parenthesized!(args_content in content);
                        let min_lit: syn::LitInt = args_content.parse()?;
                        args_content.parse::<Token![,]>()?;
                        let max_lit: syn::LitInt = args_content.parse()?;
                        ValidationRule::Range {
                            min: min_lit.base10_parse()?,
                            max: max_lit.base10_parse()?,
                        }
                    }
                    _ => ValidationRule::Email, // default fallback
                };

                validations.push(rule);

                // Check for + (more rules) or | (optional)
                if content.peek(Token![+]) {
                    content.parse::<Token![+]>()?;
                    continue;
                } else if content.peek(Token![|]) {
                    content.parse::<Token![|]>()?;
                    let opt: Ident = content.parse()?;
                    if opt == "optional" {
                        validations.push(ValidationRule::Optional);
                    }
                    break;
                } else {
                    break;
                }
            }

            fields.push(RequestField {
                name: field_name,
                validations,
            });

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(RequestInput { name, fields })
    }
}
