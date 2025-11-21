# Auto-Resolution Feature Documentation

## Overview

The auto-resolution feature provides automatic dependency injection for the `rf-container` service registry. Instead of manually constructing services with their dependencies, you can implement the `Resolvable` trait and let the container automatically resolve and inject dependencies.

## Motivation

**Before (Manual Construction):**
```rust
let db = Arc::new(Database::new());
let cache = Arc::new(Cache::new());
let logger = Arc::new(Logger::new());

registry.register(Scope::Singleton, move || {
    Arc::new(UserRepository::new(
        db.clone(),    // Must manually pass
        cache.clone(), // Must manually pass
        logger.clone() // Must manually pass
    ))
});
```

**After (Auto-Resolution):**
```rust
impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let db = registry.resolve::<Database>()?;
        let cache = registry.resolve::<Cache>()?;
        let logger = registry.resolve::<Logger>()?;
        Ok(UserRepository { db, cache, logger })
    }
}

// Dependencies auto-injected!
let repo = registry.resolve::<UserRepository>()?;
```

## Features

- **Automatic Constructor Injection**: Dependencies are automatically resolved from the container
- **Circular Dependency Detection**: Detects and prevents circular dependencies
- **Type-Safe**: Compile-time type checking ensures all dependencies are correctly typed
- **Lifecycle Support**: Works with Singleton, Scoped, and Transient lifecycles
- **Thread-Safe**: All operations are thread-safe via internal synchronization

## Core Concepts

### The `Resolvable` Trait

The `Resolvable` trait is the foundation of auto-resolution:

```rust
pub trait Resolvable: Send + Sync + 'static {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError>
    where
        Self: Sized;
}
```

Any type implementing this trait can be automatically resolved from the container.

### The `AutoResolver`

The `AutoResolver` handles the actual resolution with circular dependency detection:

```rust
pub struct AutoResolver {
    resolution_stack: Mutex<Vec<TypeId>>,
}

impl AutoResolver {
    pub fn resolve<T: Resolvable>(&self, registry: &ServiceRegistry) -> ContainerResult<Arc<T>>;
    pub fn is_resolving<T: 'static>(&self) -> bool;
    pub fn resolution_depth(&self) -> usize;
}
```

## Usage Guide

### Basic Auto-Resolution

1. **Implement `Resolvable` for your types:**

```rust
struct Database;

impl Resolvable for Database {
    fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        Ok(Database)
    }
}
```

2. **Register in the container:**

```rust
let mut registry = ServiceRegistry::new();
registry.register(Scope::Singleton, || {
    Arc::new(Database)
});
```

3. **Resolve the type:**

```rust
let db = registry.resolve::<Database>()?;
```

### Dependency Injection

For types with dependencies, resolve them from the registry:

```rust
struct UserRepository {
    db: Arc<Database>,
    cache: Arc<Cache>,
}

impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let db = registry.resolve::<Database>()?;
        let cache = registry.resolve::<Cache>()?;
        Ok(UserRepository { db, cache })
    }
}
```

### Nested Dependencies

Dependencies can have their own dependencies:

```rust
struct UserService {
    repository: Arc<UserRepository>,
    logger: Arc<Logger>,
}

impl Resolvable for UserService {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let repository = registry.resolve::<UserRepository>()?;
        let logger = registry.resolve::<Logger>()?;
        Ok(UserService { repository, logger })
    }
}
```

The container will automatically resolve the entire dependency graph:
- `UserService` depends on `UserRepository` and `Logger`
- `UserRepository` depends on `Database` and `Cache`

### Circular Dependency Detection

The `AutoResolver` tracks the resolution stack and detects circular dependencies:

```rust
// A depends on B
impl Resolvable for ServiceA {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let b = registry.resolve::<ServiceB>()?;
        Ok(ServiceA { b })
    }
}

// B depends on A - CIRCULAR!
impl Resolvable for ServiceB {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let a = registry.resolve::<ServiceA>()?; // ERROR: CircularDependency
        Ok(ServiceB { a })
    }
}

// This will fail with ContainerError::CircularDependency
let resolver = AutoResolver::new();
let result = resolver.resolve::<ServiceA>(&registry);
```

## Lifecycle Scopes

Auto-resolution works with all three lifecycle scopes:

### Singleton

One instance for the entire application:

```rust
registry.register(Scope::Singleton, || {
    Arc::new(Database)
});

let db1 = registry.resolve::<Database>()?;
let db2 = registry.resolve::<Database>()?;
// db1 and db2 are the SAME instance
```

### Transient

New instance on every resolution:

```rust
registry.register(Scope::Transient, || {
    Arc::new(Database)
});

let db1 = registry.resolve::<Database>()?;
let db2 = registry.resolve::<Database>()?;
// db1 and db2 are DIFFERENT instances
```

### Scoped

One instance per scope (e.g., per HTTP request):

```rust
registry.register(Scope::Scoped, || {
    Arc::new(RequestLogger)
});

// Within a scope, same instance
manager.with_scope("request-1", async {
    let logger1 = registry.resolve::<RequestLogger>()?;
    let logger2 = registry.resolve::<RequestLogger>()?;
    // logger1 and logger2 are the SAME instance
}).await;

// Different scope, different instance
manager.with_scope("request-2", async {
    let logger = registry.resolve::<RequestLogger>()?;
    // This is a DIFFERENT instance
}).await;
```

## Extension Methods

The `ServiceRegistry` provides convenience methods for binding resolvable types:

### `bind<T>()`

Register with default Singleton scope:

```rust
registry.bind::<Database>();
let db = registry.resolve::<Database>()?;
```

### `bind_with_scope<T>(scope)`

Register with specific scope:

```rust
registry.bind_with_scope::<Logger>(Scope::Singleton);
registry.bind_with_scope::<RequestContext>(Scope::Scoped);
registry.bind_with_scope::<TempFile>(Scope::Transient);
```

### `bind_transient<T>()`

Convenience method for transient scope:

```rust
registry.bind_transient::<TempFile>();
```

### `bind_scoped<T>()`

Convenience method for scoped scope:

```rust
registry.bind_scoped::<RequestLogger>();
```

## Error Handling

Auto-resolution can fail with several error types:

### `ServiceNotFound`

A required dependency is not registered:

```rust
impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let db = registry.resolve::<Database>()?; // ERROR if Database not registered
        Ok(UserRepository { db })
    }
}
```

### `CircularDependency`

A circular dependency is detected:

```rust
// A -> B -> A (circular!)
match resolver.resolve::<ServiceA>(&registry) {
    Err(ContainerError::CircularDependency { type_name }) => {
        eprintln!("Circular dependency detected: {}", type_name);
    }
    _ => {}
}
```

### `DowncastFailed`

Type mismatch during resolution (should not occur with correct usage):

```rust
match registry.resolve::<Database>() {
    Err(ContainerError::DowncastFailed { type_name }) => {
        eprintln!("Type downcast failed: {}", type_name);
    }
    _ => {}
}
```

## Best Practices

### 1. Keep Resolvable Implementations Simple

The `resolve` method should only fetch dependencies and construct the type:

```rust
// GOOD
impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let db = registry.resolve::<Database>()?;
        Ok(UserRepository { db })
    }
}

// BAD - don't do complex logic here
impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let db = registry.resolve::<Database>()?;

        // Don't do this!
        db.migrate().await?;
        db.seed_data().await?;

        Ok(UserRepository { db })
    }
}
```

### 2. Register Dependencies Before Dependents

Always register dependencies before the types that depend on them:

```rust
// Register dependencies first
registry.register(Scope::Singleton, || Arc::new(Database));
registry.register(Scope::Singleton, || Arc::new(Cache));

// Then register dependents
let repo = UserRepository::resolve(&registry)?; // Works!
```

### 3. Use Appropriate Lifecycles

Choose the right lifecycle for each service:

- **Singleton**: Database connections, configuration, shared state
- **Scoped**: Request-specific data, user context, tenant information
- **Transient**: Temporary objects, stateless services, short-lived data

### 4. Avoid Circular Dependencies

Design your dependency graph to be acyclic:

```rust
// GOOD - Linear dependency chain
Database -> Repository -> Service -> Controller

// BAD - Circular dependency
ServiceA -> ServiceB -> ServiceA (circular!)
```

### 5. Use Type Aliases for Complex Types

For complex generic types, use type aliases:

```rust
type UserRepo = Arc<Repository<User, PostgresDatabase>>;

impl Resolvable for UserService {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let repo = registry.resolve::<UserRepo>()?;
        Ok(UserService { repo })
    }
}
```

## Advanced Patterns

### Factory Pattern

Use factories for complex construction:

```rust
struct DatabaseFactory;

impl Resolvable for DatabaseFactory {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        Ok(DatabaseFactory)
    }
}

impl DatabaseFactory {
    fn create_connection(&self) -> Database {
        // Complex construction logic here
        Database::new()
    }
}
```

### Builder Pattern

Combine with the builder pattern:

```rust
impl Resolvable for DatabaseBuilder {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let config = registry.resolve::<DatabaseConfig>()?;
        Ok(DatabaseBuilder::new()
            .with_host(&config.host)
            .with_port(config.port))
    }
}
```

### Conditional Resolution

Resolve different types based on configuration:

```rust
impl Resolvable for EmailService {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let config = registry.resolve::<AppConfig>()?;

        let sender: Arc<dyn EmailSender> = if config.use_smtp {
            registry.resolve::<SmtpSender>()? as Arc<dyn EmailSender>
        } else {
            registry.resolve::<SendGridSender>()? as Arc<dyn EmailSender>
        };

        Ok(EmailService { sender })
    }
}
```

## Performance Considerations

### Resolution Overhead

Auto-resolution adds a small overhead:
- Type lookup in the registry
- Circular dependency checking
- Dynamic dispatch for the `resolve` method

For most applications, this overhead is negligible. If performance is critical:

1. Use Singleton scope to cache instances
2. Manually register frequently-used services
3. Profile to identify bottlenecks

### Memory Usage

The `AutoResolver` tracks the resolution stack:
- Each level of nesting adds one `TypeId` (16 bytes) to the stack
- Stack is cleared after resolution completes
- Memory usage is proportional to dependency depth

Typical dependency depth is 3-5 levels, so memory impact is minimal.

## Testing

### Unit Testing Resolvable Types

Test the `resolve` method directly:

```rust
#[test]
fn test_user_repository_resolve() {
    let mut registry = ServiceRegistry::new();

    registry.register(Scope::Singleton, || Arc::new(Database));
    registry.register(Scope::Singleton, || Arc::new(Cache));

    let repo = UserRepository::resolve(&registry).unwrap();
    assert!(Arc::strong_count(&repo.db) > 0);
}
```

### Integration Testing

Test the complete dependency graph:

```rust
#[test]
fn test_full_dependency_graph() {
    let mut registry = ServiceRegistry::new();

    // Register all dependencies
    registry.register(Scope::Singleton, || Arc::new(Database));
    registry.register(Scope::Singleton, || Arc::new(Cache));
    registry.register(Scope::Singleton, || Arc::new(Logger));

    // Resolve top-level service
    let service = UserService::resolve(&registry).unwrap();

    // Verify dependencies are correctly injected
    assert!(Arc::strong_count(&service.repository) > 0);
    assert!(Arc::strong_count(&service.logger) > 0);
}
```

### Mocking Dependencies

Use test doubles for dependencies:

```rust
struct MockDatabase;

impl Resolvable for MockDatabase {
    fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        Ok(MockDatabase)
    }
}

#[test]
fn test_with_mock() {
    let mut registry = ServiceRegistry::new();
    registry.register(Scope::Singleton, || Arc::new(MockDatabase));

    let repo = UserRepository::resolve(&registry).unwrap();
    // Test with mock database
}
```

## Comparison with Other Frameworks

### Laravel (PHP)

Laravel's container auto-resolution:

```php
// Laravel
class UserRepository {
    public function __construct(Database $db, Cache $cache) {
        $this->db = $db;
        $this->cache = $cache;
    }
}

$repo = app(UserRepository::class); // Auto-resolves dependencies
```

`rf-container` equivalent:

```rust
// Rust
impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        let db = registry.resolve::<Database>()?;
        let cache = registry.resolve::<Cache>()?;
        Ok(UserRepository { db, cache })
    }
}

let repo = registry.resolve::<UserRepository>()?;
```

### Spring (Java)

Spring's dependency injection:

```java
// Spring
@Service
public class UserRepository {
    @Autowired
    private Database db;

    @Autowired
    private Cache cache;
}
```

`rf-container` uses explicit trait implementation instead of annotations.

## Roadmap

Future enhancements planned:

1. **Derive Macro**: Auto-implement `Resolvable` via `#[derive(Resolvable)]`
2. **Named Bindings**: Multiple implementations of the same type
3. **Contextual Binding**: Different implementations based on context
4. **Property Injection**: Inject dependencies after construction
5. **Method Injection**: Inject dependencies via setter methods

## Conclusion

The auto-resolution feature brings Laravel-style dependency injection to Rust, making it easier to build applications with proper separation of concerns and testability. By implementing the `Resolvable` trait, your types can automatically resolve their dependencies from the container, reducing boilerplate and improving maintainability.

For more examples, see:
- `examples/auto_resolution.rs` - Complete working example
- `tests/auto_resolve_test.rs` - Comprehensive test suite
