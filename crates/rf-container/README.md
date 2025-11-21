# rf-container: Dependency Injection Container

A powerful, Laravel-inspired dependency injection container for Rust with full support for singleton, scoped, and transient service lifetimes.

## Features

- **Type-Safe**: Fully type-safe service registration and resolution
- **Three Lifecycle Scopes**: Singleton, Scoped, and Transient
- **Thread-Safe**: All operations are safe for concurrent use
- **Async-First**: Built on tokio for async/await support
- **Request-Scoped Services**: Perfect for web applications
- **Tenant-Scoped Services**: Ideal for multi-tenant applications
- **Zero Configuration**: Simple, intuitive API

## Service Lifetimes

### Singleton

Created once for the entire application lifetime. All resolutions return the same instance.

```rust
use rf_container::{ServiceRegistry, Scope};
use std::sync::Arc;

#[derive(Clone)]
struct DatabasePool {
    url: String,
}

let mut registry = ServiceRegistry::new();

registry.register(Scope::Singleton, || {
    Arc::new(DatabasePool {
        url: "postgres://localhost".to_string(),
    })
});

// All resolves get the same instance
let pool1: Arc<DatabasePool> = registry.resolve().unwrap();
let pool2: Arc<DatabasePool> = registry.resolve().unwrap();
assert_eq!(Arc::as_ptr(&pool1), Arc::as_ptr(&pool2));
```

### Scoped

Created once per scope (e.g., HTTP request, tenant session). Shared within that scope.

```rust
use rf_container::{ServiceRegistry, ScopeManager, ScopedContainer, Scope};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct RequestLogger {
    request_id: u32,
}

#[tokio::main]
async fn main() {
    let mut registry = ServiceRegistry::new();
    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = counter.clone();

    registry.register(Scope::Scoped, move || {
        let mut count = counter_clone.lock().unwrap();
        *count += 1;
        Arc::new(RequestLogger { request_id: *count })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    // First request/scope
    manager.with_scope("request-1".to_string(), async {
        let scope = ScopedContainer::current().unwrap();

        // Both resolves get the same instance within this scope
        let logger1: Arc<RequestLogger> = scope.resolve().unwrap();
        let logger2: Arc<RequestLogger> = scope.resolve().unwrap();
        assert_eq!(logger1.request_id, logger2.request_id);
    }).await;

    // Second request/scope gets a new instance
    manager.with_scope("request-2".to_string(), async {
        let scope = ScopedContainer::current().unwrap();
        let logger: Arc<RequestLogger> = scope.resolve().unwrap();
        // Different request_id than first scope
    }).await;
}
```

### Transient

Created every time it's resolved. Each resolution gets a new instance.

```rust
use rf_container::{ServiceRegistry, Scope};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct TaskRunner {
    task_id: u32,
}

let mut registry = ServiceRegistry::new();
let counter = Arc::new(Mutex::new(0u32));
let counter_clone = counter.clone();

registry.register(Scope::Transient, move || {
    let mut count = counter_clone.lock().unwrap();
    *count += 1;
    Arc::new(TaskRunner { task_id: *count })
});

// Each resolve gets a new instance
let task1: Arc<TaskRunner> = registry.resolve().unwrap();
let task2: Arc<TaskRunner> = registry.resolve().unwrap();
assert_ne!(task1.task_id, task2.task_id);
```

## Common Use Cases

### Web Application with Request-Scoped Services

```rust
use rf_container::{ServiceRegistry, ScopeManager, ScopedContainer, Scope};
use std::sync::Arc;

#[derive(Clone)]
struct DatabasePool { /* ... */ }

#[derive(Clone)]
struct RequestLogger {
    request_id: String,
}

#[derive(Clone)]
struct Cache { /* ... */ }

#[tokio::main]
async fn main() {
    let mut registry = ServiceRegistry::new();

    // Singleton: Database pool shared across all requests
    registry.register(Scope::Singleton, || {
        Arc::new(DatabasePool { /* ... */ })
    });

    // Scoped: Logger per request
    registry.register(Scope::Scoped, || {
        Arc::new(RequestLogger {
            request_id: format!("REQ-{}", /* ... */),
        })
    });

    // Scoped: Cache per request
    registry.register(Scope::Scoped, || {
        Arc::new(Cache { /* ... */ })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    // Handle HTTP request
    manager.with_scope("request-123".to_string(), async {
        let scope = ScopedContainer::current().unwrap();

        // All services are available
        let db: Arc<DatabasePool> = scope.resolve().unwrap();
        let logger: Arc<RequestLogger> = scope.resolve().unwrap();
        let cache: Arc<Cache> = scope.resolve().unwrap();

        // Use services to handle request
        // logger, cache are unique to this request
        // db is shared across all requests
    }).await;
}
```

### Multi-Tenant Application

```rust
use rf_container::{ServiceRegistry, ScopeManager, ScopedContainer, Scope};
use std::sync::Arc;

#[derive(Clone)]
struct TenantDatabase {
    tenant_id: String,
    schema: String,
}

#[derive(Clone)]
struct TenantConfig {
    tenant_id: String,
    features: Vec<String>,
}

#[tokio::main]
async fn main() {
    let mut registry = ServiceRegistry::new();

    // Scoped: Database connection per tenant
    registry.register(Scope::Scoped, || {
        // Get tenant from context
        let tenant_id = get_current_tenant();
        Arc::new(TenantDatabase {
            tenant_id: tenant_id.clone(),
            schema: format!("tenant_{}", tenant_id),
        })
    });

    // Scoped: Configuration per tenant
    registry.register(Scope::Scoped, || {
        let tenant_id = get_current_tenant();
        Arc::new(TenantConfig {
            tenant_id,
            features: load_features(),
        })
    });

    let registry = Arc::new(registry);
    let manager = ScopeManager::new(registry);

    // Handle request for tenant
    manager.with_scope("tenant-acme".to_string(), async {
        let scope = ScopedContainer::current().unwrap();

        let db: Arc<TenantDatabase> = scope.resolve().unwrap();
        let config: Arc<TenantConfig> = scope.resolve().unwrap();

        // All services are isolated to this tenant
    }).await;
}

fn get_current_tenant() -> String {
    // Get from thread-local or context
    "acme-corp".to_string()
}

fn load_features() -> Vec<String> {
    vec!["feature1".to_string(), "feature2".to_string()]
}
```

## API Reference

### ServiceRegistry

The main container for service registration and resolution.

```rust
// Create a new registry
let mut registry = ServiceRegistry::new();

// Register a service
registry.register(Scope::Singleton, || Arc::new(MyService::new()));

// Check if service is registered
if registry.has::<MyService>() {
    // Resolve the service
    let service: Arc<MyService> = registry.resolve().unwrap();
}

// Check service scope
assert_eq!(registry.get_scope::<MyService>(), Some(Scope::Singleton));

// Remove a service
registry.remove::<MyService>();

// Clear all services
registry.clear();
```

### ScopeManager

Manages service scopes for request/tenant isolation.

```rust
let manager = ScopeManager::new(registry);

// Execute code within a scope
let result = manager.with_scope("scope-id".to_string(), async {
    // Your async code here
    42
}).await;
```

### ScopedContainer

Access to scoped services within a scope.

```rust
// Get current scope (only available within a scope)
if let Some(scope) = ScopedContainer::current() {
    // Resolve services
    let service: Arc<MyService> = scope.resolve().unwrap();

    // Get scope ID
    println!("Scope: {}", scope.scope_id());

    // Check cache
    if scope.has_cached::<MyService>() {
        println!("Service is cached");
    }

    // Get cache count
    println!("Cached services: {}", scope.cached_count());

    // Clear cache (useful for testing)
    scope.clear();
}
```

## Comparison with Laravel

This container is inspired by Laravel's service container:

| Feature | Laravel | rf-container |
|---------|---------|--------------|
| Singleton | ✅ `singleton()` | ✅ `Scope::Singleton` |
| Scoped | ✅ `scoped()` | ✅ `Scope::Scoped` |
| Transient | ✅ `bind()` | ✅ `Scope::Transient` |
| Type Safety | ❌ (PHP) | ✅ (Rust) |
| Async Support | ❌ | ✅ |
| Thread Safety | ✅ | ✅ |
| Auto-wiring | ✅ | 🚧 (Future) |
| Service Providers | ✅ | 🚧 (Future) |

## Examples

See the `examples/` directory for complete examples:

- `basic_scoped.rs` - Simple scoped service demonstration
- `scoped_services.rs` - Request-scoped services with logger, database, and cache
- `multi_tenant.rs` - Multi-tenant application with isolated services

Run examples:

```bash
cargo run --example basic_scoped
cargo run --example scoped_services
cargo run --example multi_tenant
```

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_scoped_instance_created_once_per_scope
```

## Performance

- **Singleton**: No overhead after first resolution (cached)
- **Scoped**: Minimal overhead (HashMap lookup within scope)
- **Transient**: Factory call on every resolution

All operations use `Arc<T>` for efficient cloning and sharing.

## Thread Safety

All components are thread-safe:

- `ServiceRegistry` uses `Arc<Mutex<...>>` internally
- `ScopedContainer` uses `Arc<Mutex<...>>` for cache
- `ScopeManager` can be safely cloned and shared

## Future Features

- [ ] Auto-wiring / Constructor injection
- [ ] Service providers
- [ ] Conditional registration
- [ ] Named services
- [ ] Service decorators
- [ ] Lazy resolution
- [ ] Service tags

## License

MIT OR Apache-2.0
