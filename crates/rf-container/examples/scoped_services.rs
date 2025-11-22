//! Scoped Services Example
//!
//! Demonstrates request-scoped and tenant-scoped services that are created
//! once per scope and shared within that scope.
//!
//! Run with: cargo run --example scoped_services

use rf_container::{Scope, ScopeManager, ScopedContainer, ServiceRegistry};
use std::sync::{Arc, Mutex};

/// Request-scoped logger that includes a unique request ID
#[derive(Clone)]
struct RequestLogger {
    request_id: String,
    entries: Arc<Mutex<Vec<String>>>,
}

impl RequestLogger {
    fn new(request_id: String) -> Self {
        println!("📝 Creating RequestLogger for: {}", request_id);
        Self {
            request_id: request_id.clone(),
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn log(&self, message: &str) {
        let formatted = format!("[{}] {}", self.request_id, message);
        println!("{}", formatted);
        self.entries.lock().unwrap().push(formatted);
    }

    fn get_entries(&self) -> Vec<String> {
        self.entries.lock().unwrap().clone()
    }
}

/// Tenant-scoped database connection
#[derive(Clone)]
struct TenantDatabase {
    tenant_id: String,
    connection: String,
}

impl TenantDatabase {
    fn new(tenant_id: String) -> Self {
        let connection = format!("postgres://db/{}", tenant_id);
        println!("🔌 Connecting to database: {}", connection);
        Self {
            tenant_id,
            connection,
        }
    }

    fn query(&self, sql: &str) {
        println!("[DB:{}] {}", self.tenant_id, sql);
    }

    fn get_connection(&self) -> &str {
        &self.connection
    }
}

/// Request-scoped cache instance
#[derive(Clone)]
struct RequestCache {
    scope_id: String,
    data: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl RequestCache {
    fn new(scope_id: String) -> Self {
        println!("💾 Creating RequestCache for: {}", scope_id);
        Self {
            scope_id,
            data: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn set(&self, key: String, value: String) {
        self.data.lock().unwrap().insert(key, value);
    }

    fn get(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().get(key).cloned()
    }
}

/// Application service that uses scoped dependencies
struct UserService {
    logger: Arc<RequestLogger>,
    db: Arc<TenantDatabase>,
    cache: Arc<RequestCache>,
}

impl UserService {
    fn new(logger: Arc<RequestLogger>, db: Arc<TenantDatabase>, cache: Arc<RequestCache>) -> Self {
        Self { logger, db, cache }
    }

    fn get_user(&self, user_id: u32) -> String {
        self.logger.log(&format!("Fetching user {}", user_id));

        // Check cache first
        let cache_key = format!("user:{}", user_id);
        if let Some(cached) = self.cache.get(&cache_key) {
            self.logger.log("User found in cache");
            return cached;
        }

        // Query database
        self.db
            .query(&format!("SELECT * FROM users WHERE id = {}", user_id));

        let user = format!("User {}", user_id);
        self.cache.set(cache_key, user.clone());

        self.logger.log("User loaded from database");
        user
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 Scoped Services Example\n");

    // Setup service registry
    let mut registry = ServiceRegistry::new();

    // Register request-scoped logger
    let request_counter = Arc::new(Mutex::new(0u32));
    let request_counter_clone = request_counter.clone();
    registry.register(Scope::Scoped, move || {
        let mut counter = request_counter_clone.lock().unwrap();
        *counter += 1;
        let request_id = format!("REQ-{:04}", *counter);
        Arc::new(RequestLogger::new(request_id))
    });

    // Register tenant-scoped database
    registry.register(Scope::Scoped, || {
        // In production, this would come from request context
        let tenant_id = "tenant-123".to_string();
        Arc::new(TenantDatabase::new(tenant_id))
    });

    // Register request-scoped cache
    let cache_counter = Arc::new(Mutex::new(0u32));
    let cache_counter_clone = cache_counter.clone();
    registry.register(Scope::Scoped, move || {
        let mut counter = cache_counter_clone.lock().unwrap();
        *counter += 1;
        let scope_id = format!("CACHE-{:04}", *counter);
        Arc::new(RequestCache::new(scope_id))
    });

    let registry = Arc::new(registry);
    let scope_manager = ScopeManager::new(registry);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Simulating 3 HTTP requests");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Simulate multiple HTTP requests
    for i in 1..=3 {
        let scope_id = format!("request-{}", i);
        println!("\n📨 Request {} started", i);
        println!("────────────────────────────────────────");

        scope_manager
            .with_scope(scope_id.clone(), async move {
                // Resolve scoped services
                let scope = ScopedContainer::current().unwrap();

                let logger: Arc<RequestLogger> = scope.resolve().unwrap();
                let db: Arc<TenantDatabase> = scope.resolve().unwrap();
                let cache: Arc<RequestCache> = scope.resolve().unwrap();

                logger.log("Request processing started");

                // Create application service
                let user_service =
                    UserService::new(Arc::clone(&logger), Arc::clone(&db), Arc::clone(&cache));

                // First call - hits database
                let user = user_service.get_user(42);
                logger.log(&format!("Result: {}", user));

                // Second call - hits cache
                let user = user_service.get_user(42);
                logger.log(&format!("Result: {}", user));

                logger.log("Request processing completed");

                // Show request log
                println!("\n📊 Request Log:");
                for entry in logger.get_entries() {
                    println!("  {}", entry);
                }

                println!("✅ Request {} completed\n", i);
            })
            .await;
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("All requests processed successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
