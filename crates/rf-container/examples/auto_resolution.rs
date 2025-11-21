//! Auto-resolution example demonstrating dependency injection
//!
//! This example shows how to use the `Resolvable` trait and auto-resolution
//! to automatically inject dependencies into services.
//!
//! Run with:
//! ```bash
//! cargo run --example auto_resolution
//! ```

use rf_container::{ContainerError, Resolvable, Scope, ServiceRegistry};
use std::sync::Arc;

// ============================================================================
// Domain Types
// ============================================================================

#[derive(Debug, Clone)]
struct DatabaseConfig {
    host: String,
    port: u16,
    database: String,
}

impl Resolvable for DatabaseConfig {
    fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        println!("  [Creating] DatabaseConfig");
        Ok(DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "myapp".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
struct CacheConfig {
    host: String,
    port: u16,
}

impl Resolvable for CacheConfig {
    fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        println!("  [Creating] CacheConfig");
        Ok(CacheConfig {
            host: "localhost".to_string(),
            port: 6379,
        })
    }
}

// ============================================================================
// Infrastructure Services
// ============================================================================

#[derive(Clone)]
struct DatabaseConnection {
    config: Arc<DatabaseConfig>,
    connection_id: u32,
}

impl Resolvable for DatabaseConnection {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        println!("  [Creating] DatabaseConnection");
        let config = registry.resolve::<DatabaseConfig>()?;

        // Simulate creating a connection
        static mut COUNTER: u32 = 0;
        let connection_id = unsafe {
            COUNTER += 1;
            COUNTER
        };

        Ok(DatabaseConnection {
            config,
            connection_id,
        })
    }
}

impl DatabaseConnection {
    fn query(&self, sql: &str) -> String {
        format!(
            "[DB #{}] Executing: {} on {}:{}/{}",
            self.connection_id, sql, self.config.host, self.config.port, self.config.database
        )
    }
}

#[derive(Clone)]
struct CacheConnection {
    config: Arc<CacheConfig>,
    connection_id: u32,
}

impl Resolvable for CacheConnection {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        println!("  [Creating] CacheConnection");
        let config = registry.resolve::<CacheConfig>()?;

        // Simulate creating a connection
        static mut COUNTER: u32 = 0;
        let connection_id = unsafe {
            COUNTER += 1;
            COUNTER
        };

        Ok(CacheConnection {
            config,
            connection_id,
        })
    }
}

impl CacheConnection {
    fn get(&self, key: &str) -> String {
        format!(
            "[Cache #{}] GET {} from {}:{}",
            self.connection_id, key, self.config.host, self.config.port
        )
    }

    fn set(&self, key: &str, value: &str) -> String {
        format!(
            "[Cache #{}] SET {} = {} on {}:{}",
            self.connection_id, key, value, self.config.host, self.config.port
        )
    }
}

// ============================================================================
// Application Services
// ============================================================================

#[derive(Clone)]
struct UserRepository {
    db: Arc<DatabaseConnection>,
    cache: Arc<CacheConnection>,
}

impl Resolvable for UserRepository {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        println!("  [Creating] UserRepository");
        let db = registry.resolve::<DatabaseConnection>()?;
        let cache = registry.resolve::<CacheConnection>()?;
        Ok(UserRepository { db, cache })
    }
}

impl UserRepository {
    fn find_user(&self, id: u32) -> String {
        // Try cache first
        let cache_result = self.cache.get(&format!("user:{}", id));
        println!("  {}", cache_result);

        // If not in cache, query database
        let db_result = self.db.query(&format!("SELECT * FROM users WHERE id = {}", id));
        println!("  {}", db_result);

        // Store in cache
        let cache_set = self.cache.set(&format!("user:{}", id), "user_data");
        println!("  {}", cache_set);

        format!("User #{}", id)
    }
}

#[derive(Clone)]
struct UserService {
    repository: Arc<UserRepository>,
}

impl Resolvable for UserService {
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
        println!("  [Creating] UserService");
        let repository = registry.resolve::<UserRepository>()?;
        Ok(UserService { repository })
    }
}

impl UserService {
    fn get_user(&self, id: u32) -> String {
        println!("\nUserService::get_user({})", id);
        self.repository.find_user(id)
    }
}

// ============================================================================
// Main Example
// ============================================================================

fn main() {
    println!("=== Auto-Resolution Example ===\n");

    // Create service registry
    let mut registry = ServiceRegistry::new();

    println!("Step 1: Registering services\n");

    // Register configuration services as singletons
    registry.register(Scope::Singleton, || {
        println!("  [Factory] DatabaseConfig");
        Arc::new(DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "myapp".to_string(),
        })
    });

    registry.register(Scope::Singleton, || {
        println!("  [Factory] CacheConfig");
        Arc::new(CacheConfig {
            host: "localhost".to_string(),
            port: 6379,
        })
    });

    // Register connections as singletons (shared across application)
    registry.register(Scope::Singleton, || {
        let config = Arc::new(DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "myapp".to_string(),
        });

        static mut COUNTER: u32 = 0;
        let connection_id = unsafe {
            COUNTER += 1;
            COUNTER
        };

        println!("  [Factory] DatabaseConnection #{}", connection_id);
        Arc::new(DatabaseConnection {
            config,
            connection_id,
        })
    });

    registry.register(Scope::Singleton, || {
        let config = Arc::new(CacheConfig {
            host: "localhost".to_string(),
            port: 6379,
        });

        static mut COUNTER: u32 = 0;
        let connection_id = unsafe {
            COUNTER += 1;
            COUNTER
        };

        println!("  [Factory] CacheConnection #{}", connection_id);
        Arc::new(CacheConnection {
            config,
            connection_id,
        })
    });

    println!("\nStep 2: Resolving UserRepository (auto-injects dependencies)\n");

    // Resolve UserRepository - dependencies are automatically injected!
    let repo = UserRepository::resolve(&registry).unwrap();
    println!("\n  Successfully created UserRepository");
    println!("  - Database connection ID: {}", repo.db.connection_id);
    println!("  - Cache connection ID: {}", repo.cache.connection_id);

    println!("\nStep 3: Resolving UserService (nested dependencies)\n");

    // Register UserRepository for UserService to resolve
    registry.register(Scope::Singleton, || {
        let db = Arc::new(DatabaseConnection {
            config: Arc::new(DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "myapp".to_string(),
            }),
            connection_id: 1,
        });

        let cache = Arc::new(CacheConnection {
            config: Arc::new(CacheConfig {
                host: "localhost".to_string(),
                port: 6379,
            }),
            connection_id: 1,
        });

        println!("  [Factory] UserRepository");
        Arc::new(UserRepository { db, cache })
    });

    // Resolve UserService - auto-resolves UserRepository
    let service = UserService::resolve(&registry).unwrap();
    println!("\n  Successfully created UserService");

    println!("\nStep 4: Using the service\n");

    // Use the service
    let user = service.get_user(123);
    println!("\n  Result: {}", user);

    println!("\n=== Singleton Behavior Demo ===\n");

    println!("Resolving UserRepository twice...\n");

    let repo1 = registry.resolve::<DatabaseConnection>().unwrap();
    let repo2 = registry.resolve::<DatabaseConnection>().unwrap();

    println!("\nConnection ID #1: {}", repo1.connection_id);
    println!("Connection ID #2: {}", repo2.connection_id);
    println!(
        "\nSame instance? {}",
        repo1.connection_id == repo2.connection_id
    );

    println!("\n=== Transient Behavior Demo ===\n");

    let mut transient_registry = ServiceRegistry::new();

    // Register as transient (new instance each time)
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let counter_clone = counter.clone();

    transient_registry.register(Scope::Transient, move || {
        let mut count = counter_clone.lock().unwrap();
        *count += 1;
        let id = *count;

        println!("  [Factory] Creating DatabaseConnection #{}", id);
        Arc::new(DatabaseConnection {
            config: Arc::new(DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "myapp".to_string(),
            }),
            connection_id: id,
        })
    });

    println!("Resolving DatabaseConnection twice (Transient)...\n");

    let conn1 = transient_registry.resolve::<DatabaseConnection>().unwrap();
    let conn2 = transient_registry.resolve::<DatabaseConnection>().unwrap();

    println!("\nConnection ID #1: {}", conn1.connection_id);
    println!("Connection ID #2: {}", conn2.connection_id);
    println!(
        "\nDifferent instances? {}",
        conn1.connection_id != conn2.connection_id
    );

    println!("\n=== Example Complete ===\n");
}
