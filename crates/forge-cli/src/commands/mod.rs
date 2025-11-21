pub mod about;
pub mod cache;
pub mod config;
pub mod inspire;
pub mod mail;
pub mod make;
pub mod migrate;
pub mod new;
pub mod optimize;
pub mod queue;
pub mod route;
pub mod serve;
pub mod tinker;

use std::path::Path;
use crate::errors;

/// Check if we're in a RustForge project
pub fn is_forge_project() -> bool {
    Path::new("Cargo.toml").exists() &&
    Path::new("src").exists()
}

/// Ensure we're in a RustForge project, or show error
pub fn ensure_forge_project() -> anyhow::Result<()> {
    if !is_forge_project() {
        let error = errors::not_in_forge_project();
        error.display();
        std::process::exit(1);
    }
    Ok(())
}
