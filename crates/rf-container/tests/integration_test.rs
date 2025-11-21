//! Integration tests for all service lifecycle scopes
//!
//! Tests singleton, scoped, and transient services working together.

use rf_container::{ScopeManager, ScopedContainer, ServiceRegistry, Scope};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct SingletonConfig {
    app_name: String,
}

#[derive(Clone)]
struct ScopedRequestContext {
    request_id: u32,
}

#[derive(Clone)]
struct TransientLogger {
    instance_id: u32,
}

#[tokio::test]
async fn test_all_scopes_together() {
    let mut registry = ServiceRegistry::new();

    // Singleton - created once for entire app
    registry.register(Scope::Singleton, || {
        Arc::new(SingletonConfig {
            app_name: "TestApp".to_string(),
        })
    });

    // Scoped - created once per request
    let scoped_counter = Arc::new(Mutex::new(0u32));
    let scoped_counter_clone = scoped_counter.clone();
    registry.register(Scope::Scoped, move || {
        let mut count = scoped_counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(ScopedRequestContext { request_id: *count })
    });

    // Transient - created on every resolve
    let transient_counter = Arc::new(Mutex::new(0u32));
    let transient_counter_clone = transient_counter.clone();
    registry.register(Scope::Transient, move || {
        let mut count = transient_counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(TransientLogger { instance_id: *count })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(Arc::clone(&registry));

    // First scope/request
    manager
        .with_scope("request-1".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            // Resolve services
            let config1: Arc<SingletonConfig> = scope.resolve().unwrap();
            let context1: Arc<ScopedRequestContext> = scope.resolve().unwrap();
            let logger1: Arc<TransientLogger> = scope.resolve().unwrap();

            // Resolve again
            let config2: Arc<SingletonConfig> = scope.resolve().unwrap();
            let context2: Arc<ScopedRequestContext> = scope.resolve().unwrap();
            let logger2: Arc<TransientLogger> = scope.resolve().unwrap();

            // Singleton: same instance
            assert_eq!(Arc::as_ptr(&config1), Arc::as_ptr(&config2));

            // Scoped: same instance within scope
            assert_eq!(context1.request_id, context2.request_id);
            assert_eq!(context1.request_id, 1);

            // Transient: different instances
            assert_ne!(logger1.instance_id, logger2.instance_id);
            assert_eq!(logger1.instance_id, 1);
            assert_eq!(logger2.instance_id, 2);
        })
        .await;

    // Second scope/request
    manager
        .with_scope("request-2".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            let config: Arc<SingletonConfig> = scope.resolve().unwrap();
            let context: Arc<ScopedRequestContext> = scope.resolve().unwrap();
            let logger: Arc<TransientLogger> = scope.resolve().unwrap();

            // Singleton: still same instance from first request
            assert_eq!(config.app_name, "TestApp");

            // Scoped: new instance for new scope
            assert_eq!(context.request_id, 2);

            // Transient: continues incrementing
            assert_eq!(logger.instance_id, 3);
        })
        .await;

    // Verify final counts
    assert_eq!(*scoped_counter.lock().unwrap(), 2); // Two scopes
    assert_eq!(*transient_counter.lock().unwrap(), 3); // Three resolves
}

#[tokio::test]
async fn test_scope_isolation() {
    let mut registry = ServiceRegistry::new();

    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = counter.clone();

    registry.register(Scope::Scoped, move || {
        let mut count = counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(ScopedRequestContext { request_id: *count })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    // Create two scopes concurrently
    let (result1, result2) = tokio::join!(
        manager.with_scope("scope-1".to_string(), async {
            let scope = ScopedContainer::current().unwrap();
            let ctx: Arc<ScopedRequestContext> = scope.resolve().unwrap();
            ctx.request_id
        }),
        manager.with_scope("scope-2".to_string(), async {
            let scope = ScopedContainer::current().unwrap();
            let ctx: Arc<ScopedRequestContext> = scope.resolve().unwrap();
            ctx.request_id
        })
    );

    // Both scopes should get unique instances
    assert_ne!(result1, result2);
}

#[tokio::test]
async fn test_singleton_shared_across_scopes() {
    let mut registry = ServiceRegistry::new();

    let creation_count = Arc::new(Mutex::new(0u32));
    let creation_count_clone = creation_count.clone();

    registry.register(Scope::Singleton, move || {
        let mut count = creation_count_clone.lock().unwrap();
        *count += 1;
        Arc::new(SingletonConfig {
            app_name: format!("App-{}", *count),
        })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    let mut app_names = Vec::new();

    for i in 1..=3 {
        let scope_id = format!("scope-{}", i);
        let name = manager
            .with_scope(scope_id, async {
                let scope = ScopedContainer::current().unwrap();
                let config: Arc<SingletonConfig> = scope.resolve().unwrap();
                config.app_name.clone()
            })
            .await;
        app_names.push(name);
    }

    // All scopes should get the same singleton instance
    assert_eq!(app_names[0], "App-1");
    assert_eq!(app_names[1], "App-1");
    assert_eq!(app_names[2], "App-1");

    // Factory should only be called once
    assert_eq!(*creation_count.lock().unwrap(), 1);
}

#[tokio::test]
async fn test_transient_always_new() {
    let mut registry = ServiceRegistry::new();

    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = counter.clone();

    registry.register(Scope::Transient, move || {
        let mut count = counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(TransientLogger { instance_id: *count })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    manager
        .with_scope("test-scope".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            let logger1: Arc<TransientLogger> = scope.resolve().unwrap();
            let logger2: Arc<TransientLogger> = scope.resolve().unwrap();
            let logger3: Arc<TransientLogger> = scope.resolve().unwrap();

            // All different instances
            assert_eq!(logger1.instance_id, 1);
            assert_eq!(logger2.instance_id, 2);
            assert_eq!(logger3.instance_id, 3);
        })
        .await;

    // Factory called three times
    assert_eq!(*counter.lock().unwrap(), 3);
}

#[tokio::test]
async fn test_scoped_cache_cleanup() {
    let mut registry = ServiceRegistry::new();

    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = counter.clone();

    registry.register(Scope::Scoped, move || {
        let mut count = counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(ScopedRequestContext { request_id: *count })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    // Scope 1
    manager
        .with_scope("scope-1".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            let ctx1: Arc<ScopedRequestContext> = scope.resolve().unwrap();
            let ctx2: Arc<ScopedRequestContext> = scope.resolve().unwrap();

            // Same instance within scope
            assert_eq!(ctx1.request_id, ctx2.request_id);
            assert_eq!(ctx1.request_id, 1);

            // Cache has one entry
            assert_eq!(scope.cached_count(), 1);
        })
        .await;
    // Scope 1 ends, cache is dropped

    // Scope 2 - should create new instance
    manager
        .with_scope("scope-2".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            let ctx: Arc<ScopedRequestContext> = scope.resolve().unwrap();

            // New instance for new scope
            assert_eq!(ctx.request_id, 2);
        })
        .await;

    // Total factory calls: 2 (once per scope)
    assert_eq!(*counter.lock().unwrap(), 2);
}

#[tokio::test]
async fn test_mixed_dependencies() {
    #[derive(Clone)]
    struct AppConfig {
        env: String,
    }

    #[derive(Clone)]
    struct RequestId {
        id: u32,
    }

    #[derive(Clone)]
    struct TaskId {
        id: u32,
    }

    let mut registry = ServiceRegistry::new();

    // Singleton config
    registry.register(Scope::Singleton, || {
        Arc::new(AppConfig {
            env: "production".to_string(),
        })
    });

    // Scoped request ID
    let request_counter = Arc::new(Mutex::new(0u32));
    let request_counter_clone = request_counter.clone();
    registry.register(Scope::Scoped, move || {
        let mut count = request_counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(RequestId { id: *count })
    });

    // Transient task ID
    let task_counter = Arc::new(Mutex::new(0u32));
    let task_counter_clone = task_counter.clone();
    registry.register(Scope::Transient, move || {
        let mut count = task_counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(TaskId { id: *count })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    manager
        .with_scope("request".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            // Resolve all types
            let config: Arc<AppConfig> = scope.resolve().unwrap();
            let request_id: Arc<RequestId> = scope.resolve().unwrap();
            let task1: Arc<TaskId> = scope.resolve().unwrap();
            let task2: Arc<TaskId> = scope.resolve().unwrap();

            assert_eq!(config.env, "production");
            assert_eq!(request_id.id, 1);
            assert_eq!(task1.id, 1);
            assert_eq!(task2.id, 2); // Different transient instances
        })
        .await;
}

#[test]
fn test_scope_types() {
    assert_eq!(Scope::default(), Scope::Singleton);
    assert_ne!(Scope::Singleton, Scope::Scoped);
    assert_ne!(Scope::Singleton, Scope::Transient);
    assert_ne!(Scope::Scoped, Scope::Transient);
}

#[tokio::test]
async fn test_resolve_from_nested_scopes() {
    let mut registry = ServiceRegistry::new();

    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = counter.clone();

    registry.register(Scope::Scoped, move || {
        let mut count = counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(ScopedRequestContext { request_id: *count })
    });

    let registry = Arc::new(registry);
    let outer_manager = ScopeManager::new(Arc::clone(&registry));

    outer_manager
        .with_scope("outer".to_string(), async move {
            let outer_scope = ScopedContainer::current().unwrap();
            let outer_ctx: Arc<ScopedRequestContext> = outer_scope.resolve().unwrap();
            assert_eq!(outer_ctx.request_id, 1);

            let inner_manager = ScopeManager::new(registry);
            inner_manager
                .with_scope("inner".to_string(), async {
                    let inner_scope = ScopedContainer::current().unwrap();
                    let inner_ctx: Arc<ScopedRequestContext> = inner_scope.resolve().unwrap();

                    // Inner scope gets its own instance
                    assert_eq!(inner_ctx.request_id, 2);
                    assert_ne!(outer_ctx.request_id, inner_ctx.request_id);
                })
                .await;
        })
        .await;
}
