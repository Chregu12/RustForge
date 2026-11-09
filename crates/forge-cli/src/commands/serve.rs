use anyhow::{ensure, Context, Result};
use tokio::process::Command;

use super::ensure_forge_project;

pub async fn run(host: &str, port: u16) -> Result<()> {
    run_configured(Some(host), Some(port)).await
}

pub async fn run_configured(host: Option<&str>, port: Option<u16>) -> Result<()> {
    ensure_forge_project()?;
    let mut command = Command::new("cargo");
    command.arg("run").kill_on_drop(true);
    if let Some(host) = host {
        command.env("HOST", host);
    }
    if let Some(port) = port {
        command.env("PORT", port.to_string());
    }
    let status = command
        .status()
        .await
        .context("Run the RustForge application")?;
    ensure!(status.success(), "Application exited with {status}");
    Ok(())
}
