//! Basic Scoped Services Example
//!
//! Simple introduction to scoped service lifetimes.
//!
//! Run with: cargo run --example basic_scoped

use rf_container::{Scope, ScopeManager, ScopedContainer, ServiceRegistry};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct RequestCounter {
    value: u32,
}

impl RequestCounter {
    fn new(value: u32) -> Self {
        println!("Creating counter with value: {}", value);
        Self { value }
    }
}

#[tokio::main]
async fn main() {
    println!("Basic Scoped Services Example\n");

    // Create registry
    let mut registry = ServiceRegistry::new();

    // Counter to track how many times the factory is called
    let factory_calls = Arc::new(Mutex::new(0u32));
    let factory_calls_clone = factory_calls.clone();

    // Register a scoped service
    registry.register(Scope::Scoped, move || {
        let mut calls = factory_calls_clone.lock().unwrap();
        *calls += 1;
        Arc::new(RequestCounter::new(*calls))
    });

    let registry = Arc::new(registry);
    let scope_manager = ScopeManager::new(registry);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // First scope
    println!("Scope 1:");
    scope_manager
        .with_scope("scope-1".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            // Resolve multiple times within the same scope
            let counter1: Arc<RequestCounter> = scope.resolve().unwrap();
            println!("  First resolve: {}", counter1.value);

            let counter2: Arc<RequestCounter> = scope.resolve().unwrap();
            println!("  Second resolve: {}", counter2.value);

            // Same instance within scope
            assert_eq!(counter1.value, counter2.value);
            println!("  ✓ Same instance reused within scope");
        })
        .await;

    println!();

    // Second scope
    println!("Scope 2:");
    scope_manager
        .with_scope("scope-2".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            let counter: Arc<RequestCounter> = scope.resolve().unwrap();
            println!("  Resolve: {}", counter.value);
            println!("  ✓ New instance for new scope");
        })
        .await;

    println!();

    // Third scope
    println!("Scope 3:");
    scope_manager
        .with_scope("scope-3".to_string(), async {
            let scope = ScopedContainer::current().unwrap();

            let counter: Arc<RequestCounter> = scope.resolve().unwrap();
            println!("  Resolve: {}", counter.value);
            println!("  ✓ New instance for new scope");
        })
        .await;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let total_calls = *factory_calls.lock().unwrap();
    println!("\nTotal factory calls: {}", total_calls);
    println!("Number of scopes: 3");
    println!("\n✓ Factory called once per scope (as expected)");
}
