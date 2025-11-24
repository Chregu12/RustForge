//! # rf-global-helpers
//!
//! Laravel-style global helper functions for the RustForge framework.
//!
//! This crate provides convenient global functions for common tasks like
//! redirects, hashing, events, CSRF protection, and translations.
//!
//! ## Features
//!
//! - **Redirects**: `redirect()`, `back()` functions
//! - **Password Hashing**: `Hash` facade for bcrypt/argon2
//! - **Events**: `event()` function for dispatching events
//! - **CSRF**: `csrf_token()` for generating CSRF tokens
//! - **Translation**: `__()` for internationalization
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_global_helpers::{redirect, Hash};
//!
//! // Redirect to a path
//! let response = redirect("/dashboard");
//!
//! // Hash a password
//! let hash = Hash::make("password123");
//!
//! // Check a password
//! let is_valid = Hash::check("password123", &hash);
//! ```

pub mod csrf;
pub mod event;
pub mod hash;
pub mod redirect;
pub mod translation;

pub use csrf::{csrf_token, csrf_field};
pub use event::{event, Event};
pub use hash::{Hash, HashAlgorithm, HashInfo};
pub use redirect::{redirect, back, RedirectResponse};
pub use translation::__;
