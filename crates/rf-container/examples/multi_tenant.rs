//! Multi-Tenant Example
//!
//! Demonstrates tenant-scoped services where each tenant has isolated
//! database connections, cache instances, and configuration.
//!
//! Run with: cargo run --example multi_tenant

use rf_container::{ScopeManager, ScopedContainer, ServiceRegistry, Scope};
use std::sync::{Arc, Mutex};

/// Tenant context extracted from request
#[derive(Clone, Debug)]
struct TenantContext {
    tenant_id: String,
    domain: String,
}

thread_local! {
    static TENANT_CONTEXT: std::cell::RefCell<Option<TenantContext>> = std::cell::RefCell::new(None);
}

impl TenantContext {
    fn set_current(ctx: TenantContext) {
        TENANT_CONTEXT.with(|c| {
            *c.borrow_mut() = Some(ctx);
        });
    }

    fn current() -> Option<TenantContext> {
        TENANT_CONTEXT.with(|c| c.borrow().clone())
    }
}

/// Tenant-specific database connection
#[derive(Clone)]
struct TenantDatabaseConnection {
    tenant_id: String,
    schema: String,
    connection_id: u32,
}

impl TenantDatabaseConnection {
    fn new(connection_id: u32) -> Self {
        let ctx = TenantContext::current().expect("No tenant context");
        println!(
            "🔌 [Tenant: {}] Creating database connection #{}",
            ctx.tenant_id, connection_id
        );

        Self {
            tenant_id: ctx.tenant_id.clone(),
            schema: format!("tenant_{}", ctx.tenant_id),
            connection_id,
        }
    }

    fn execute(&self, query: &str) {
        println!(
            "📊 [Tenant: {}][Conn: {}] SET search_path TO {}; {}",
            self.tenant_id, self.connection_id, self.schema, query
        );
    }

    fn get_schema(&self) -> &str {
        &self.schema
    }
}

/// Tenant-specific configuration
#[derive(Clone)]
struct TenantConfig {
    tenant_id: String,
    max_users: u32,
    features: Vec<String>,
}

impl TenantConfig {
    fn new() -> Self {
        let ctx = TenantContext::current().expect("No tenant context");
        println!("⚙️  [Tenant: {}] Loading configuration", ctx.tenant_id);

        // In production, load from database
        let (max_users, features) = match ctx.tenant_id.as_str() {
            "acme-corp" => (100, vec!["advanced".to_string(), "analytics".to_string()]),
            "startup-inc" => (10, vec!["basic".to_string()]),
            "enterprise-llc" => (
                1000,
                vec![
                    "advanced".to_string(),
                    "analytics".to_string(),
                    "api".to_string(),
                ],
            ),
            _ => (5, vec!["basic".to_string()]),
        };

        Self {
            tenant_id: ctx.tenant_id,
            max_users,
            features,
        }
    }

    fn has_feature(&self, feature: &str) -> bool {
        self.features.contains(&feature.to_string())
    }
}

/// Tenant-scoped cache
#[derive(Clone)]
struct TenantCache {
    tenant_id: String,
    prefix: String,
    data: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl TenantCache {
    fn new() -> Self {
        let ctx = TenantContext::current().expect("No tenant context");
        println!("💾 [Tenant: {}] Initializing cache", ctx.tenant_id);

        Self {
            tenant_id: ctx.tenant_id.clone(),
            prefix: format!("tenant:{}:", ctx.tenant_id),
            data: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn set(&self, key: &str, value: String) {
        let full_key = format!("{}{}", self.prefix, key);
        self.data.lock().unwrap().insert(full_key, value);
    }

    fn get(&self, key: &str) -> Option<String> {
        let full_key = format!("{}{}", self.prefix, key);
        self.data.lock().unwrap().get(&full_key).cloned()
    }
}

/// Application service using tenant-scoped dependencies
struct ProductService {
    db: Arc<TenantDatabaseConnection>,
    config: Arc<TenantConfig>,
    cache: Arc<TenantCache>,
}

impl ProductService {
    fn new(
        db: Arc<TenantDatabaseConnection>,
        config: Arc<TenantConfig>,
        cache: Arc<TenantCache>,
    ) -> Self {
        Self { db, config, cache }
    }

    fn list_products(&self) -> Vec<String> {
        // Check cache
        if let Some(cached) = self.cache.get("products") {
            println!("  ✓ Products loaded from cache");
            return vec![cached];
        }

        // Query tenant's schema
        self.db
            .execute("SELECT * FROM products WHERE active = true");

        let products = vec![
            format!("Product A ({})", self.config.tenant_id),
            format!("Product B ({})", self.config.tenant_id),
        ];

        // Cache for next time
        self.cache.set("products", products.join(", "));

        products
    }

    fn get_analytics(&self) -> Result<String, &'static str> {
        if !self.config.has_feature("analytics") {
            return Err("Analytics feature not available for this tenant");
        }

        self.db.execute("SELECT COUNT(*) FROM analytics_events");
        Ok("Analytics data retrieved".to_string())
    }
}

#[tokio::main]
async fn main() {
    println!("🏢 Multi-Tenant Application Example\n");

    // Setup service registry
    let mut registry = ServiceRegistry::new();

    // Register tenant-scoped database
    let db_counter = Arc::new(Mutex::new(0u32));
    let db_counter_clone = db_counter.clone();
    registry.register(Scope::Scoped, move || {
        let mut counter = db_counter_clone.lock().unwrap();
        *counter += 1;
        Arc::new(TenantDatabaseConnection::new(*counter))
    });

    // Register tenant-scoped config
    registry.register(Scope::Scoped, || Arc::new(TenantConfig::new()));

    // Register tenant-scoped cache
    registry.register(Scope::Scoped, || Arc::new(TenantCache::new()));

    let registry = Arc::new(registry);
    let scope_manager = ScopeManager::new(registry);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Processing requests from different tenants");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Simulate requests from different tenants
    let tenants = vec![
        TenantContext {
            tenant_id: "acme-corp".to_string(),
            domain: "acme.example.com".to_string(),
        },
        TenantContext {
            tenant_id: "startup-inc".to_string(),
            domain: "startup.example.com".to_string(),
        },
        TenantContext {
            tenant_id: "enterprise-llc".to_string(),
            domain: "enterprise.example.com".to_string(),
        },
    ];

    for (i, tenant) in tenants.iter().enumerate() {
        println!("\n📨 Request {} - Domain: {}", i + 1, tenant.domain);
        println!("────────────────────────────────────────");

        let tenant_clone = tenant.clone();
        let scope_id = format!("request-{}-{}", tenant.tenant_id, i);

        scope_manager
            .with_scope(scope_id, async move {
                // Set tenant context for this request
                TenantContext::set_current(tenant_clone);

                // Resolve tenant-scoped services
                let scope = ScopedContainer::current().unwrap();

                let db: Arc<TenantDatabaseConnection> = scope.resolve().unwrap();
                let config: Arc<TenantConfig> = scope.resolve().unwrap();
                let cache: Arc<TenantCache> = scope.resolve().unwrap();

                println!(
                    "  Schema: {}, Max Users: {}, Features: {:?}",
                    db.get_schema(),
                    config.max_users,
                    config.features
                );

                // Create application service
                let product_service = ProductService::new(
                    Arc::clone(&db),
                    Arc::clone(&config),
                    Arc::clone(&cache),
                );

                // List products (will hit DB first time)
                println!("\n  📦 Listing products:");
                let products = product_service.list_products();
                for product in &products {
                    println!("    - {}", product);
                }

                // Try to get analytics
                println!("\n  📈 Requesting analytics:");
                match product_service.get_analytics() {
                    Ok(data) => println!("    ✓ {}", data),
                    Err(err) => println!("    ✗ {}", err),
                }

                // List products again (will hit cache)
                println!("\n  📦 Listing products again:");
                let _products = product_service.list_products();

                println!("\n✅ Request for {} completed", config.tenant_id);
            })
            .await;
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("All tenant requests processed successfully!");
    println!("Each tenant had isolated services and data");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
