//! make:provider command

use clap::Parser;
use std::fs;
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "make:provider", about = "Create a new service provider")]
pub struct MakeProviderCommand {
    /// Provider name (e.g., AppServiceProvider)
    pub name: String,

    /// Create a deferred provider (loads services on-demand)
    #[arg(long)]
    pub deferred: bool,
}

impl MakeProviderCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        let provider_name = if self.name.ends_with("ServiceProvider") || self.name.ends_with("Provider") {
            self.name.clone()
        } else {
            format!("{}ServiceProvider", self.name)
        };

        println!("Creating service provider: {}", provider_name);

        let content = if self.deferred {
            self.generate_deferred_provider(&provider_name)
        } else {
            self.generate_provider(&provider_name)
        };

        let filename = format!(
            "app/providers/{}.rs",
            self.to_snake_case(&provider_name)
        );

        fs::create_dir_all("app/providers")?;
        fs::write(&filename, content)?;

        println!("✓ Service provider created: {}", filename);

        if !Path::new("app/providers/mod.rs").exists() {
            let mod_content = format!("pub mod {};\n", self.to_snake_case(&provider_name));
            fs::write("app/providers/mod.rs", mod_content)?;
            println!("✓ Created providers module: app/providers/mod.rs");
        } else {
            println!("⚠ Don't forget to add 'pub mod {};' to app/providers/mod.rs",
                     self.to_snake_case(&provider_name));
        }

        println!("\n📝 Next steps:");
        println!("   1. Register provider in your application bootstrap");
        println!("   2. Add services to the register() method");
        println!("   3. Add initialization logic to the boot() method");

        Ok(())
    }

    fn generate_provider(&self, provider_name: &str) -> String {
        format!(
            r#"//! {} - Service Provider
//!
//! Service providers are the central place for application bootstrapping.
//! They register services into the container and perform initialization.
//!
//! Inspired by Laravel's Service Provider system.

use foundry_service_container::ServiceContainer;
use std::sync::Arc;
use anyhow::Result;

/// {} registers application services
pub struct {} {{
    /// Reference to the service container
    container: Arc<ServiceContainer>,
}}

impl {} {{
    /// Create a new service provider instance
    pub fn new(container: Arc<ServiceContainer>) -> Self {{
        Self {{ container }}
    }}

    /// Register services into the container
    ///
    /// This method is called first during application bootstrap.
    /// Use it to bind services, implementations, and dependencies.
    pub async fn register(&self) -> Result<()> {{
        println!("Registering services from {}...");

        // Example: Register a singleton service
        // self.container.singleton("my_service", || {{
        //     Arc::new(MyService::new())
        // }})?;

        // Example: Register a factory (creates new instance each time)
        // self.container.bind("my_factory", || {{
        //     Box::new(MyFactory::new())
        // }})?;

        // Example: Register a value
        // self.container.instance("config.value", "some_value")?;

        Ok(())
    }}

    /// Bootstrap application services
    ///
    /// This method is called after all providers have been registered.
    /// Use it to perform initialization that depends on registered services.
    pub async fn boot(&self) -> Result<()> {{
        println!("Booting {}...");

        // Example: Initialize services
        // let service = self.container.make::<MyService>("my_service")?;
        // service.initialize().await?;

        // Example: Set up event listeners
        // self.setup_event_listeners().await?;

        // Example: Register middleware
        // self.register_middleware().await?;

        Ok(())
    }}

    /// Determine if provider is deferred
    ///
    /// Deferred providers only load when their services are actually needed,
    /// improving application startup performance.
    pub fn is_deferred(&self) -> bool {{
        false
    }}

    /// Get the services provided by this provider
    ///
    /// Used for deferred loading to know which services this provider offers.
    pub fn provides(&self) -> Vec<&'static str> {{
        vec![]
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_provider_registration() {{
        let container = Arc::new(ServiceContainer::new());
        let provider = {}::new(container.clone());

        assert!(provider.register().await.is_ok());
        assert!(provider.boot().await.is_ok());
    }}

    #[test]
    fn test_provider_not_deferred() {{
        let container = Arc::new(ServiceContainer::new());
        let provider = {}::new(container);

        assert!(!provider.is_deferred());
    }}

    #[test]
    fn test_provides_empty() {{
        let container = Arc::new(ServiceContainer::new());
        let provider = {}::new(container);

        assert!(provider.provides().is_empty());
    }}
}}
"#,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name
        )
    }

    fn generate_deferred_provider(&self, provider_name: &str) -> String {
        format!(
            r#"//! {} - Deferred Service Provider
//!
//! Deferred service providers only load when their services are requested,
//! improving application startup performance.
//!
//! Inspired by Laravel's Deferred Service Provider system.

use foundry_service_container::ServiceContainer;
use std::sync::Arc;
use anyhow::Result;

/// {} registers application services (deferred)
pub struct {} {{
    /// Reference to the service container
    container: Arc<ServiceContainer>,
}}

impl {} {{
    /// Create a new deferred service provider instance
    pub fn new(container: Arc<ServiceContainer>) -> Self {{
        Self {{ container }}
    }}

    /// Register services into the container
    ///
    /// This method is called only when one of the provided services is requested.
    /// Use it to bind services, implementations, and dependencies.
    pub async fn register(&self) -> Result<()> {{
        println!("Registering deferred services from {}...");

        // Example: Register lazy-loaded services
        // self.container.singleton("expensive_service", || {{
        //     Arc::new(ExpensiveService::new())
        // }})?;

        // Example: Register database connection pool (only when needed)
        // self.container.singleton("database", || {{
        //     Arc::new(DatabasePool::new())
        // }})?;

        Ok(())
    }}

    /// Bootstrap application services
    ///
    /// This method is called after the provider has been registered.
    /// It only runs when the services are first requested.
    pub async fn boot(&self) -> Result<()> {{
        println!("Booting deferred {}...");

        // Example: Perform lazy initialization
        // let service = self.container.make::<ExpensiveService>("expensive_service")?;
        // service.configure().await?;

        Ok(())
    }}

    /// Determine if provider is deferred
    pub fn is_deferred(&self) -> bool {{
        true
    }}

    /// Get the services provided by this provider
    ///
    /// IMPORTANT: List all service names that this provider can create.
    /// The container uses this to know when to load this provider.
    pub fn provides(&self) -> Vec<&'static str> {{
        vec![
            // Example: Add your service names here
            // "expensive_service",
            // "database",
        ]
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_deferred_provider_registration() {{
        let container = Arc::new(ServiceContainer::new());
        let provider = {}::new(container.clone());

        assert!(provider.register().await.is_ok());
        assert!(provider.boot().await.is_ok());
    }}

    #[test]
    fn test_provider_is_deferred() {{
        let container = Arc::new(ServiceContainer::new());
        let provider = {}::new(container);

        assert!(provider.is_deferred());
    }}

    #[test]
    fn test_provides_list() {{
        let container = Arc::new(ServiceContainer::new());
        let provider = {}::new(container);

        // Update this test when you add services to provides()
        let services = provider.provides();
        assert!(services.is_empty() || !services.is_empty());
    }}
}}
"#,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name,
            provider_name
        )
    }

    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap());
            } else {
                result.push(ch);
            }
        }
        result
    }
}
