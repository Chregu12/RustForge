use anyhow::Result;
use colored::*;
use std::process::Command;

use super::ensure_forge_project;

pub async fn run(host: &str, port: u16) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", "Starting development server...".green().bold());
    println!();
    println!("  {} Host: {}", "•".cyan(), host);
    println!("  {} Port: {}", "•".cyan(), port);
    println!();
    println!("{}", format!("🚀 Server will start at http://{}:{}", host, port).bright_cyan().bold());
    println!();
    println!("{}", "Press Ctrl+C to stop".yellow());
    println!();

    // Run cargo run with environment variables
    let status = Command::new("cargo")
        .arg("run")
        .env("HOST", host)
        .env("PORT", port.to_string())
        .status()?;

    if !status.success() {
        anyhow::bail!("Server failed to start");
    }

    Ok(())
}
