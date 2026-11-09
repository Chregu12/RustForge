//! Application route inspection.

use anyhow::{ensure, Context, Result};
use std::process::Command;

use super::ensure_forge_project;

pub async fn list(method_filter: Option<&str>, path_filter: Option<&str>) -> Result<()> {
    ensure_forge_project()?;
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--rustforge-route-list"])
        .output()
        .context("Run the application's route listing")?;
    ensure!(
        output.status.success(),
        "Application route listing failed:\n{}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let routes = String::from_utf8(output.stdout).context("Read application route listing")?;
    println!("{:<8} URI", "Method");
    for line in routes.lines() {
        let (method, path) = line
            .split_once('\t')
            .context("Application must output METHOD followed by a tab and URI")?;
        if method_filter.is_none_or(|filter| method.eq_ignore_ascii_case(filter))
            && path_filter.is_none_or(|filter| path.contains(filter))
        {
            println!("{method:<8} {path}");
        }
    }
    Ok(())
}

pub async fn cache() -> Result<()> {
    anyhow::bail!("Route caching is unavailable. Routes are compiled into the application.");
}

pub async fn clear() -> Result<()> {
    anyhow::bail!("No compiled route cache is managed by Forge.");
}
