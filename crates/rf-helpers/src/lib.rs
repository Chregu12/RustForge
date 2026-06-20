//! # RustForge Helpers
//!
//! Laravel-style global helper functions for maximum developer productivity.
//!
//! ## Modules
//! - `arr` - Array/Vec helper functions
//! - `str` - String manipulation helpers
//! - `url` - URL generation helpers
//! - `path` - Path helpers (storage_path, public_path, etc.)
//! - `misc` - Miscellaneous helpers (abort, dd, dump, env)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rf_helpers::*;
//!
//! // String helpers
//! let slug = str::slug("Hello World"); // "hello-world"
//! let plural = str::plural("user"); // "users"
//!
//! // Array helpers
//! let items = vec![1, 2, 3, 4, 5];
//! let first = arr::first(&items, |&x| x > 2); // Some(&3)
//!
//! // Debug helpers
//! let some_value = "debug me";
//! let another_value = 42;
//! dd!(some_value); // Dump and die
//! dump!(another_value); // Dump without dying
//! ```

pub mod arr;
pub mod path;
pub mod str;
pub mod url;

// Re-export commonly used functions
pub use arr::*;
pub use path::*;
pub use str::*;
pub use url::*;

// `arr` and `str` both define `random` and `contains`. Surface the string
// variants at the crate root (the common Laravel `Str::random` / `Str::contains`
// helpers); the array variants remain available as `arr::random` / `arr::contains`.
// The explicit re-exports shadow the globs above and resolve the otherwise
// ambiguous-glob-reexport warnings.
pub use str::{contains, random};

// Macros
/// Dump and die - prints debug output and exits the process
#[macro_export]
macro_rules! dd {
    ($($arg:tt)*) => {{
        eprintln!("🔍 Debug Dump ({}:{})", file!(), line!());
        eprintln!("{:#?}", $($arg)*);
        eprintln!("💀 Process terminated by dd!");
        std::process::exit(1);
    }};
}

/// Dump - prints debug output without exiting
#[macro_export]
macro_rules! dump {
    ($($arg:tt)*) => {{
        eprintln!("🔍 Debug Dump ({}:{})", file!(), line!());
        eprintln!("{:#?}", $($arg)*);
    }};
}

// Misc helpers
use std::env;
use std::process;

/// Get environment variable value
pub fn env_var(key: &str) -> Option<String> {
    env::var(key).ok()
}

/// Get environment variable with default value
pub fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Abort with HTTP status code
pub fn abort(code: u16) -> axum::http::Response<axum::body::Body> {
    axum::http::Response::builder()
        .status(code)
        .body(axum::body::Body::empty())
        .unwrap()
}

/// Abort if condition is true
pub fn abort_if(condition: bool, code: u16) -> Option<axum::http::Response<axum::body::Body>> {
    if condition {
        Some(abort(code))
    } else {
        None
    }
}

/// Abort unless condition is true
pub fn abort_unless(condition: bool, code: u16) -> Option<axum::http::Response<axum::body::Body>> {
    abort_if(!condition, code)
}

/// Exit the process with a status code
pub fn exit(code: i32) -> ! {
    process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_or() {
        env::set_var("TEST_VAR", "test_value");
        assert_eq!(env_or("TEST_VAR", "default"), "test_value");
        assert_eq!(env_or("NON_EXISTENT", "default"), "default");
    }

    #[test]
    fn test_abort_if() {
        assert!(abort_if(true, 404).is_some());
        assert!(abort_if(false, 404).is_none());
    }

    #[test]
    fn test_abort_unless() {
        assert!(abort_unless(false, 404).is_some());
        assert!(abort_unless(true, 404).is_none());
    }
}
