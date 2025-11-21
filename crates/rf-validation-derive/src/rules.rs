//! Validation rules definitions
//!
//! This module defines all supported validation rules and their parameters.

use syn::{Expr, Lit, Meta};

/// A validation rule with its parameters
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ValidationRule {
    // String Rules
    Required,
    String,
    Email,
    Url,
    Ip,
    Uuid,
    Min(usize),
    Max(usize),
    Between { min: usize, max: usize },
    StartsWith(String),
    EndsWith(String),
    Regex(String),
    Alpha,
    AlphaNumeric,
    Lowercase,
    Uppercase,

    // Number Rules
    Integer,
    Numeric,
    Digits(usize),
    DigitsBetween { min: usize, max: usize },
    Positive,
    Negative,

    // Date Rules
    Date,
    DateFormat(String),
    Before(String),
    After(String),
    BetweenDates { start: String, end: String },

    // Database Rules
    Exists { table: String, column: Option<String> },
    Unique { table: String, column: Option<String> },
    UniqueIgnore { table: String, column: String, id: String },

    // Conditional Rules
    RequiredIf { field: String, value: String },
    RequiredUnless { field: String, value: String },
    RequiredWith(String),

    // Nested Validation
    Nested,

    // Optional marker
    Nullable,
}

impl ValidationRule {
    /// Parse validation rules from a Meta
    pub fn from_meta(meta: &Meta) -> Result<Vec<Self>, syn::Error> {
        let mut rules = Vec::new();

        match meta {
            Meta::Path(path) => {
                // Simple attributes like #[validate(required)]
                if let Some(ident) = path.get_ident() {
                    let rule = Self::parse_simple_rule(ident.to_string())?;
                    rules.push(rule);
                }
            }
            Meta::List(list) => {
                // Parse nested attributes like #[validate(required, email, max = 255)]
                for nested in list.parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
                )? {
                    rules.extend(Self::from_meta(&nested)?);
                }
            }
            Meta::NameValue(nv) => {
                // Parse name-value pairs like #[validate(max = 255)]
                if let Some(ident) = nv.path.get_ident() {
                    let rule = Self::parse_name_value_rule(
                        ident.to_string(),
                        &nv.value
                    )?;
                    rules.push(rule);
                }
            }
        }

        Ok(rules)
    }

    /// Parse simple rules (no parameters)
    fn parse_simple_rule(name: String) -> Result<Self, syn::Error> {
        match name.as_str() {
            "required" => Ok(Self::Required),
            "string" => Ok(Self::String),
            "email" => Ok(Self::Email),
            "url" => Ok(Self::Url),
            "ip" => Ok(Self::Ip),
            "uuid" => Ok(Self::Uuid),
            "alpha" => Ok(Self::Alpha),
            "alpha_numeric" => Ok(Self::AlphaNumeric),
            "lowercase" => Ok(Self::Lowercase),
            "uppercase" => Ok(Self::Uppercase),
            "integer" => Ok(Self::Integer),
            "numeric" => Ok(Self::Numeric),
            "positive" => Ok(Self::Positive),
            "negative" => Ok(Self::Negative),
            "date" => Ok(Self::Date),
            "nullable" => Ok(Self::Nullable),
            _ => Err(syn::Error::new_spanned(
                name.clone(),
                format!("Unknown validation rule: {}", name)
            ))
        }
    }

    /// Parse name-value rules (with parameters)
    fn parse_name_value_rule(name: String, value: &Expr) -> Result<Self, syn::Error> {
        match name.as_str() {
            "min" => {
                let val = Self::extract_int(value)?;
                Ok(Self::Min(val))
            }
            "max" => {
                let val = Self::extract_int(value)?;
                Ok(Self::Max(val))
            }
            "digits" => {
                let val = Self::extract_int(value)?;
                Ok(Self::Digits(val))
            }
            "starts_with" => {
                let val = Self::extract_string(value)?;
                Ok(Self::StartsWith(val))
            }
            "ends_with" => {
                let val = Self::extract_string(value)?;
                Ok(Self::EndsWith(val))
            }
            "regex" => {
                let val = Self::extract_string(value)?;
                Ok(Self::Regex(val))
            }
            "date_format" => {
                let val = Self::extract_string(value)?;
                Ok(Self::DateFormat(val))
            }
            "before" => {
                let val = Self::extract_string(value)?;
                Ok(Self::Before(val))
            }
            "after" => {
                let val = Self::extract_string(value)?;
                Ok(Self::After(val))
            }
            "exists" => {
                let (table, column) = Self::extract_table_column(value)?;
                Ok(Self::Exists { table, column })
            }
            "unique" => {
                let (table, column) = Self::extract_table_column(value)?;
                Ok(Self::Unique { table, column })
            }
            "required_with" => {
                let field = Self::extract_string(value)?;
                Ok(Self::RequiredWith(field))
            }
            _ => Err(syn::Error::new_spanned(
                name.clone(),
                format!("Unknown validation rule: {}", name)
            ))
        }
    }

    /// Extract integer value from expression
    fn extract_int(expr: &Expr) -> Result<usize, syn::Error> {
        match expr {
            Expr::Lit(lit) => {
                if let Lit::Int(int) = &lit.lit {
                    int.base10_parse()
                } else {
                    Err(syn::Error::new_spanned(expr, "Expected integer literal"))
                }
            }
            _ => Err(syn::Error::new_spanned(expr, "Expected integer literal"))
        }
    }

    /// Extract string value from expression
    fn extract_string(expr: &Expr) -> Result<String, syn::Error> {
        match expr {
            Expr::Lit(lit) => {
                if let Lit::Str(s) = &lit.lit {
                    Ok(s.value())
                } else {
                    Err(syn::Error::new_spanned(expr, "Expected string literal"))
                }
            }
            _ => Err(syn::Error::new_spanned(expr, "Expected string literal"))
        }
    }

    /// Extract table and optional column from expression
    fn extract_table_column(expr: &Expr) -> Result<(String, Option<String>), syn::Error> {
        match expr {
            Expr::Lit(lit) => {
                if let Lit::Str(s) = &lit.lit {
                    Ok((s.value(), None))
                } else {
                    Err(syn::Error::new_spanned(expr, "Expected string literal"))
                }
            }
            Expr::Array(arr) => {
                if arr.elems.len() == 2 {
                    let table = Self::extract_string(&arr.elems[0])?;
                    let column = Self::extract_string(&arr.elems[1])?;
                    Ok((table, Some(column)))
                } else {
                    Err(syn::Error::new_spanned(
                        expr,
                        "Expected [table, column] array"
                    ))
                }
            }
            _ => Err(syn::Error::new_spanned(
                expr,
                "Expected string or [table, column] array"
            ))
        }
    }

    /// Get the error message for this rule
    pub fn error_message(&self, field_name: &str) -> String {
        match self {
            Self::Required => format!("The {} field is required", field_name),
            Self::String => format!("The {} must be a string", field_name),
            Self::Email => format!("The {} must be a valid email address", field_name),
            Self::Url => format!("The {} must be a valid URL", field_name),
            Self::Ip => format!("The {} must be a valid IP address", field_name),
            Self::Uuid => format!("The {} must be a valid UUID", field_name),
            Self::Min(n) => format!("The {} must be at least {} characters", field_name, n),
            Self::Max(n) => format!("The {} may not be greater than {} characters", field_name, n),
            Self::Between { min, max } => {
                format!("The {} must be between {} and {} characters", field_name, min, max)
            }
            Self::StartsWith(prefix) => {
                format!("The {} must start with {}", field_name, prefix)
            }
            Self::EndsWith(suffix) => {
                format!("The {} must end with {}", field_name, suffix)
            }
            Self::Regex(_) => format!("The {} format is invalid", field_name),
            Self::Alpha => format!("The {} may only contain letters", field_name),
            Self::AlphaNumeric => {
                format!("The {} may only contain letters and numbers", field_name)
            }
            Self::Lowercase => format!("The {} must be lowercase", field_name),
            Self::Uppercase => format!("The {} must be uppercase", field_name),
            Self::Integer => format!("The {} must be an integer", field_name),
            Self::Numeric => format!("The {} must be a number", field_name),
            Self::Digits(n) => format!("The {} must be {} digits", field_name, n),
            Self::DigitsBetween { min, max } => {
                format!("The {} must be between {} and {} digits", field_name, min, max)
            }
            Self::Positive => format!("The {} must be positive", field_name),
            Self::Negative => format!("The {} must be negative", field_name),
            Self::Date => format!("The {} must be a valid date", field_name),
            Self::DateFormat(format) => {
                format!("The {} must be a valid date in format {}", field_name, format)
            }
            Self::Before(date) => format!("The {} must be before {}", field_name, date),
            Self::After(date) => format!("The {} must be after {}", field_name, date),
            Self::BetweenDates { start, end } => {
                format!("The {} must be between {} and {}", field_name, start, end)
            }
            Self::Exists { table, column } => {
                if let Some(col) = column {
                    format!("The selected {} does not exist in {}.{}", field_name, table, col)
                } else {
                    format!("The selected {} does not exist in {}", field_name, table)
                }
            }
            Self::Unique { table, column } => {
                if let Some(col) = column {
                    format!("The {} has already been taken in {}.{}", field_name, table, col)
                } else {
                    format!("The {} has already been taken in {}", field_name, table)
                }
            }
            Self::UniqueIgnore { table, column, .. } => {
                format!("The {} has already been taken in {}.{}", field_name, table, column)
            }
            Self::RequiredIf { field, value } => {
                format!("The {} field is required when {} is {}", field_name, field, value)
            }
            Self::RequiredUnless { field, value } => {
                format!("The {} field is required unless {} is {}", field_name, field, value)
            }
            Self::RequiredWith(field) => {
                format!("The {} field is required when {} is present", field_name, field)
            }
            Self::Nested => format!("The {} failed nested validation", field_name),
            Self::Nullable => String::new(), // Not an error
        }
    }

    /// Get the error code for this rule
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::String => "string",
            Self::Email => "email",
            Self::Url => "url",
            Self::Ip => "ip",
            Self::Uuid => "uuid",
            Self::Min(_) => "min",
            Self::Max(_) => "max",
            Self::Between { .. } => "between",
            Self::StartsWith(_) => "starts_with",
            Self::EndsWith(_) => "ends_with",
            Self::Regex(_) => "regex",
            Self::Alpha => "alpha",
            Self::AlphaNumeric => "alpha_numeric",
            Self::Lowercase => "lowercase",
            Self::Uppercase => "uppercase",
            Self::Integer => "integer",
            Self::Numeric => "numeric",
            Self::Digits(_) => "digits",
            Self::DigitsBetween { .. } => "digits_between",
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Date => "date",
            Self::DateFormat(_) => "date_format",
            Self::Before(_) => "before",
            Self::After(_) => "after",
            Self::BetweenDates { .. } => "between_dates",
            Self::Exists { .. } => "exists",
            Self::Unique { .. } => "unique",
            Self::UniqueIgnore { .. } => "unique",
            Self::RequiredIf { .. } => "required_if",
            Self::RequiredUnless { .. } => "required_unless",
            Self::RequiredWith(_) => "required_with",
            Self::Nested => "nested",
            Self::Nullable => "nullable",
        }
    }
}
