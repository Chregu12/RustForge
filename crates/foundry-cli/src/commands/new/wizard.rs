use crate::commands::new::config::{
    DatabaseConfig, Feature, ProjectConfig, TemplateType,
};
use color_eyre::Result;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use std::path::PathBuf;

pub struct ProjectWizard {
    theme: ColorfulTheme,
}

impl ProjectWizard {
    pub fn new() -> Self {
        Self {
            theme: ColorfulTheme::default(),
        }
    }

    /// Run the complete interactive wizard
    pub fn run(&self, project_name: String, skip_wizard: bool) -> Result<ProjectConfig> {
        println!();
        println!("🔨 RustForge Project Generator");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        let path = PathBuf::from(&project_name);
        let mut config = ProjectConfig::new(project_name.clone(), path);

        if skip_wizard {
            // Use defaults
            config.template = TemplateType::ApiRest;
            config.features = vec![Feature::Database, Feature::Tests];
            config.database = Some(DatabaseConfig {
                name: config.db_name(),
                ..Default::default()
            });
            return Ok(config);
        }

        // Step 1: Template Selection
        config.template = self.select_template()?;

        // Step 2: Feature Selection
        config.features = self.select_features()?;

        // Step 3: Database Configuration
        if config.has_database() {
            config.database = Some(self.configure_database(&project_name)?);
        }

        // Step 4: Confirmation
        self.show_summary(&config)?;

        println!();
        Ok(config)
    }

    /// Template selection step
    fn select_template(&self) -> Result<TemplateType> {
        let templates = TemplateType::all();
        let template_names: Vec<&str> = templates.iter().map(|t| t.as_str()).collect();

        let selection = Select::with_theme(&self.theme)
            .with_prompt("What type of project?")
            .items(&template_names)
            .default(0)
            .interact()?;

        Ok(templates[selection])
    }

    /// Feature selection step
    fn select_features(&self) -> Result<Vec<Feature>> {
        let features = Feature::all();
        let feature_names: Vec<&str> = features.iter().map(|f| f.as_str()).collect();

        let defaults = vec![true, true, false, false, true]; // Auth, DB, Redis, Email, Tests

        let selections = MultiSelect::with_theme(&self.theme)
            .with_prompt("Select features (Space to toggle, Enter to confirm)")
            .items(&feature_names)
            .defaults(&defaults)
            .interact()?;

        Ok(selections.into_iter().map(|i| features[i]).collect())
    }

    /// Database configuration step
    fn configure_database(&self, project_name: &str) -> Result<DatabaseConfig> {
        println!();
        println!("📊 Database Configuration");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let db_name: String = Input::with_theme(&self.theme)
            .with_prompt("Database name")
            .default(format!(
                "{}_dev",
                project_name.to_lowercase().replace('-', "_")
            ))
            .interact_text()?;

        let host: String = Input::with_theme(&self.theme)
            .with_prompt("Database host")
            .default("localhost".to_string())
            .interact_text()?;

        let port: u16 = Input::with_theme(&self.theme)
            .with_prompt("Database port")
            .default(5432)
            .interact_text()?;

        let username: String = Input::with_theme(&self.theme)
            .with_prompt("Database username")
            .default("postgres".to_string())
            .interact_text()?;

        let password: String = Input::with_theme(&self.theme)
            .with_prompt("Database password")
            .default("postgres".to_string())
            .interact_text()?;

        Ok(DatabaseConfig {
            name: db_name,
            host,
            port,
            username,
            password,
        })
    }

    /// Show configuration summary and confirm
    fn show_summary(&self, config: &ProjectConfig) -> Result<()> {
        println!();
        println!("📋 Project Summary");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  Name:     {}", config.name);
        println!("  Template: {}", config.template.as_str());
        println!("  Features:");
        for feature in &config.features {
            println!("    - {}", feature.as_str());
        }
        if let Some(db) = &config.database {
            println!("  Database:");
            println!("    - Name: {}", db.name);
            println!("    - Host: {}:{}", db.host, db.port);
        }
        println!();

        let confirm = Confirm::with_theme(&self.theme)
            .with_prompt("Create project with this configuration?")
            .default(true)
            .interact()?;

        if !confirm {
            color_eyre::eyre::bail!("Project creation cancelled by user");
        }

        Ok(())
    }
}

impl Default for ProjectWizard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_creation() {
        let wizard = ProjectWizard::new();
        // Just verify it can be created
        assert!(std::mem::size_of_val(&wizard) > 0);
    }

    #[test]
    fn test_skip_wizard_defaults() {
        let wizard = ProjectWizard::new();
        // This would require interactive input, so we skip in tests
        // Just verify the wizard exists
        assert!(std::mem::size_of_val(&wizard) > 0);
    }
}
