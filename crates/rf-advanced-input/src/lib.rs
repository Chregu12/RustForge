//! # Foundry Advanced Input
//!
//! Advanced input handling with array support and comprehensive validation.
//!
//! ## Features
//!
//! - **Array Parsing**: Parse CSV-style arrays from command-line options
//! - **Input Validation**: Form-request-like validation with built-in rules
//! - **Custom Validators**: Define your own validation logic
//! - **Error Messages**: Customizable error messages per rule
//!
//! ## Example
//!
//! ```rust
//! use foundry_advanced_input::{Input, ValidationRule};
//!
//! let mut input = Input::new();
//! input.set("email", "user@example.com");
//! input.set("age", "25");
//! input.set_array("tags", vec!["rust", "web"]);
//!
//! let rules = vec![
//!     ("email", vec![ValidationRule::Required, ValidationRule::Email]),
//!     ("age", vec![ValidationRule::Required, ValidationRule::Numeric]),
//!     ("tags", vec![ValidationRule::Array, ValidationRule::MinLength(1)]),
//! ];
//!
//! let validated = input.validate(&rules).unwrap();
//! ```

mod array;
mod error;
mod input;
mod validator;

pub use array::{parse_array, parse_numeric_array};
pub use error::{ValidationError, ValidationResult};
pub use input::Input;
pub use validator::{ValidationRule, Validator};
