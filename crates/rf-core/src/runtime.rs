//! Runtime utilities for synchronous/asynchronous bridge
//!
//! Provides a shared Tokio runtime for blocking sync APIs that need to call async code internally.

use std::sync::OnceLock;
use tokio::runtime::Runtime;

/// Global runtime for sync API bridge
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Get or initialize the global runtime
fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}

/// Execute an async block synchronously using the global runtime
///
/// # Example
///
/// ```rust
/// use rf_core::runtime::block_on;
///
/// async fn async_operation() -> String {
///     "result".to_string()
/// }
///
/// fn sync_wrapper() -> String {
///     block_on(async_operation())
/// }
/// ```
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_on() {
        async fn test_async() -> i32 {
            42
        }

        let result = block_on(test_async());
        assert_eq!(result, 42);
    }

    #[test]
    fn test_multiple_calls() {
        async fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let result1 = block_on(add(1, 2));
        let result2 = block_on(add(3, 4));

        assert_eq!(result1, 3);
        assert_eq!(result2, 7);
    }
}
