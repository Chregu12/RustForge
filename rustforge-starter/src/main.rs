/// RustForge Application
///
/// This is the entry point for your RustForge application.

mod app;
mod routes;

use rf_core::prelude::*;
use rf_web::{Application, Router};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    // Bootstrap the application
    let app = Application::new()
        .config_path("config/")
        .register_routes(routes::web::routes())
        .register_routes(routes::api::routes())
        .boot()
        .await?;

    // Get server address from config
    let host = std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("APP_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    // Start the server
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║                                                      ║");
    println!("║   🔥 RustForge Application Started                  ║");
    println!("║                                                      ║");
    println!("║   Server running at: http://{}              ║", addr);
    println!("║                                                      ║");
    println!("║   Press Ctrl+C to stop                              ║");
    println!("║                                                      ║");
    println!("╚══════════════════════════════════════════════════════╝");

    app.serve(&addr).await?;

    Ok(())
}
