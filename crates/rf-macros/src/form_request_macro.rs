//! FormRequest Macro - Laravel-style Form Validation
//!
//! Define form requests with automatic validation, just like Laravel:
//!
//! ```rust,ignore
//! form_request! {
//!     struct CreateUserRequest {
//!         #[required, email, unique("users", "email")]
//!         email: String,
//!
//!         #[required, min(8)]
//!         password: String,
//!
//!         #[required, min(2), max(100)]
//!         name: String,
//!     }
//!
//!     fn authorize(&self) -> bool {
//!         true  // Or check Auth::check()
//!     }
//!
//!     fn messages() -> HashMap<&'static str, &'static str> {
//!         hashmap! {
//!             "email.required" => "Email is required",
//!             "email.email" => "Please provide a valid email",
//!         }
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, format_ident};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Attribute, Fields, Ident, ItemFn, ItemStruct, Token, Type, Visibility,
    punctuated::Punctuated,
    braced,
};

/// Parsed form request definition
struct FormRequestDef {
    vis: Visibility,
    name: Ident,
    fields: Vec<FormField>,
    authorize_fn: Option<ItemFn>,
    messages_fn: Option<ItemFn>,
    prepare_fn: Option<ItemFn>,
}

/// A field with its validation rules
struct FormField {
    vis: Visibility,
    name: Ident,
    ty: Type,
    rules: Vec<ValidationRule>,
}

/// A parsed validation rule
// Some variants model Laravel rules the parser does not yet construct (WIP).
#[allow(dead_code)]
#[derive(Clone)]
enum ValidationRule {
    Required,
    Email,
    Url,
    Ip,
    Uuid,
    Integer,
    Numeric,
    String,
    Boolean,
    Array,
    Alpha,
    AlphaNum,
    Lowercase,
    Uppercase,
    Date,
    Confirmed,
    Min(TokenStream2),
    Max(TokenStream2),
    Between(TokenStream2, TokenStream2),
    MinLength(TokenStream2),
    MaxLength(TokenStream2),
    Size(TokenStream2),
    In(Vec<TokenStream2>),
    NotIn(Vec<TokenStream2>),
    Regex(TokenStream2),
    DateFormat(TokenStream2),
    Before(TokenStream2),
    After(TokenStream2),
    Unique(TokenStream2, TokenStream2),
    Exists(TokenStream2, TokenStream2),
    Same(TokenStream2),
    Different(TokenStream2),
    RequiredIf(TokenStream2, TokenStream2),
    RequiredUnless(TokenStream2, TokenStream2),
    RequiredWith(TokenStream2),
    RequiredWithout(TokenStream2),
    Nullable,
    Custom(Ident, Vec<TokenStream2>),
}

impl Parse for FormRequestDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse struct keyword
        let vis: Visibility = input.parse()?;
        let _struct_token: Token![struct] = input.parse()?;
        let name: Ident = input.parse()?;

        // Parse fields in braces
        let content;
        braced!(content in input);

        let mut fields = Vec::new();
        while !content.is_empty() {
            // Parse field attributes
            let attrs: Vec<Attribute> = content.call(Attribute::parse_outer)?;
            let field_vis: Visibility = content.parse()?;
            let field_name: Ident = content.parse()?;
            let _colon: Token![:] = content.parse()?;
            let field_ty: Type = content.parse()?;

            // Consume comma if present
            let _ = content.parse::<Token![,]>();

            // Parse validation rules from attributes
            let rules = parse_validation_attrs(&attrs)?;

            fields.push(FormField {
                vis: field_vis,
                name: field_name,
                ty: field_ty,
                rules,
            });
        }

        // Parse optional functions
        let mut authorize_fn = None;
        let mut messages_fn = None;
        let mut prepare_fn = None;

        while !input.is_empty() {
            let func: ItemFn = input.parse()?;
            let func_name = func.sig.ident.to_string();

            match func_name.as_str() {
                "authorize" => authorize_fn = Some(func),
                "messages" => messages_fn = Some(func),
                "prepare_for_validation" | "prepare" => prepare_fn = Some(func),
                _ => {} // Ignore unknown functions
            }
        }

        Ok(FormRequestDef {
            vis,
            name,
            fields,
            authorize_fn,
            messages_fn,
            prepare_fn,
        })
    }
}

fn parse_validation_attrs(attrs: &[Attribute]) -> syn::Result<Vec<ValidationRule>> {
    let mut rules = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("validate") || attr.path().is_ident("rules") {
            // Parse #[validate(...)] or #[rules(...)]
            attr.parse_nested_meta(|meta| {
                let rule = parse_single_rule(&meta)?;
                rules.push(rule);
                Ok(())
            })?;
        } else {
            // Try to parse as direct rule attribute like #[required] or #[email]
            let path = attr.path();
            if let Some(ident) = path.get_ident() {
                let ident_str = ident.to_string();
                match ident_str.as_str() {
                    "required" => rules.push(ValidationRule::Required),
                    "email" => rules.push(ValidationRule::Email),
                    "url" => rules.push(ValidationRule::Url),
                    "ip" => rules.push(ValidationRule::Ip),
                    "uuid" => rules.push(ValidationRule::Uuid),
                    "integer" => rules.push(ValidationRule::Integer),
                    "numeric" => rules.push(ValidationRule::Numeric),
                    "string" => rules.push(ValidationRule::String),
                    "boolean" => rules.push(ValidationRule::Boolean),
                    "array" => rules.push(ValidationRule::Array),
                    "alpha" => rules.push(ValidationRule::Alpha),
                    "alpha_num" => rules.push(ValidationRule::AlphaNum),
                    "lowercase" => rules.push(ValidationRule::Lowercase),
                    "uppercase" => rules.push(ValidationRule::Uppercase),
                    "date" => rules.push(ValidationRule::Date),
                    "confirmed" => rules.push(ValidationRule::Confirmed),
                    "nullable" => rules.push(ValidationRule::Nullable),
                    _ => {
                        // Check if it has arguments
                        if let syn::Meta::List(meta_list) = &attr.meta {
                            let rule = parse_rule_with_args(&ident_str, &meta_list.tokens)?;
                            rules.push(rule);
                        }
                    }
                }
            }
        }
    }

    Ok(rules)
}

fn parse_single_rule(meta: &syn::meta::ParseNestedMeta) -> syn::Result<ValidationRule> {
    let path = meta.path.get_ident().map(|i| i.to_string()).unwrap_or_default();

    match path.as_str() {
        "required" => Ok(ValidationRule::Required),
        "email" => Ok(ValidationRule::Email),
        "url" => Ok(ValidationRule::Url),
        "ip" => Ok(ValidationRule::Ip),
        "uuid" => Ok(ValidationRule::Uuid),
        "integer" => Ok(ValidationRule::Integer),
        "numeric" => Ok(ValidationRule::Numeric),
        "string" => Ok(ValidationRule::String),
        "boolean" => Ok(ValidationRule::Boolean),
        "array" => Ok(ValidationRule::Array),
        "alpha" => Ok(ValidationRule::Alpha),
        "alpha_num" => Ok(ValidationRule::AlphaNum),
        "lowercase" => Ok(ValidationRule::Lowercase),
        "uppercase" => Ok(ValidationRule::Uppercase),
        "date" => Ok(ValidationRule::Date),
        "confirmed" => Ok(ValidationRule::Confirmed),
        "nullable" => Ok(ValidationRule::Nullable),
        "min" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let value: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Min(value))
        }
        "max" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let value: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Max(value))
        }
        "between" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let min: TokenStream2 = content.parse()?;
            let _comma: Token![,] = content.parse()?;
            let max: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Between(min, max))
        }
        "min_length" | "minLength" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let value: TokenStream2 = content.parse()?;
            Ok(ValidationRule::MinLength(value))
        }
        "max_length" | "maxLength" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let value: TokenStream2 = content.parse()?;
            Ok(ValidationRule::MaxLength(value))
        }
        "size" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let value: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Size(value))
        }
        "regex" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let pattern: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Regex(pattern))
        }
        "date_format" | "dateFormat" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let format: TokenStream2 = content.parse()?;
            Ok(ValidationRule::DateFormat(format))
        }
        "before" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let date: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Before(date))
        }
        "after" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let date: TokenStream2 = content.parse()?;
            Ok(ValidationRule::After(date))
        }
        "unique" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let table: TokenStream2 = content.parse()?;
            let _comma: Token![,] = content.parse()?;
            let column: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Unique(table, column))
        }
        "exists" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let table: TokenStream2 = content.parse()?;
            let _comma: Token![,] = content.parse()?;
            let column: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Exists(table, column))
        }
        "same" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let other: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Same(other))
        }
        "different" => {
            let content;
            syn::parenthesized!(content in meta.input);
            let other: TokenStream2 = content.parse()?;
            Ok(ValidationRule::Different(other))
        }
        _ => {
            // Custom rule
            let ident = format_ident!("{}", path);
            let args = if meta.input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in meta.input);
                let args: Punctuated<TokenStream2, Token![,]> =
                    Punctuated::parse_terminated(&content)?;
                args.into_iter().collect()
            } else {
                vec![]
            };
            Ok(ValidationRule::Custom(ident, args))
        }
    }
}

fn parse_rule_with_args(name: &str, tokens: &TokenStream2) -> syn::Result<ValidationRule> {
    match name {
        "min" => Ok(ValidationRule::Min(tokens.clone())),
        "max" => Ok(ValidationRule::Max(tokens.clone())),
        "min_length" | "minLength" => Ok(ValidationRule::MinLength(tokens.clone())),
        "max_length" | "maxLength" => Ok(ValidationRule::MaxLength(tokens.clone())),
        "size" => Ok(ValidationRule::Size(tokens.clone())),
        "regex" => Ok(ValidationRule::Regex(tokens.clone())),
        "date_format" | "dateFormat" => Ok(ValidationRule::DateFormat(tokens.clone())),
        "before" => Ok(ValidationRule::Before(tokens.clone())),
        "after" => Ok(ValidationRule::After(tokens.clone())),
        _ => {
            let ident = format_ident!("{}", name);
            Ok(ValidationRule::Custom(ident, vec![tokens.clone()]))
        }
    }
}

fn rule_to_tokens(rule: &ValidationRule) -> TokenStream2 {
    match rule {
        ValidationRule::Required => quote! { Box::new(rf_validation::rules::RequiredRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Email => quote! { Box::new(rf_validation::rules::EmailRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Url => quote! { Box::new(rf_validation::rules::UrlRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Ip => quote! { Box::new(rf_validation::rules::IpRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Uuid => quote! { Box::new(rf_validation::rules::UuidRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Integer => quote! { Box::new(rf_validation::rules::IntegerRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Numeric => quote! { Box::new(rf_validation::rules::NumericRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::String => quote! { Box::new(rf_validation::rules::StringRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Boolean => quote! { Box::new(rf_validation::rules::BooleanRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Array => quote! { Box::new(rf_validation::rules::ArrayRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Alpha => quote! { Box::new(rf_validation::rules::AlphaRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::AlphaNum => quote! { Box::new(rf_validation::rules::AlphaNumericRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Lowercase => quote! { Box::new(rf_validation::rules::LowercaseRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Uppercase => quote! { Box::new(rf_validation::rules::UppercaseRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Date => quote! { Box::new(rf_validation::rules::DateRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Confirmed => quote! { Box::new(rf_validation::rules::ConfirmedRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Nullable => quote! { Box::new(rf_validation::rules::NullableRule) as Box<dyn rf_validation::Rule> },
        ValidationRule::Min(val) => quote! { Box::new(rf_validation::rules::MinRule::new(#val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Max(val) => quote! { Box::new(rf_validation::rules::MaxRule::new(#val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Between(min, max) => quote! { Box::new(rf_validation::rules::BetweenRule::new(#min, #max)) as Box<dyn rf_validation::Rule> },
        ValidationRule::MinLength(val) => quote! { Box::new(rf_validation::rules::MinLengthRule::new(#val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::MaxLength(val) => quote! { Box::new(rf_validation::rules::MaxLengthRule::new(#val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Size(val) => quote! { Box::new(rf_validation::rules::SizeRule::new(#val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::In(vals) => quote! { Box::new(rf_validation::rules::InRule::new(vec![#(#vals.to_string()),*])) as Box<dyn rf_validation::Rule> },
        ValidationRule::NotIn(vals) => quote! { Box::new(rf_validation::rules::NotInRule::new(vec![#(#vals.to_string()),*])) as Box<dyn rf_validation::Rule> },
        ValidationRule::Regex(pattern) => quote! { Box::new(rf_validation::rules::RegexRule::new(#pattern)) as Box<dyn rf_validation::Rule> },
        ValidationRule::DateFormat(fmt) => quote! { Box::new(rf_validation::rules::DateFormatRule::new(#fmt)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Before(date) => quote! { Box::new(rf_validation::rules::BeforeRule::new(#date)) as Box<dyn rf_validation::Rule> },
        ValidationRule::After(date) => quote! { Box::new(rf_validation::rules::AfterRule::new(#date)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Unique(table, col) => quote! { Box::new(rf_validation::rules::SimpleUniqueRule::new(#table, #col)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Exists(table, col) => quote! { Box::new(rf_validation::rules::SimpleExistsRule::new(#table, #col)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Same(other) => quote! { Box::new(rf_validation::rules::SameRule::new(#other)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Different(other) => quote! { Box::new(rf_validation::rules::DifferentRule::new(#other)) as Box<dyn rf_validation::Rule> },
        ValidationRule::RequiredIf(field, val) => quote! { Box::new(rf_validation::rules::RequiredIfRule::new(#field, #val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::RequiredUnless(field, val) => quote! { Box::new(rf_validation::rules::RequiredUnlessRule::new(#field, #val)) as Box<dyn rf_validation::Rule> },
        ValidationRule::RequiredWith(field) => quote! { Box::new(rf_validation::rules::RequiredWithRule::new(#field)) as Box<dyn rf_validation::Rule> },
        ValidationRule::RequiredWithout(field) => quote! { Box::new(rf_validation::rules::RequiredWithoutRule::new(#field)) as Box<dyn rf_validation::Rule> },
        ValidationRule::Custom(name, args) => {
            if args.is_empty() {
                quote! { Box::new(#name) as Box<dyn rf_validation::Rule> }
            } else {
                quote! { Box::new(#name::new(#(#args),*)) as Box<dyn rf_validation::Rule> }
            }
        }
    }
}

pub fn form_request_impl(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as FormRequestDef);

    let vis = &def.vis;
    let name = &def.name;

    // Generate struct fields
    let struct_fields: Vec<_> = def.fields.iter().map(|f| {
        let fvis = &f.vis;
        let fname = &f.name;
        let fty = &f.ty;
        quote! { #fvis #fname: #fty }
    }).collect();

    // Generate rules for each field
    let rules_entries: Vec<_> = def.fields.iter().filter_map(|f| {
        if f.rules.is_empty() {
            return None;
        }
        let fname = f.name.to_string();
        let rule_tokens: Vec<_> = f.rules.iter().map(rule_to_tokens).collect();
        Some(quote! {
            rules.insert(#fname, vec![#(#rule_tokens),*]);
        })
    }).collect();

    // Generate authorize function
    let authorize_impl = if let Some(func) = &def.authorize_fn {
        let block = &func.block;
        quote! { #block }
    } else {
        quote! { { true } }
    };

    // Generate messages function
    let messages_impl = if let Some(func) = &def.messages_fn {
        let block = &func.block;
        quote! { #block }
    } else {
        quote! { { std::collections::HashMap::new() } }
    };

    // Generate prepare function
    let prepare_impl = if let Some(func) = &def.prepare_fn {
        let block = &func.block;
        quote! { #block }
    } else {
        quote! { {} }
    };

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        #vis struct #name {
            #(#struct_fields),*
        }

        #[async_trait::async_trait]
        impl rf_validation::FormRequest for #name {
            type Validated = Self;

            fn rules(&self) -> rf_validation::ValidationRules {
                let mut rules: rf_validation::ValidationRules = std::collections::HashMap::new();
                #(#rules_entries)*
                rules
            }

            fn messages(&self) -> rf_validation::ValidationMessages {
                #messages_impl
            }

            fn authorize(&self) -> bool {
                #authorize_impl
            }

            fn prepare_for_validation(&mut self) {
                #prepare_impl
            }

            async fn validate(self) -> rf_validation::FormRequestResult<Self::Validated> {
                // Check authorization first
                if !self.authorize() {
                    return Err(rf_validation::FormRequestError::Unauthorized);
                }

                // Get rules and messages
                let rules = self.rules();
                let messages = self.messages();

                // Convert self to JSON for validation
                let data: serde_json::Value = serde_json::to_value(&self)
                    .map_err(|e| rf_validation::FormRequestError::InvalidBody(e.to_string()))?;

                // Create validator
                let data_map: std::collections::HashMap<String, serde_json::Value> =
                    if let serde_json::Value::Object(obj) = data {
                        obj.into_iter().collect()
                    } else {
                        std::collections::HashMap::new()
                    };

                let mut validator = rf_validation::Validator::new(data_map);
                validator.rules(rules);
                validator.messages(messages);

                // Validate
                match validator.validate().await {
                    Ok(_) => Ok(self),
                    Err(errors) => Err(rf_validation::FormRequestError::ValidationFailed(
                        rf_validation::ValidationErrors::from_validator_errors(errors)
                    )),
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Simpler attribute-based form request validation
///
/// ```rust,ignore
/// #[form_request]
/// struct CreateUserRequest {
///     #[validate(required, email)]
///     email: String,
///
///     #[validate(required, min(8))]
///     password: String,
/// }
/// ```
pub fn form_request_attr_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    let vis = &input.vis;
    let name = &input.ident;
    let attrs = &input.attrs;

    // Parse fields and their validation rules
    let fields: Vec<_> = match &input.fields {
        Fields::Named(named) => named.named.iter().collect(),
        _ => {
            return syn::Error::new_spanned(
                &input,
                "form_request only supports structs with named fields"
            ).to_compile_error().into();
        }
    };

    let mut rules_entries = Vec::new();

    for field in &fields {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let mut field_rules = Vec::new();

        for attr in &field.attrs {
            if attr.path().is_ident("validate") || attr.path().is_ident("rules") {
                if let Err(e) = attr.parse_nested_meta(|meta| {
                    if let Ok(rule) = parse_single_rule(&meta) {
                        field_rules.push(rule);
                    }
                    Ok(())
                }) {
                    return e.to_compile_error().into();
                }
            }
        }

        if !field_rules.is_empty() {
            let rule_tokens: Vec<_> = field_rules.iter().map(rule_to_tokens).collect();
            rules_entries.push(quote! {
                rules.insert(#field_name, vec![#(#rule_tokens),*]);
            });
        }
    }

    // Remove validation attributes from fields for the output struct
    let clean_fields: Vec<_> = fields.iter().map(|f| {
        let mut field = (*f).clone();
        field.attrs.retain(|attr| {
            !attr.path().is_ident("validate") && !attr.path().is_ident("rules")
        });
        field
    }).collect();

    let expanded = quote! {
        #(#attrs)*
        #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
        #vis struct #name {
            #(#clean_fields),*
        }

        #[async_trait::async_trait]
        impl rf_validation::FormRequest for #name {
            type Validated = Self;

            fn rules(&self) -> rf_validation::ValidationRules {
                let mut rules: rf_validation::ValidationRules = std::collections::HashMap::new();
                #(#rules_entries)*
                rules
            }

            async fn validate(self) -> rf_validation::FormRequestResult<Self::Validated> {
                if !self.authorize() {
                    return Err(rf_validation::FormRequestError::Unauthorized);
                }

                let rules = self.rules();
                let messages = self.messages();

                let data: serde_json::Value = serde_json::to_value(&self)
                    .map_err(|e| rf_validation::FormRequestError::InvalidBody(e.to_string()))?;

                let data_map: std::collections::HashMap<String, serde_json::Value> =
                    if let serde_json::Value::Object(obj) = data {
                        obj.into_iter().collect()
                    } else {
                        std::collections::HashMap::new()
                    };

                let mut validator = rf_validation::Validator::new(data_map);
                validator.rules(rules);
                validator.messages(messages);

                match validator.validate().await {
                    Ok(_) => Ok(self),
                    Err(errors) => Err(rf_validation::FormRequestError::ValidationFailed(
                        rf_validation::ValidationErrors::from_validator_errors(errors)
                    )),
                }
            }
        }
    };

    TokenStream::from(expanded)
}
