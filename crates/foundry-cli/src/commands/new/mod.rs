mod config;
mod generator;
mod templates;
mod wizard;

use color_eyre::Result;
use config::ProjectConfig;
use generator::ProjectGenerator;
use std::path::PathBuf;
use wizard::ProjectWizard;

/// Create a new RustForge project
pub async fn execute(name: String, skip_wizard: bool) -> Result<()> {
    // Check if directory already exists
    let project_path = PathBuf::from(&name);
    if project_path.exists() {
        color_eyre::eyre::bail!(
            "Directory '{}' already exists. Please choose a different name or remove the existing directory.",
            name
        );
    }

    // Run interactive wizard to gather project configuration
    let wizard = ProjectWizard::new();
    let config = wizard.run(name, skip_wizard)?;

    // Generate project
    let generator = ProjectGenerator::new(config);
    generator.generate().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exists() {
        // Simple test to verify module compiles
        assert!(true);
    }
}
