//! Service lifecycle scopes

/// Service lifecycle scope
///
/// Determines how services are instantiated and cached:
/// - **Singleton**: One instance for entire application lifetime
/// - **Scoped**: One instance per request/scope (not yet implemented)
/// - **Transient**: New instance on every resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Single instance shared across entire application
    ///
    /// Created once on first resolution and reused for all subsequent requests.
    /// Thread-safe via `Arc<T>`.
    ///
    /// # Example
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// #[derive(Clone)]
    /// struct Config { value: String }
    ///
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(
    ///     Scope::Singleton,
    ///     || Arc::new(Config { value: "shared".to_string() })
    /// );
    /// ```
    Singleton,

    /// One instance per request scope
    ///
    /// Future: Will create one instance per request and share it
    /// within that request's lifetime.
    Scoped,

    /// New instance on every resolution
    ///
    /// Factory is called every time `resolve()` is invoked.
    ///
    /// # Example
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::{Arc, Mutex};
    ///
    /// #[derive(Clone)]
    /// struct Logger { id: u32 }
    ///
    /// let mut registry = ServiceRegistry::new();
    /// let counter = Arc::new(Mutex::new(0u32));
    /// let counter_clone = counter.clone();
    /// registry.register(
    ///     Scope::Transient,
    ///     move || {
    ///         let mut count = counter_clone.lock().unwrap();
    ///         *count += 1;
    ///         Arc::new(Logger { id: *count })
    ///     }
    /// );
    /// ```
    Transient,
}

impl Default for Scope {
    fn default() -> Self {
        Self::Singleton
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_equality() {
        assert_eq!(Scope::Singleton, Scope::Singleton);
        assert_ne!(Scope::Singleton, Scope::Transient);
    }

    #[test]
    fn test_scope_default() {
        assert_eq!(Scope::default(), Scope::Singleton);
    }

    #[test]
    fn test_scope_debug() {
        let scope = Scope::Singleton;
        assert_eq!(format!("{:?}", scope), "Singleton");
    }
}
