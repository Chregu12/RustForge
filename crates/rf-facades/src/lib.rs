//! Facade pattern for RustForge
//!
//! Provides Laravel-style facades for common services.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use rf_facades::{facade, Facade};
//!
//! struct DatabaseFacade;
//!
//! impl Facade for DatabaseFacade {
//!     fn instance() -> &'static Self {
//!         static INSTANCE: DatabaseFacade = DatabaseFacade;
//!         &INSTANCE
//!     }
//! }
//!
//! // Define a facade
//! facade!(DB, DatabaseFacade);
//!
//! // Use the facade
//! // DB!().query("SELECT * FROM users");
//! ```

pub mod facades;
pub mod macros;

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
