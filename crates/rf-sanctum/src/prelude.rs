//! # rf-sanctum Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-sanctum.
//!
//! ## Usage
//!
//! ```rust
//! use rf_sanctum::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: token::{PersonalAccessToken, NewToken};
pub use crate:: tokenable::Tokenable;
pub use crate:: auth::{SanctumAuth, LoadFromToken};
pub use crate:: abilities::{Ability, AbilityChecker};
pub use crate:: errors::SanctumError;
pub use crate:: models::Model as PersonalAccessTokenModel;
pub use crate:: repository::TokenRepository;
