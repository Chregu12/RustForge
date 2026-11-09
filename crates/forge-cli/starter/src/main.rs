#[path = "../app/mod.rs"]
mod app;
#[path = "../bootstrap/app.rs"]
mod bootstrap;
#[path = "../config/app.rs"]
mod config;
#[path = "../routes/mod.rs"]
mod routes;

use anyhow::{bail, Result};
use rf::prelude::{DatabaseConfig, DatabaseManager};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--rustforge-route-list") if args.next().is_none() => {
            for (method, path) in bootstrap::routes().routes() {
                println!("{method}\t{path}");
            }
            return Ok(());
        }
        Some(arg) => bail!("Unknown application argument {arg}"),
        None => {}
    }

    let settings = config::Settings::load()?;
    let database = DatabaseManager::connect(DatabaseConfig {
        url: settings.database_url,
        ..Default::default()
    })
    .await?;
    let app = bootstrap::routes().database(database).build();
    let listener = tokio::net::TcpListener::bind((settings.host.as_str(), settings.port)).await?;
    println!("Listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
