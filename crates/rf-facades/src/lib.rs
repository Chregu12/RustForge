//! Facade pattern for RustForge
//!
//! Provides Laravel-style facades for common services.
//!
//! # Quick Start
//!
//! ```rust
//! use rf_facades::{facade, Facade};
//!
//! // Define a facade
//! facade!(DB, DatabaseFacade, {
//!     // Facade implementation
//! });
//!
//! // Use the facade
//! // DB!().query("SELECT * FROM users");
//! ```

pub mod macros;
pub mod facades;

/// Trait for facade implementations
pub trait Facade: Send + Sync {
    /// Get the underlying service instance
    fn instance() -> &'static Self;
}

/// Macro to define a facade
#[macro_export]
macro_rules! facade {
    ($name:ident, $type:ty) => {
        #[allow(non_snake_case)]
        pub fn $name() -> &'static $type {
            use $crate::Facade;
            <$type>::instance()
        }
    };
}

/// Shorthand macro for accessing facades
#[macro_export]
macro_rules! DB {
    () => {
        $crate::facades::db()
    };
}

#[macro_export]
macro_rules! Cache {
    () => {
        $crate::facades::cache()
    };
}

#[macro_export]
macro_rules! Log {
    () => {
        $crate::facades::log()
    };
}

#[macro_export]
macro_rules! Config {
    () => {
        $crate::facades::config()
    };
}
