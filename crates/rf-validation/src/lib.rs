//! # rf-validation - Validation & Forms
//!
//! Production-ready validation for web applications with two validation approaches:
//!
//! 1. **Declarative Validation**: Use `#[derive(Validate)]` from validator crate
//! 2. **Rule-Based Validation**: Flexible Validator with 50+ built-in rules
//!
//! ## Features
//!
//! - **50+ Built-in Rules**: Email, URL, length, range, regex, date, database, and more
//! - **Axum Integration**: ValidatedJson extractor with automatic validation
//! - **Field-Level Errors**: Detailed error messages per field
//! - **Custom Messages**: Override default error messages
//! - **Conditional Rules**: RequiredIf, RequiredWith, Same, Different, etc.
//! - **Database Rules**: Unique and Exists validation against database
//! - **Type-Safe**: Compile-time validation rule checking
//! - **RFC 7807 Compatible**: Standard error responses
//!
//! ## Quick Start - Declarative Validation
//!
//! ```ignore
//! use rf_validation::{ValidatedJson, Validate};
//! use serde::Deserialize;
//! use axum::{routing::post, Router};
//!
//! #[derive(Debug, Deserialize, Validate)]
//! struct CreateUser {
//!     #[validate(email)]
//!     email: String,
//!
//!     #[validate(length(min = 8, max = 128))]
//!     password: String,
//!
//!     #[validate(length(min = 2, max = 100))]
//!     name: String,
//! }
//!
//! async fn create_user(
//!     ValidatedJson(user): ValidatedJson<CreateUser>,
//! ) -> String {
//!     format!("Created user: {}", user.email)
//! }
//!
//! # async fn example() {
//! let app = Router::new().route("/users", post(create_user));
//! # }
//! ```
//!
//! ## Quick Start - Rule-Based Validation
//!
//! ```ignore
//! use rf_validation::{Validator, rules::*};
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! let mut data = HashMap::new();
//! data.insert("email".to_string(), json!("user@example.com"));
//! data.insert("age".to_string(), json!(25));
//!
//! let mut validator = Validator::new(data);
//!
//! validator.rules(HashMap::from([
//!     ("email", vec![
//!         Box::new(RequiredRule) as Box<dyn Rule>,
//!         Box::new(EmailRule),
//!     ]),
//!     ("age", vec![
//!         Box::new(RequiredRule) as Box<dyn Rule>,
//!         Box::new(IntegerRule),
//!         Box::new(MinRule::new(18)),
//!         Box::new(MaxRule::new(120)),
//!     ]),
//! ]));
//!
//! validator.messages(HashMap::from([
//!     ("email.required", "Email is required"),
//!     ("age.min", "You must be at least 18 years old"),
//! ]));
//!
//! match validator.validate().await {
//!     Ok(validated) => println!("Valid: {:?}", validated.all()),
//!     Err(errors) => println!("Errors: {:?}", errors),
//! }
//! ```
//!
//! ## Available Rules
//!
//! ### String Rules (15+)
//! - RequiredRule, StringRule, EmailRule, UrlRule, IpRule, UuidRule
//! - MinLengthRule, MaxLengthRule, BetweenLengthRule
//! - StartsWithRule, EndsWithRule, RegexRule
//! - AlphaRule, AlphaNumericRule, LowercaseRule, UppercaseRule
//!
//! ### Numeric Rules (9+)
//! - IntegerRule, NumericRule
//! - MinRule, MaxRule, BetweenRule
//! - DigitsRule, DigitsBetweenRule
//! - PositiveRule, NegativeRule
//!
//! ### Date Rules (7+)
//! - DateRule, DateFormatRule
//! - BeforeRule, AfterRule, BetweenDatesRule
//! - BeforeOrEqualRule, AfterOrEqualRule
//!
//! ### Array Rules (6+)
//! - ArrayRule, InRule, NotInRule, DistinctRule
//! - MinArraySizeRule, MaxArraySizeRule
//!
//! ### Database Rules (4)
//! - ExistsRule, UniqueRule
//! - SimpleExistsRule, SimpleUniqueRule
//!
//! ### Conditional Rules (6)
//! - RequiredIfRule, RequiredUnlessRule
//! - RequiredWithRule, RequiredWithoutRule
//! - SameRule, DifferentRule
//!
//! ## Error Responses
//!
//! Validation errors are returned as RFC 7807-compatible JSON:
//!
//! ```json
//! {
//!   "type": "validation-failed",
//!   "title": "Validation Failed",
//!   "status": 422,
//!   "detail": "One or more fields failed validation",
//!   "errors": {
//!     "email": [
//!       {
//!         "code": "email",
//!         "message": "Invalid email address"
//!       }
//!     ]
//!   }
//! }
//! ```

pub mod error;
pub mod extractor;
pub mod form_request;
pub mod rules;
pub mod validator;
pub mod validators;

// Re-export main types
pub use error::{FieldError, ValidationErrors};
// Byte-size helpers for the file/image size DSL (`max(mb(5))`, `min(kb(1))`).
pub use rules::file::{kb, mb};
pub use extractor::{ValidatedJson, ValidationRejection};
pub use form_request::{
    FormRequest, FormRequestError, FormRequestResult, MessagesBuilder, RulesBuilder, Validated,
    ValidationMessages, ValidationRules,
};
pub use validator::{Rule, RuleResult, ValidatedData, Validator};

// Re-export validator traits and derive macro from external crate
pub use ::validator::Validate;

// Re-export derive macro from rf-validation-derive
#[cfg(feature = "derive")]
pub use rf_validation_derive::Validate as ValidateDerive;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        error::{FieldError, ValidationErrors},
        extractor::{ValidatedJson, ValidationRejection},
        form_request::{
            FormRequest, FormRequestError, FormRequestResult, MessagesBuilder, RulesBuilder,
            Validated, ValidationMessages, ValidationRules,
        },
        rules,
        validator::{Rule, RuleResult, ValidatedData, Validator},
        validators,
    };
    pub use ::validator::Validate;

    #[cfg(feature = "derive")]
    pub use rf_validation_derive::Validate as ValidateDerive;
}
