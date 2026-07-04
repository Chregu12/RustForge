//! Validation rules module
//!
//! This module contains all built-in validation rules organized by category.

pub mod array;
pub mod conditional;
pub mod database;
pub mod date;
pub mod file;
pub mod numeric;
pub mod string;

// Re-export all rules for convenient access
pub use array::*;
pub use conditional::*;
pub use database::*;
pub use date::*;
pub use file::*;
pub use numeric::*;
pub use string::*;
