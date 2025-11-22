//! # rf-validation-derive
//!
//! Procedural macros for the rf-validation crate.
//!
//! This crate provides the `#[derive(Validate)]` macro that automatically generates
//! validation implementations based on field attributes.
//!
//! ## Example
//!
//! ```ignore
//! use rf_validation_derive::Validate;
//!
//! #[derive(Validate)]
//! struct CreatePost {
//!     #[validate(required, string, max = 255)]
//!     title: String,
//!
//!     #[validate(required, email, unique = "users")]
//!     email: String,
//!
//!     #[validate(nullable, url)]
//!     website: Option<String>,
//! }
//! ```

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod generator;
mod parser;
mod rules;

use generator::generate_validate_impl;
use parser::StructInfo;

/// Derive macro for automatic validation
///
/// Generates an implementation of the `Validate` trait based on field attributes.
///
/// ## Supported Attributes
///
/// ### String Rules
/// - `required`: Field must not be empty
/// - `string`: Field must be a valid string
/// - `email`: Valid email address
/// - `url`: Valid URL
/// - `ip`: Valid IP address (v4 or v6)
/// - `uuid`: Valid UUID
/// - `min = n`: Minimum length
/// - `max = n`: Maximum length
/// - `between = [min, max]`: Length must be between min and max
/// - `starts_with = "prefix"`: Must start with prefix
/// - `ends_with = "suffix"`: Must end with suffix
/// - `regex = "pattern"`: Must match regex pattern
/// - `alpha`: Only alphabetic characters
/// - `alpha_numeric`: Only alphanumeric characters
/// - `lowercase`: Only lowercase characters
/// - `uppercase`: Only uppercase characters
///
/// ### Number Rules
/// - `integer`: Must be an integer
/// - `numeric`: Must be numeric
/// - `digits = n`: Must have exactly n digits
/// - `digits_between = [min, max]`: Digits must be between min and max
/// - `positive`: Must be positive
/// - `negative`: Must be negative
///
/// ### Date Rules
/// - `date`: Must be a valid date
/// - `date_format = "format"`: Must match date format
/// - `before = "date"`: Must be before date
/// - `after = "date"`: Must be after date
/// - `between_dates = ["date1", "date2"]`: Must be between dates
///
/// ### Database Rules
/// - `exists = "table"`: Value must exist in table
/// - `exists = ["table", "column"]`: Value must exist in table.column
/// - `unique = "table"`: Value must be unique in table
/// - `unique = ["table", "column"]`: Value must be unique in table.column
/// - `unique_ignore = ["table", "column", id]`: Unique, ignoring specific id
///
/// ### Conditional Rules
/// - `required_if = ["field", "value"]`: Required if field equals value
/// - `required_unless = ["field", "value"]`: Required unless field equals value
/// - `required_with = "field"`: Required if field is present
///
/// ### Custom Messages
/// ```ignore
/// #[validate(required, message = "Title is required")]
/// #[validate(max = 255, message = "Title is too long")]
/// title: String,
/// ```
///
/// ### Nested Validation
/// ```ignore
/// #[validate]
/// tags: Vec<Tag>,
/// ```
///
/// ### Optional Fields
/// For `Option<T>` fields:
/// - If `required`: Error if None
/// - If not `required`: Skip validation if None
#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Parse struct and field information
    let struct_info = match StructInfo::from_derive_input(&input) {
        Ok(info) => info,
        Err(e) => return e.into_compile_error().into(),
    };

    // Generate the implementation
    let expanded = generate_validate_impl(&struct_info);

    TokenStream::from(expanded)
}
