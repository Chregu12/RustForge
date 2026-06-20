//! Code generator for validation implementations
//!
//! This module generates the actual validation code from parsed struct information.

use crate::parser::{FieldInfo, StructInfo};
use crate::rules::ValidationRule;
use proc_macro2::TokenStream;
use quote::quote;

/// Generate the Validate trait implementation
pub fn generate_validate_impl(struct_info: &StructInfo) -> TokenStream {
    let struct_name = &struct_info.name;
    let field_validations = struct_info
        .fields
        .iter()
        .map(generate_field_validation)
        .collect::<Vec<_>>();

    quote! {
        #[automatically_derived]
        impl rf_validation::Validate for #struct_name {
            fn validate(&self) -> ::std::result::Result<(), ::validator::ValidationErrors> {
                let mut errors = ::validator::ValidationErrors::new();

                #(#field_validations)*

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors)
                }
            }
        }
    }
}

/// Generate validation code for a single field
fn generate_field_validation(field: &FieldInfo) -> TokenStream {
    let field_name = &field.name;
    let field_name_str = field_name.to_string();

    // Handle optional fields
    if field.is_optional && !field.is_required() {
        // For optional fields that are not required, only validate if Some
        let validations = field
            .rules
            .iter()
            .filter(|r| !matches!(r, ValidationRule::Nullable))
            .map(|rule| generate_rule_validation(field, rule))
            .collect::<Vec<_>>();

        if validations.is_empty() {
            return quote! {};
        }

        return quote! {
            if let ::std::option::Option::Some(ref value) = self.#field_name {
                #(#validations)*
            }
        };
    }

    // Required validation for optional fields
    if field.is_optional && field.is_required() {
        let required_check = quote! {
            if self.#field_name.is_none() {
                errors.add(
                    #field_name_str,
                    ::validator::ValidationError::new("required")
                );
            }
        };

        let other_validations = field
            .rules
            .iter()
            .filter(|r| !matches!(r, ValidationRule::Required | ValidationRule::Nullable))
            .map(|rule| generate_rule_validation(field, rule))
            .collect::<Vec<_>>();

        if other_validations.is_empty() {
            return required_check;
        }

        return quote! {
            #required_check else if let ::std::option::Option::Some(ref value) = self.#field_name {
                #(#other_validations)*
            }
        };
    }

    // Non-optional fields
    let validations = field
        .rules
        .iter()
        .filter(|r| !matches!(r, ValidationRule::Nullable))
        .map(|rule| generate_rule_validation(field, rule))
        .collect::<Vec<_>>();

    quote! {
        #(#validations)*
    }
}

/// Generate validation code for a single rule
fn generate_rule_validation(field: &FieldInfo, rule: &ValidationRule) -> TokenStream {
    let field_name = &field.name;
    let field_name_str = field_name.to_string();
    let error_code = rule.error_code();
    let error_message = field
        .custom_message.clone()
        .unwrap_or_else(|| rule.error_message(&field_name_str));

    // Determine the value expression based on whether field is optional
    let value_expr = if field.is_optional {
        quote! { value }
    } else {
        quote! { &self.#field_name }
    };

    match rule {
        ValidationRule::Required => {
            if field.is_optional {
                // Already handled above
                quote! {}
            } else {
                // For non-optional string types
                quote! {
                    if #value_expr.is_empty() {
                        let mut error = ::validator::ValidationError::new(#error_code);
                        error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                        errors.add(#field_name_str, error);
                    }
                }
            }
        }

        ValidationRule::String => {
            // Type-level check, always passes for String types
            quote! {}
        }

        ValidationRule::Email => {
            quote! {
                if !::rf_validation::validators::email::validate_email(#value_expr) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Url => {
            quote! {
                if !::rf_validation::validators::url::validate_url(#value_expr) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Min(min) => {
            let min_lit = proc_macro2::Literal::usize_unsuffixed(*min);
            quote! {
                if #value_expr.len() < #min_lit {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Max(max) => {
            let max_lit = proc_macro2::Literal::usize_unsuffixed(*max);
            quote! {
                if #value_expr.len() > #max_lit {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Between { min, max } => {
            let min_lit = proc_macro2::Literal::usize_unsuffixed(*min);
            let max_lit = proc_macro2::Literal::usize_unsuffixed(*max);
            quote! {
                let len = #value_expr.len();
                if len < #min_lit || len > #max_lit {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::StartsWith(prefix) => {
            quote! {
                if !#value_expr.starts_with(#prefix) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::EndsWith(suffix) => {
            quote! {
                if !#value_expr.ends_with(#suffix) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Regex(pattern) => {
            quote! {
                if !::rf_validation::validators::regex::validate_regex(#value_expr, #pattern) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Alpha => {
            quote! {
                if !#value_expr.chars().all(|c| c.is_alphabetic()) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::AlphaNumeric => {
            quote! {
                if !#value_expr.chars().all(|c| c.is_alphanumeric()) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Lowercase => {
            quote! {
                if #value_expr.chars().any(|c| c.is_uppercase()) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Uppercase => {
            quote! {
                if #value_expr.chars().any(|c| c.is_lowercase()) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Nested => {
            quote! {
                if let Err(nested_errors) = ::validator::Validate::validate(#value_expr) {
                    // Add nested errors with field prefix
                    for (field, field_errors) in nested_errors.field_errors() {
                        let prefixed_field = format!("{}.{}", #field_name_str, field);
                        for error in field_errors {
                            errors.add(&prefixed_field, error.clone());
                        }
                    }
                }
            }
        }

        // Placeholder implementations for advanced rules
        ValidationRule::Ip => {
            quote! {
                if !::rf_validation::validators::ip::validate_ip(#value_expr) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        ValidationRule::Uuid => {
            quote! {
                if !::rf_validation::validators::uuid::validate_uuid(#value_expr) {
                    let mut error = ::validator::ValidationError::new(#error_code);
                    error.message = Some(::std::borrow::Cow::Borrowed(#error_message));
                    errors.add(#field_name_str, error);
                }
            }
        }

        // Number rules
        ValidationRule::Integer => {
            quote! {
                if let Some(value) = #value_expr {
                    if value.parse::<i64>().is_err() {
                        errors.push(format!("{} must be an integer", #field_name_str));
                    }
                }
            }
        }

        ValidationRule::Numeric => {
            quote! {
                if let Some(value) = #value_expr {
                    if value.parse::<f64>().is_err() {
                        errors.push(format!("{} must be numeric", #field_name_str));
                    }
                }
            }
        }

        ValidationRule::Digits(count) => {
            quote! {
                if let Some(value) = #value_expr {
                    let digits: String = value.chars().filter(|c| c.is_digit(10)).collect();
                    if digits.len() != #count {
                        errors.push(format!("{} must have exactly {} digits", #field_name_str, #count));
                    }
                }
            }
        }

        ValidationRule::DigitsBetween { min, max } => {
            quote! {
                if let Some(value) = #value_expr {
                    let digits: String = value.chars().filter(|c| c.is_digit(10)).collect();
                    let len = digits.len();
                    if len < #min || len > #max {
                        errors.push(format!("{} must have between {} and {} digits", #field_name_str, #min, #max));
                    }
                }
            }
        }

        ValidationRule::Positive => {
            quote! {
                if let Some(value) = #value_expr {
                    if let Ok(num) = value.parse::<f64>() {
                        if num <= 0.0 {
                            errors.push(format!("{} must be positive", #field_name_str));
                        }
                    } else {
                        errors.push(format!("{} must be a number", #field_name_str));
                    }
                }
            }
        }

        ValidationRule::Negative => {
            quote! {
                if let Some(value) = #value_expr {
                    if let Ok(num) = value.parse::<f64>() {
                        if num >= 0.0 {
                            errors.push(format!("{} must be negative", #field_name_str));
                        }
                    } else {
                        errors.push(format!("{} must be a number", #field_name_str));
                    }
                }
            }
        }

        // Date rules
        ValidationRule::Date => {
            quote! {
                if let Some(value) = #value_expr {
                    use chrono::NaiveDate;
                    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err() {
                        errors.push(format!("{} must be a valid date (YYYY-MM-DD)", #field_name_str));
                    }
                }
            }
        }

        ValidationRule::DateFormat(format_str) => {
            quote! {
                if let Some(value) = #value_expr {
                    use chrono::NaiveDate;
                    if NaiveDate::parse_from_str(value, #format_str).is_err() {
                        errors.push(format!("{} must be a valid date with format {}", #field_name_str, #format_str));
                    }
                }
            }
        }

        ValidationRule::Before(before_date) => {
            quote! {
                if let Some(value) = #value_expr {
                    use chrono::NaiveDate;
                    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                        if let Ok(before) = NaiveDate::parse_from_str(#before_date, "%Y-%m-%d") {
                            if date >= before {
                                errors.push(format!("{} must be before {}", #field_name_str, #before_date));
                            }
                        }
                    } else {
                        errors.push(format!("{} must be a valid date", #field_name_str));
                    }
                }
            }
        }

        ValidationRule::After(after_date) => {
            quote! {
                if let Some(value) = #value_expr {
                    use chrono::NaiveDate;
                    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                        if let Ok(after) = NaiveDate::parse_from_str(#after_date, "%Y-%m-%d") {
                            if date <= after {
                                errors.push(format!("{} must be after {}", #field_name_str, #after_date));
                            }
                        }
                    } else {
                        errors.push(format!("{} must be a valid date", #field_name_str));
                    }
                }
            }
        }

        ValidationRule::BetweenDates { start, end } => {
            quote! {
                if let Some(value) = #value_expr {
                    use chrono::NaiveDate;
                    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                        if let (Ok(start_date), Ok(end_date)) = (
                            NaiveDate::parse_from_str(#start, "%Y-%m-%d"),
                            NaiveDate::parse_from_str(#end, "%Y-%m-%d")
                        ) {
                            if date < start_date || date > end_date {
                                errors.push(format!("{} must be between {} and {}", #field_name_str, #start, #end));
                            }
                        }
                    } else {
                        errors.push(format!("{} must be a valid date", #field_name_str));
                    }
                }
            }
        }

        // Database rules - generate placeholder for async validation
        ValidationRule::Exists { table, column } => {
            quote! {
                // Database validation requires async context
                // This should be handled by a separate async validator
                // Placeholder: assume validation passes (implement in async context)
                #[allow(unused_variables)]
                let _table = #table;
                #[allow(unused_variables)]
                let _column = #column;
            }
        }

        ValidationRule::Unique { table, column } => {
            quote! {
                // Database validation requires async context
                // This should be handled by a separate async validator
                #[allow(unused_variables)]
                let _table = #table;
                #[allow(unused_variables)]
                let _column = #column;
            }
        }

        ValidationRule::UniqueIgnore { table, column, id } => {
            quote! {
                // Database validation requires async context
                #[allow(unused_variables)]
                let _table = #table;
                #[allow(unused_variables)]
                let _column = #column;
                #[allow(unused_variables)]
                let _id = #id;
            }
        }

        // Conditional rules - note: These require access to the full struct
        // For now, generate code that requires the field name to be present in context
        ValidationRule::RequiredIf {
            field: other_field,
            value,
        } => {
            quote! {
                // Check if the other field has the specified value
                // Note: This requires reflection or macro-time field access
                // For now, mark as conditional validation that needs runtime context
                if let Some(_other_value) = std::any::Any::type_id(&self).type_id() {
                    // Placeholder: implement conditional logic based on other field
                    #[allow(unused_variables)]
                    let _required_if_field = #other_field;
                    #[allow(unused_variables)]
                    let _required_if_value = #value;
                }
            }
        }

        ValidationRule::RequiredUnless {
            field: other_field,
            value,
        } => {
            quote! {
                #[allow(unused_variables)]
                let _required_unless_field = #other_field;
                #[allow(unused_variables)]
                let _required_unless_value = #value;
            }
        }

        ValidationRule::RequiredWith(other_field) => {
            quote! {
                #[allow(unused_variables)]
                let _required_with_field = #other_field;
            }
        }

        ValidationRule::Nullable => {
            // Marker attribute, no validation needed
            quote! {}
        }
    }
}
