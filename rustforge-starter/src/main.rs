//! RustForge application entry point.
//!
//! The Laravel-style `app/` and `routes/` directories live at the project root
//! (not under `src/`), so they are attached here with `#[path]`.

// PascalCase module directories (Http, Models, ...) mirror Laravel's layout.
#![allow(non_snake_case)]

#[path = "../app/mod.rs"]
mod app;
#[path = "../routes/mod.rs"]
mod routes;

use rf_web::RouterBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from `.env`, if present.
    dotenvy::dotenv().ok();

    // Initialize logging.
    tracing_subscriber::fmt().with_target(false).init();

    // Build the base router (tracing + CORS provided by the framework) and
    // merge in the application's web and API routes.
    let app = RouterBuilder::new()
        .with_tracing(true)
        .with_cors(true)
        .build()
        .merge(routes::web::routes())
        .merge(routes::api::routes());

    // Resolve the listen address from the environment.
    let host = std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("APP_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{host}:{port}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🔥 RustForge running at http://{addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
