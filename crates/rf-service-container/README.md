# Foundry Service Container

A powerful Dependency Injection (DI) container for the Foundry framework, inspired by Laravel's Service Container.

## Features

- **Type-safe dependency injection** with Rust generics
- **Singleton and transient bindings**
- **Service providers** for organizing service registration
- **Lazy loading** with deferred services
- **Tagging** for grouping related services
- **Alias support** for service naming
- **Thread-safe** with async/await support

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foundry-service-container = { path = "../foundry-service-container" }
```

## Quick Start

### Basic Container Usage

```rust
use foundry_service_container::Container;
use std::sync::Arc;

#[derive(Clone)]
struct Database {
    url: String,
}

#[tokio::main]
async fn main() {
    let container = Container::new();

    // Bind a singleton
    container
        .singleton("database", || {
            Ok(Database {
                url: "postgres://localhost/mydb".to_string(),
            })
        })
        .await
        .unwrap();

    // Resolve the service
    let db: Arc<Database> = container.resolve("database").await.unwrap();
    println!("Database URL: {}", db.url);
}
```

### Service Providers

```rust
use foundry_service_container::{
    async_trait, Container, ServiceProvider, Result,
};

struct DatabaseServiceProvider;

#[async_trait]
impl ServiceProvider for DatabaseServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        container
            .singleton("database.host", || {
                Ok(std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()))
            })
            .await?;

        Ok(())
    }

    async fn boot(&self, _container: &Container) -> Result<()> {
        println!("Database provider booted!");
        Ok(())
    }

    fn name(&self) -> &str {
        "DatabaseServiceProvider"
    }
}
```

## API Reference

### Container

#### Core Methods

- `Container::new()` - Create a new container instance
- `container.bind<T>(key, factory)` - Bind a transient service
- `container.singleton<T>(key, factory)` - Bind a singleton service
- `container.factory<T>(key, factory)` - Bind a factory
- `container.instance<T>(key, instance)` - Bind an existing instance
- `container.resolve<T>(key)` - Resolve a service by key
- `container.get<T>()` - Resolve by type name
- `container.has(key)` - Check if service exists

#### Advanced Features

- `container.alias(alias, original)` - Create service alias
- `container.tag(tags, services)` - Tag services
- `container.tagged(tag)` - Get services by tag
- `container.defer(key)` - Mark service as deferred
- `container.is_deferred(key)` - Check if service is deferred
- `container.flush()` - Clear all bindings
- `container.extend(other)` - Merge containers

### Service Provider

```rust
#[async_trait]
pub trait ServiceProvider: Send + Sync {
    async fn register(&self, container: &Container) -> Result<()>;
    async fn boot(&self, container: &Container) -> Result<()>;
    fn defer(&self) -> Vec<String>;
    fn name(&self) -> &str;
}
```

### Built-in Providers

- `ApplicationServiceProvider` - App configuration
- `AuthServiceProvider` - Authentication services
- `CacheServiceProvider` - Cache configuration
- `DatabaseServiceProvider` - Database connections
- `MailServiceProvider` - Email services

## Examples

See the `examples/` directory for complete examples:

- `basic_usage.rs` - Basic container usage
- `service_providers.rs` - Using service providers

Run examples with:

```bash
cargo run --example basic_usage
cargo run --example service_providers
```

## Testing

Run tests with:

```bash
cargo test
```

## License

MIT OR Apache-2.0
