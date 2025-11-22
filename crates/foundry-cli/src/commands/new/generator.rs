use crate::commands::new::config::{ProjectConfig, TemplateType};
use crate::commands::new::templates::api_rest::ApiRestTemplate;
use color_eyre::Result;
use std::process::Command;

pub struct ProjectGenerator {
    config: ProjectConfig,
}

impl ProjectGenerator {
    pub fn new(config: ProjectConfig) -> Self {
        Self { config }
    }

    /// Generate the complete project
    pub async fn generate(&self) -> Result<()> {
        println!();
        println!("✨ Creating project...");
        println!();

        // Generate project files based on template
        self.generate_template()?;
        println!("  ✅ Generated project structure");

        // Finalize project setup
        self.finalize().await?;

        // Show success message
        self.show_success_message();

        Ok(())
    }

    /// Generate files based on template type
    fn generate_template(&self) -> Result<()> {
        match self.config.template {
            TemplateType::ApiRest => {
                let template = ApiRestTemplate::new(self.config.clone());
                template.generate(&self.config.path)?;
            }
            TemplateType::FullStackReact => {
                // TODO: Implement Full-Stack React template
                println!("  ⚠️  Full-Stack React template not yet implemented");
                println!("  📝 Using API REST template as fallback");
                let template = ApiRestTemplate::new(self.config.clone());
                template.generate(&self.config.path)?;
            }
            TemplateType::FullStackLeptos => {
                // TODO: Implement Full-Stack Leptos template
                println!("  ⚠️  Full-Stack Leptos template not yet implemented");
                println!("  📝 Using API REST template as fallback");
                let template = ApiRestTemplate::new(self.config.clone());
                template.generate(&self.config.path)?;
            }
            TemplateType::CliTool => {
                // TODO: Implement CLI Tool template
                println!("  ⚠️  CLI Tool template not yet implemented");
                println!("  📝 Using API REST template as fallback");
                let template = ApiRestTemplate::new(self.config.clone());
                template.generate(&self.config.path)?;
            }
        }

        Ok(())
    }

    /// Finalize project setup
    async fn finalize(&self) -> Result<()> {
        // Initialize git repository
        self.init_git()?;

        // Run cargo check
        self.run_cargo_check()?;

        // Setup database if needed
        if self.config.has_database() {
            self.setup_database().await?;
        }

        Ok(())
    }

    /// Initialize git repository
    fn init_git(&self) -> Result<()> {
        print!("  🔄 Initializing git repository...");

        let output = Command::new("git")
            .arg("init")
            .current_dir(&self.config.path)
            .output()?;

        if output.status.success() {
            println!(" ✅");

            // Initial commit
            Command::new("git")
                .args(["add", "."])
                .current_dir(&self.config.path)
                .output()?;

            Command::new("git")
                .args(["commit", "-m", "Initial commit from RustForge"])
                .current_dir(&self.config.path)
                .output()?;
        } else {
            println!(" ⚠️  (git not available)");
        }

        Ok(())
    }

    /// Run cargo check to verify project compiles
    fn run_cargo_check(&self) -> Result<()> {
        print!("  🔄 Running cargo check...");

        let output = Command::new("cargo")
            .arg("check")
            .current_dir(&self.config.path)
            .output()?;

        if output.status.success() {
            println!(" ✅");
        } else {
            println!(" ⚠️");
            println!("\n  Warning: cargo check failed. You may need to fix dependencies.");
            println!("  Error output:");
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Setup database (create database and run migrations)
    async fn setup_database(&self) -> Result<()> {
        if let Some(db_config) = &self.config.database {
            print!("  🔄 Setting up database...");

            // Try to create database
            match self.create_database(db_config).await {
                Ok(_) => {
                    println!(" ✅");
                    println!("  ✅ Created database '{}'", db_config.name);
                }
                Err(e) => {
                    println!(" ⚠️");
                    println!("  ⚠️  Could not create database: {}", e);
                    println!("  💡 You may need to create it manually:");
                    println!(
                        "     psql -U {} -c 'CREATE DATABASE {};'",
                        db_config.username, db_config.name
                    );
                }
            }

            // Try to run migrations
            if let Err(e) = self.run_migrations().await {
                println!("  ⚠️  Could not run migrations: {}", e);
                println!("  💡 You may need to run migrations manually with sqlx-cli");
            }
        }

        Ok(())
    }

    /// Create PostgreSQL database
    async fn create_database(
        &self,
        db_config: &crate::commands::new::config::DatabaseConfig,
    ) -> Result<()> {
        use sqlx::postgres::PgPoolOptions;

        // Connect to postgres database
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&db_config.connection_url_without_db())
            .await?;

        // Create new database
        let query = format!("CREATE DATABASE {}", db_config.name);
        sqlx::query(&query).execute(&pool).await?;

        Ok(())
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        // Check if sqlx-cli is installed
        let output = Command::new("sqlx").arg("--version").output();

        if output.is_ok() && output.unwrap().status.success() {
            print!("  🔄 Running migrations...");

            let migration_output = Command::new("sqlx")
                .args(["migrate", "run"])
                .current_dir(&self.config.path)
                .output()?;

            if migration_output.status.success() {
                println!(" ✅");
            } else {
                println!(" ⚠️");
                return Err(color_eyre::eyre::eyre!(
                    "Migration failed: {}",
                    String::from_utf8_lossy(&migration_output.stderr)
                ));
            }
        } else {
            println!("  ℹ️  sqlx-cli not installed, skipping migrations");
            println!("  💡 Install with: cargo install sqlx-cli");
        }

        Ok(())
    }

    /// Show success message with next steps
    fn show_success_message(&self) {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🎉 Project '{}' created successfully!", self.config.name);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("📋 Next steps:");
        println!();
        println!("  1. Navigate to your project:");
        println!("     cd {}", self.config.name);
        println!();

        if self.config.has_database() {
            println!("  2. Ensure PostgreSQL is running");
            println!();
            println!("  3. Run the application:");
        } else {
            println!("  2. Run the application:");
        }
        println!("     cargo run");
        println!();

        println!("  The server will start on http://localhost:3000");
        println!();

        if self.config.has_auth() {
            println!("📚 API Endpoints:");
            println!("  - GET  /health              - Health check");
            println!("  - POST /api/auth/register   - Register new user");
            println!("  - POST /api/auth/login      - Login user");
            println!();
        }

        println!("📖 Documentation:");
        println!("  - Check the .env file for configuration");
        println!("  - Read the generated code for implementation details");
        println!("  - Customize the templates to fit your needs");
        println!();

        println!("🚀 Happy coding with RustForge!");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::new::config::{Feature, TemplateType};
    use std::path::PathBuf;

    #[test]
    fn test_generator_creation() {
        let config = ProjectConfig {
            name: "test_app".to_string(),
            path: PathBuf::from("/tmp/test_app"),
            template: TemplateType::ApiRest,
            features: vec![Feature::Database, Feature::Tests],
            database: Some(crate::commands::new::config::DatabaseConfig::default()),
        };

        let generator = ProjectGenerator::new(config);
        assert!(std::mem::size_of_val(&generator) > 0);
    }
}
