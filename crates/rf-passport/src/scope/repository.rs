//! Scope repository for managing available scopes

use super::{Scope, ScopeChecker};
use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Global scope repository
static SCOPE_REPOSITORY: Lazy<RwLock<ScopeChecker>> =
    Lazy::new(|| RwLock::new(ScopeChecker::new()));

/// Repository for managing OAuth scopes
pub struct ScopeRepository;

impl ScopeRepository {
    /// Register a scope globally
    pub fn register(scope: Scope) {
        let mut checker = SCOPE_REPOSITORY
            .write()
            .expect("Failed to acquire write lock on scope repository");
        checker.register(scope);
    }

    /// Register multiple scopes globally
    pub fn register_many(scopes: Vec<Scope>) {
        let mut checker = SCOPE_REPOSITORY
            .write()
            .expect("Failed to acquire write lock on scope repository");
        checker.register_many(scopes);
    }

    /// Check if a scope exists
    pub fn exists(scope_id: &str) -> bool {
        let checker = SCOPE_REPOSITORY
            .read()
            .expect("Failed to acquire read lock on scope repository");
        checker.exists(scope_id)
    }

    /// Get a scope by ID
    pub fn get(scope_id: &str) -> Option<Scope> {
        let checker = SCOPE_REPOSITORY
            .read()
            .expect("Failed to acquire read lock on scope repository");
        checker.get(scope_id).cloned()
    }

    /// Validate requested scopes
    pub fn validate(requested: &[String]) -> Result<(), Vec<String>> {
        let checker = SCOPE_REPOSITORY
            .read()
            .expect("Failed to acquire read lock on scope repository");
        checker.validate(requested)
    }

    /// Get all scopes
    pub fn all() -> Vec<Scope> {
        let checker = SCOPE_REPOSITORY
            .read()
            .expect("Failed to acquire read lock on scope repository");
        checker.all().into_iter().cloned().collect()
    }

    /// Get scope count
    pub fn count() -> usize {
        let checker = SCOPE_REPOSITORY
            .read()
            .expect("Failed to acquire read lock on scope repository");
        checker.count()
    }

    /// Clear all scopes (useful for testing)
    pub fn clear() {
        let mut checker = SCOPE_REPOSITORY
            .write()
            .expect("Failed to acquire write lock on scope repository");
        *checker = ScopeChecker::new();
    }
}

/// Macro for easily registering scopes
#[macro_export]
macro_rules! register_scopes {
    ($($id:expr => $desc:expr),* $(,)?) => {
        {
            let scopes = vec![
                $(
                    $crate::scope::Scope::new($id, $desc),
                )*
            ];
            $crate::scope::ScopeRepository::register_many(scopes);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_scope_repository() {
        ScopeRepository::clear();

        ScopeRepository::register(Scope::new("read:posts", "Read posts"));
        ScopeRepository::register(Scope::new("write:posts", "Write posts"));

        assert!(ScopeRepository::exists("read:posts"));
        assert!(ScopeRepository::exists("write:posts"));
        assert!(!ScopeRepository::exists("delete:posts"));

        assert_eq!(ScopeRepository::count(), 2);
    }

    #[test]
    fn test_scope_macro() {
        ScopeRepository::clear();

        register_scopes! {
            "read:posts" => "Read posts",
            "write:posts" => "Write posts",
            "delete:posts" => "Delete posts",
        }

        assert_eq!(ScopeRepository::count(), 3);
        assert!(ScopeRepository::exists("read:posts"));
        assert!(ScopeRepository::exists("write:posts"));
        assert!(ScopeRepository::exists("delete:posts"));
    }
}
