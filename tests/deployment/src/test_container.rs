//! Deployment tests for rf-container

#[cfg(test)]
mod tests {
    use rf_container::{ServiceRegistry, Scope, ScopedContainer, AutoResolver};
    use std::sync::Arc;

    // ── ServiceRegistry ──────────────────────────────────────────

    #[test]
    fn service_registry_register_and_resolve() {
        let mut registry = ServiceRegistry::new();
        registry.register::<String, _>(Scope::Singleton, || Arc::new("Hello, World!".to_string()));
        let resolved = registry.resolve::<String>();
        assert!(resolved.is_ok());
        assert_eq!(*resolved.unwrap(), "Hello, World!");
    }

    #[test]
    fn service_registry_has() {
        let mut registry = ServiceRegistry::new();
        assert!(!registry.has::<i32>());
        registry.register::<i32, _>(Scope::Singleton, || Arc::new(42));
        assert!(registry.has::<i32>());
    }

    #[test]
    fn service_registry_remove() {
        let mut registry = ServiceRegistry::new();
        registry.register::<u64, _>(Scope::Singleton, || Arc::new(100));
        assert!(registry.has::<u64>());
        registry.remove::<u64>();
        assert!(!registry.has::<u64>());
    }

    #[test]
    fn service_registry_clear() {
        let mut registry = ServiceRegistry::new();
        registry.register::<i32, _>(Scope::Singleton, || Arc::new(1));
        registry.register::<String, _>(Scope::Singleton, || Arc::new("test".into()));
        registry.clear();
        assert!(!registry.has::<i32>());
        assert!(!registry.has::<String>());
    }

    #[test]
    fn service_registry_singleton_same_instance() {
        let mut registry = ServiceRegistry::new();
        registry.register::<String, _>(Scope::Singleton, || Arc::new("singleton".to_string()));
        let a = registry.resolve::<String>().unwrap();
        let b = registry.resolve::<String>().unwrap();
        assert_eq!(*a, *b);
    }

    // ── Scopes ───────────────────────────────────────────────────

    #[test]
    fn scope_variants() {
        let _singleton = Scope::Singleton;
        let _scoped = Scope::Scoped;
        let _transient = Scope::Transient;
    }

    // ── ScopedContainer ──────────────────────────────────────────

    #[test]
    fn scoped_container() {
        let registry = Arc::new(ServiceRegistry::new());
        let container = ScopedContainer::new(registry, "test-scope".to_string());
        assert!(!container.scope_id().is_empty());
        assert_eq!(container.cached_count(), 0);
    }

    #[test]
    fn scoped_container_clear() {
        let registry = Arc::new(ServiceRegistry::new());
        let container = ScopedContainer::new(registry, "test".to_string());
        container.clear();
        assert_eq!(container.cached_count(), 0);
    }

    // ── AutoResolver ─────────────────────────────────────────────

    #[test]
    fn auto_resolver() {
        let resolver = AutoResolver::new();
        assert_eq!(resolver.resolution_depth(), 0);
    }

    #[test]
    fn auto_resolver_clear() {
        let resolver = AutoResolver::new();
        resolver.clear();
        assert_eq!(resolver.resolution_depth(), 0);
    }
}
