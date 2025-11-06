use anyhow::Result;
use foundry_api::http::HttpServer;
use foundry_api::invocation::FoundryInvoker;
use foundry_application::FoundryApp;
use foundry_infra::{LocalArtifactPort, SeaOrmMigrationService, SeaOrmSeedService};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

fn load_config() -> Result<Value> {
    // Simple configuration for testing
    let config = serde_json::json!({
        "app_name": "RustForge Test Application",
        "environment": "development",
        "debug": true
    });

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing();

    info!("🚀 Starting RustForge Test Application");

    // Load configuration
    let config = load_config()?;
    info!("✅ Configuration loaded");

    // Initialize framework components
    let artifacts = Arc::new(LocalArtifactPort::default());
    let migrations = Arc::new(SeaOrmMigrationService::default());
    let seeds = Arc::new(SeaOrmSeedService::default());

    info!("✅ Framework components initialized");

    // Bootstrap the application
    let app = FoundryApp::bootstrap(config, artifacts, migrations, seeds)?;
    info!("✅ FoundryApp bootstrapped successfully");

    // Create the invoker
    let invoker = FoundryInvoker::new(app);
    info!("✅ FoundryInvoker created");

    // Test command execution
    info!("🧪 Testing command execution...");

    // Test 'list' command
    let result = invoker
        .invoke_command("list", vec![], foundry_plugins::ResponseFormat::Human, Default::default())
        .await?;

    info!("✅ 'list' command executed successfully");
    info!("   Status: {:?}", result.status);
    if let Some(message) = &result.message {
        info!("   Message: {}", message);
    }

    // Start HTTP server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("🌐 Starting HTTP server on {}", addr);
    info!("   Test the API with:");
    info!("   curl http://localhost:8080/api/commands");
    info!("   curl -X POST http://localhost:8080/api/invoke -H 'Content-Type: application/json' -d '{{\"command\":\"list\"}}'");

    // Start the server (this will block)
    HttpServer::new(invoker)
        .serve(addr)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}
