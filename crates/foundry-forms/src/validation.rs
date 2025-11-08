//! Comprehensive Laravel-style validation system
//!
//! This module provides a powerful, fluent validation API similar to Laravel's validator.
//! It supports 20+ built-in validation rules, custom rules, and detailed error messages.
//!
//! # Example
//!
//! ```
//! use foundry_forms::validation::*;
//! use std::collections::HashMap;
//!
//! let mut data = HashMap::new();
//! data.insert("email".to_string(), "user@example.com".to_string());
//! data.insert("age".to_string(), "25".to_string());
//!
//! let result = Validator::new(ValidationData::from(data))
//!     .rule("email", vec![required(), email()])
//!     .rule("age", vec![required(), numeric(), min(18.0)])
//!     .validate();
//!
//! match result {
//!     Ok(_) => println!("Validation passed!"),
//!     Err(errors) => println!("Errors: {:?}", errors),
//! }
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Validation error for a single field
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub rule: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>, rule: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            rule: rule.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Collection of validation errors
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationErrors {
    pub errors: HashMap<String, Vec<String>>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.errors
            .entry(field.into())
            .or_insert_with(Vec::new)
            .push(message.into());
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn get(&self, field: &str) -> Option<&Vec<String>> {
        self.errors.get(field)
    }

    pub fn first(&self, field: &str) -> Option<&String> {
        self.errors.get(field).and_then(|v| v.first())
    }

    pub fn all(&self) -> Vec<String> {
        self.errors
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (field, messages) in &self.errors {
            for message in messages {
                writeln!(f, "{}: {}", field, message)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

// ============================================================================
// Validation Data
// ============================================================================

/// Data to be validated
#[derive(Debug, Clone)]
pub struct ValidationData {
    data: HashMap<String, String>,
}

impl ValidationData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
}

impl Default for ValidationData {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashMap<String, String>> for ValidationData {
    fn from(data: HashMap<String, String>) -> Self {
        Self { data }
    }
}

// ============================================================================
// Validation Rule Trait
// ============================================================================

/// Trait for validation rules
pub trait ValidationRuleTrait: Send + Sync {
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError>;
    fn name(&self) -> &str;
}

// ============================================================================
// Built-in Validation Rules
// ============================================================================

/// Rule: Field is required
#[derive(Debug, Clone)]
pub struct Required;

impl ValidationRuleTrait for Required {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if value.is_none() || value.unwrap().trim().is_empty() {
            return Err(ValidationError::new(
                field,
                format!("The {} field is required.", field),
                "required",
            ));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "required"
    }
}

/// Rule: Field is required if another field equals a value
#[derive(Debug, Clone)]
pub struct RequiredIf {
    other_field: String,
    value: String,
}

impl RequiredIf {
    pub fn new(other_field: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
            value: value.into(),
        }
    }
}

impl ValidationRuleTrait for RequiredIf {
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(other_value) = data.get(&self.other_field) {
            if other_value == self.value {
                if value.is_none() || value.unwrap().trim().is_empty() {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field is required when {} is {}.", field, self.other_field, self.value),
                        "required_if",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "required_if"
    }
}

/// Rule: Field is required if another field is present
#[derive(Debug, Clone)]
pub struct RequiredWith {
    other_field: String,
}

impl RequiredWith {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }
}

impl ValidationRuleTrait for RequiredWith {
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError> {
        if data.has(&self.other_field) {
            if value.is_none() || value.unwrap().trim().is_empty() {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field is required when {} is present.", field, self.other_field),
                    "required_with",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "required_with"
    }
}

/// Rule: Field must be a valid email address
#[derive(Debug, Clone)]
pub struct Email;

impl ValidationRuleTrait for Email {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                let email_regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
                if !email_regex.is_match(v) {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be a valid email address.", field),
                        "email",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "email"
    }
}

/// Rule: Field must be a valid URL
#[derive(Debug, Clone)]
pub struct Url;

impl ValidationRuleTrait for Url {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                let url_regex = Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
                if !url_regex.is_match(v) {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be a valid URL.", field),
                        "url",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "url"
    }
}

/// Rule: Field must be a valid IP address
#[derive(Debug, Clone)]
pub struct Ip;

impl ValidationRuleTrait for Ip {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                if v.parse::<std::net::IpAddr>().is_err() {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be a valid IP address.", field),
                        "ip",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "ip"
    }
}

/// Rule: Field must be a valid UUID
#[derive(Debug, Clone)]
pub struct Uuid;

impl ValidationRuleTrait for Uuid {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                let uuid_regex = Regex::new(
                    r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
                ).unwrap();
                if !uuid_regex.is_match(v) {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be a valid UUID.", field),
                        "uuid",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "uuid"
    }
}

/// Rule: Field must have minimum length
#[derive(Debug, Clone)]
pub struct MinLength {
    min: usize,
}

impl MinLength {
    pub fn new(min: usize) -> Self {
        Self { min }
    }
}

impl ValidationRuleTrait for MinLength {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if v.len() < self.min {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must be at least {} characters.", field, self.min),
                    "min",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "min"
    }
}

/// Rule: Field must have maximum length
#[derive(Debug, Clone)]
pub struct MaxLength {
    max: usize,
}

impl MaxLength {
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}

impl ValidationRuleTrait for MaxLength {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if v.len() > self.max {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must not exceed {} characters.", field, self.max),
                    "max",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "max"
    }
}

/// Rule: Field must be between min and max length
#[derive(Debug, Clone)]
pub struct Between {
    min: usize,
    max: usize,
}

impl Between {
    pub fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

impl ValidationRuleTrait for Between {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            let len = v.len();
            if len < self.min || len > self.max {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must be between {} and {} characters.", field, self.min, self.max),
                    "between",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "between"
    }
}

/// Rule: Field must be a specific size
#[derive(Debug, Clone)]
pub struct Size {
    size: usize,
}

impl Size {
    pub fn new(size: usize) -> Self {
        Self { size }
    }
}

impl ValidationRuleTrait for Size {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if v.len() != self.size {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must be exactly {} characters.", field, self.size),
                    "size",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "size"
    }
}

/// Rule: Field must be numeric
#[derive(Debug, Clone)]
pub struct Numeric;

impl ValidationRuleTrait for Numeric {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() && v.parse::<f64>().is_err() {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must be a number.", field),
                    "numeric",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "numeric"
    }
}

/// Rule: Field must be an integer
#[derive(Debug, Clone)]
pub struct Integer;

impl ValidationRuleTrait for Integer {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() && v.parse::<i64>().is_err() {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must be an integer.", field),
                    "integer",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "integer"
    }
}

/// Rule: Field must be a string
#[derive(Debug, Clone)]
pub struct StringRule;

impl ValidationRuleTrait for StringRule {
    fn validate(&self, _field: &str, _value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        // In Rust, everything from form data is already a string
        Ok(())
    }

    fn name(&self) -> &str {
        "string"
    }
}

/// Rule: Field must be a boolean
#[derive(Debug, Clone)]
pub struct Boolean;

impl ValidationRuleTrait for Boolean {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                let v_lower = v.to_lowercase();
                if !["true", "false", "1", "0", "yes", "no", "on", "off"].contains(&v_lower.as_str()) {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be a boolean value.", field),
                        "boolean",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "boolean"
    }
}

/// Rule: Field must be an array (comma-separated values)
#[derive(Debug, Clone)]
pub struct Array;

impl ValidationRuleTrait for Array {
    fn validate(&self, _field: &str, _value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        // For form data, we consider comma-separated values as arrays
        Ok(())
    }

    fn name(&self) -> &str {
        "array"
    }
}

/// Rule: Numeric field must be at least min
#[derive(Debug, Clone)]
pub struct Min {
    min: f64,
}

impl Min {
    pub fn new(min: f64) -> Self {
        Self { min }
    }
}

impl ValidationRuleTrait for Min {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                if let Ok(num) = v.parse::<f64>() {
                    if num < self.min {
                        return Err(ValidationError::new(
                            field,
                            format!("The {} field must be at least {}.", field, self.min),
                            "min",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "min"
    }
}

/// Rule: Numeric field must not exceed max
#[derive(Debug, Clone)]
pub struct Max {
    max: f64,
}

impl Max {
    pub fn new(max: f64) -> Self {
        Self { max }
    }
}

impl ValidationRuleTrait for Max {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                if let Ok(num) = v.parse::<f64>() {
                    if num > self.max {
                        return Err(ValidationError::new(
                            field,
                            format!("The {} field must not exceed {}.", field, self.max),
                            "max",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "max"
    }
}

/// Rule: Field must match confirmation field (e.g., password_confirmation)
#[derive(Debug, Clone)]
pub struct Confirmed;

impl ValidationRuleTrait for Confirmed {
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError> {
        let confirmation_field = format!("{}_confirmation", field);
        let confirmation_value = data.get(&confirmation_field);

        if value != confirmation_value {
            return Err(ValidationError::new(
                field,
                format!("The {} confirmation does not match.", field),
                "confirmed",
            ));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "confirmed"
    }
}

/// Rule: Field must be the same as another field
#[derive(Debug, Clone)]
pub struct Same {
    other_field: String,
}

impl Same {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }
}

impl ValidationRuleTrait for Same {
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError> {
        let other_value = data.get(&self.other_field);
        if value != other_value {
            return Err(ValidationError::new(
                field,
                format!("The {} field must match {}.", field, self.other_field),
                "same",
            ));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "same"
    }
}

/// Rule: Field must be different from another field
#[derive(Debug, Clone)]
pub struct Different {
    other_field: String,
}

impl Different {
    pub fn new(other_field: impl Into<String>) -> Self {
        Self {
            other_field: other_field.into(),
        }
    }
}

impl ValidationRuleTrait for Different {
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError> {
        let other_value = data.get(&self.other_field);
        if value == other_value && value.is_some() {
            return Err(ValidationError::new(
                field,
                format!("The {} field must be different from {}.", field, self.other_field),
                "different",
            ));
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "different"
    }
}

/// Rule: Field must be in a list of values
#[derive(Debug, Clone)]
pub struct In {
    values: Vec<String>,
}

impl In {
    pub fn new(values: Vec<String>) -> Self {
        Self { values }
    }
}

impl ValidationRuleTrait for In {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() && !self.values.contains(&v.to_string()) {
                return Err(ValidationError::new(
                    field,
                    format!("The selected {} is invalid.", field),
                    "in",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "in"
    }
}

/// Rule: Field must not be in a list of values
#[derive(Debug, Clone)]
pub struct NotIn {
    values: Vec<String>,
}

impl NotIn {
    pub fn new(values: Vec<String>) -> Self {
        Self { values }
    }
}

impl ValidationRuleTrait for NotIn {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() && self.values.contains(&v.to_string()) {
                return Err(ValidationError::new(
                    field,
                    format!("The selected {} is invalid.", field),
                    "not_in",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "not_in"
    }
}

/// Rule: Field must match a regex pattern
#[derive(Debug, Clone)]
pub struct RegexRule {
    pattern: String,
}

impl RegexRule {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }
}

impl ValidationRuleTrait for RegexRule {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                let re = Regex::new(&self.pattern).map_err(|_| {
                    ValidationError::new(field, "Invalid regex pattern", "regex")
                })?;
                if !re.is_match(v) {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field format is invalid.", field),
                        "regex",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "regex"
    }
}

/// Rule: Field must contain only alphabetic characters
#[derive(Debug, Clone)]
pub struct Alpha;

impl ValidationRuleTrait for Alpha {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() && !v.chars().all(|c| c.is_alphabetic()) {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must contain only letters.", field),
                    "alpha",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "alpha"
    }
}

/// Rule: Field must contain only alphanumeric characters
#[derive(Debug, Clone)]
pub struct AlphaNumeric;

impl ValidationRuleTrait for AlphaNumeric {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() && !v.chars().all(|c| c.is_alphanumeric()) {
                return Err(ValidationError::new(
                    field,
                    format!("The {} field must contain only letters and numbers.", field),
                    "alpha_numeric",
                ));
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "alpha_numeric"
    }
}

/// Rule: Field must be a valid date (YYYY-MM-DD)
#[derive(Debug, Clone)]
pub struct Date;

impl ValidationRuleTrait for Date {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                let date_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
                if !date_regex.is_match(v) {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be a valid date (YYYY-MM-DD).", field),
                        "date",
                    ));
                }
                // Basic validation for month and day ranges
                let parts: Vec<&str> = v.split('-').collect();
                if parts.len() == 3 {
                    if let (Ok(month), Ok(day)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                        if month == 0 || month > 12 || day == 0 || day > 31 {
                            return Err(ValidationError::new(
                                field,
                                format!("The {} field must be a valid date.", field),
                                "date",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "date"
    }
}

/// Rule: Field must be before a given date
#[derive(Debug, Clone)]
pub struct Before {
    date: String,
}

impl Before {
    pub fn new(date: impl Into<String>) -> Self {
        Self {
            date: date.into(),
        }
    }
}

impl ValidationRuleTrait for Before {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                if v >= &self.date {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be before {}.", field, self.date),
                        "before",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "before"
    }
}

/// Rule: Field must be after a given date
#[derive(Debug, Clone)]
pub struct After {
    date: String,
}

impl After {
    pub fn new(date: impl Into<String>) -> Self {
        Self {
            date: date.into(),
        }
    }
}

impl ValidationRuleTrait for After {
    fn validate(&self, field: &str, value: Option<&str>, _data: &ValidationData) -> Result<(), ValidationError> {
        if let Some(v) = value {
            if !v.is_empty() {
                if v <= &self.date {
                    return Err(ValidationError::new(
                        field,
                        format!("The {} field must be after {}.", field, self.date),
                        "after",
                    ));
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "after"
    }
}

// ============================================================================
// Helper Functions for Rule Creation
// ============================================================================

pub fn required() -> Box<dyn ValidationRuleTrait> {
    Box::new(Required)
}

pub fn required_if(other_field: impl Into<String>, value: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(RequiredIf::new(other_field, value))
}

pub fn required_with(other_field: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(RequiredWith::new(other_field))
}

pub fn email() -> Box<dyn ValidationRuleTrait> {
    Box::new(Email)
}

pub fn url() -> Box<dyn ValidationRuleTrait> {
    Box::new(Url)
}

pub fn ip() -> Box<dyn ValidationRuleTrait> {
    Box::new(Ip)
}

pub fn uuid() -> Box<dyn ValidationRuleTrait> {
    Box::new(Uuid)
}

pub fn min_length(min: usize) -> Box<dyn ValidationRuleTrait> {
    Box::new(MinLength::new(min))
}

pub fn max_length(max: usize) -> Box<dyn ValidationRuleTrait> {
    Box::new(MaxLength::new(max))
}

pub fn between(min: usize, max: usize) -> Box<dyn ValidationRuleTrait> {
    Box::new(Between::new(min, max))
}

pub fn size(size: usize) -> Box<dyn ValidationRuleTrait> {
    Box::new(Size::new(size))
}

pub fn numeric() -> Box<dyn ValidationRuleTrait> {
    Box::new(Numeric)
}

pub fn integer() -> Box<dyn ValidationRuleTrait> {
    Box::new(Integer)
}

pub fn string() -> Box<dyn ValidationRuleTrait> {
    Box::new(StringRule)
}

pub fn boolean() -> Box<dyn ValidationRuleTrait> {
    Box::new(Boolean)
}

pub fn array() -> Box<dyn ValidationRuleTrait> {
    Box::new(Array)
}

pub fn min(min: f64) -> Box<dyn ValidationRuleTrait> {
    Box::new(Min::new(min))
}

pub fn max(max: f64) -> Box<dyn ValidationRuleTrait> {
    Box::new(Max::new(max))
}

pub fn confirmed() -> Box<dyn ValidationRuleTrait> {
    Box::new(Confirmed)
}

pub fn same(other_field: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(Same::new(other_field))
}

pub fn different(other_field: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(Different::new(other_field))
}

pub fn in_list(values: Vec<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(In::new(values))
}

pub fn not_in(values: Vec<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(NotIn::new(values))
}

pub fn regex(pattern: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(RegexRule::new(pattern))
}

pub fn alpha() -> Box<dyn ValidationRuleTrait> {
    Box::new(Alpha)
}

pub fn alpha_numeric() -> Box<dyn ValidationRuleTrait> {
    Box::new(AlphaNumeric)
}

pub fn date() -> Box<dyn ValidationRuleTrait> {
    Box::new(Date)
}

pub fn before(date: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(Before::new(date))
}

pub fn after(date: impl Into<String>) -> Box<dyn ValidationRuleTrait> {
    Box::new(After::new(date))
}

// ============================================================================
// Validator
// ============================================================================

/// Main validator struct
pub struct Validator {
    data: ValidationData,
    rules: HashMap<String, Vec<Box<dyn ValidationRuleTrait>>>,
    messages: HashMap<String, String>,
}

impl Validator {
    /// Create a new validator with data
    pub fn new(data: ValidationData) -> Self {
        Self {
            data,
            rules: HashMap::new(),
            messages: HashMap::new(),
        }
    }

    /// Add validation rules for a field
    pub fn rule(mut self, field: impl Into<String>, rules: Vec<Box<dyn ValidationRuleTrait>>) -> Self {
        self.rules.insert(field.into(), rules);
        self
    }

    /// Add custom error message for a field
    pub fn message(mut self, field: impl Into<String>, message: impl Into<String>) -> Self {
        self.messages.insert(field.into(), message.into());
        self
    }

    /// Validate all fields
    pub fn validate(self) -> Result<ValidationData, ValidationErrors> {
        let mut errors = ValidationErrors::new();

        for (field, rules) in &self.rules {
            let value = self.data.get(field);

            for rule in rules {
                if let Err(error) = rule.validate(field, value, &self.data) {
                    // Use custom message if provided
                    let message = self.messages.get(field)
                        .cloned()
                        .unwrap_or(error.message);
                    errors.add(field, message);
                    break; // Stop at first error for this field
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(self.data)
        }
    }

    /// Validate and return only errors (doesn't consume self)
    pub fn check(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        for (field, rules) in &self.rules {
            let value = self.data.get(field);

            for rule in rules {
                if let Err(error) = rule.validate(field, value, &self.data) {
                    let message = self.messages.get(field)
                        .cloned()
                        .unwrap_or(error.message);
                    errors.add(field, message);
                    break;
                }
            }
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// Custom Rule Support
// ============================================================================

/// Custom validation rule
pub struct CustomRule<F>
where
    F: Fn(&str, Option<&str>, &ValidationData) -> Result<(), String> + Send + Sync,
{
    name: String,
    validator: F,
}

impl<F> CustomRule<F>
where
    F: Fn(&str, Option<&str>, &ValidationData) -> Result<(), String> + Send + Sync,
{
    pub fn new(name: impl Into<String>, validator: F) -> Self {
        Self {
            name: name.into(),
            validator,
        }
    }
}

impl<F> ValidationRuleTrait for CustomRule<F>
where
    F: Fn(&str, Option<&str>, &ValidationData) -> Result<(), String> + Send + Sync,
{
    fn validate(&self, field: &str, value: Option<&str>, data: &ValidationData) -> Result<(), ValidationError> {
        (self.validator)(field, value, data)
            .map_err(|msg| ValidationError::new(field, msg, &self.name))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Helper to create custom rules
pub fn custom<F>(name: impl Into<String>, validator: F) -> Box<dyn ValidationRuleTrait>
where
    F: Fn(&str, Option<&str>, &ValidationData) -> Result<(), String> + Send + Sync + 'static,
{
    Box::new(CustomRule::new(name, validator))
}

/// Macro to easily create custom validation rules
#[macro_export]
macro_rules! custom_rule {
    ($name:expr, |$field:ident, $value:ident, $data:ident| $body:expr) => {
        $crate::validation::custom($name, |$field, $value, $data| $body)
    };
}

// ============================================================================
// Backward Compatibility (for existing ValidationRule enum)
// ============================================================================

/// Legacy ValidationRule enum for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum ValidationRule {
    Required,
    MinLength(usize),
    MaxLength(usize),
    Pattern(String),
    Email,
    Url,
    Numeric,
    Integer,
    Min(f64),
    Max(f64),
    Between { min: f64, max: f64 },
    In { values: Vec<String> },
    Custom { message: String },
}

impl ValidationRule {
    pub fn validate(&self, field: &str, value: Option<&str>) -> Result<(), ValidationError> {
        let data = ValidationData::new();
        match self {
            ValidationRule::Required => Required.validate(field, value, &data),
            ValidationRule::MinLength(min) => MinLength::new(*min).validate(field, value, &data),
            ValidationRule::MaxLength(max) => MaxLength::new(*max).validate(field, value, &data),
            ValidationRule::Pattern(pattern) => RegexRule::new(pattern).validate(field, value, &data),
            ValidationRule::Email => Email.validate(field, value, &data),
            ValidationRule::Url => Url.validate(field, value, &data),
            ValidationRule::Numeric => Numeric.validate(field, value, &data),
            ValidationRule::Integer => Integer.validate(field, value, &data),
            ValidationRule::Min(min) => Min::new(*min).validate(field, value, &data),
            ValidationRule::Max(max) => Max::new(*max).validate(field, value, &data),
            ValidationRule::Between { min, max } => {
                // For backward compatibility, treat as numeric between
                if let Some(v) = value {
                    if !v.is_empty() {
                        if let Ok(num) = v.parse::<f64>() {
                            if num < *min || num > *max {
                                return Err(ValidationError::new(
                                    field,
                                    format!("The {} field must be between {} and {}.", field, min, max),
                                    "between",
                                ));
                            }
                        }
                    }
                }
                Ok(())
            }
            ValidationRule::In { values } => In::new(values.clone()).validate(field, value, &data),
            ValidationRule::Custom { message } => {
                Err(ValidationError::new(field, message, "custom"))
            }
        }
    }
}
