//! # rf-validation Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-validation.
//!
//! ## Usage
//!
//! ```rust
//! use rf_validation::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: error::{FieldError, ValidationErrors};
pub use crate:: extractor::{ValidatedJson, ValidationRejection};
pub use crate:: form_request::{
pub use crate:: validator::{Rule, RuleResult, ValidatedData, Validator};
pub use crate:: ::validator::Validate;
pub use crate:: rf_validation_derive::Validate as ValidateDerive;
