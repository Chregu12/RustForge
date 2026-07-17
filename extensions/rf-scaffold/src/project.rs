//! Project scaffolding

use crate::{ProjectOptions, ProjectType, ScaffoldEngine, ScaffoldResult};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Project scaffolder
pub struct ProjectScaffolder<'a> {
    engine: &'a ScaffoldEngine,
}

impl<'a> ProjectScaffolder<'a> {
    pub fn new(engine: &'a ScaffoldEngine) -> Self {
        Self { engine }
    }

    /// Create a new project
    pub async fn create(&self, options: &ProjectOptions<'_>) -> ScaffoldResult<PathBuf> {
        let project_path = self.engine.base_path().join(options.name);

        // Create project structure
        self.create_directory_structure(&project_path, options)
            .await?;

        // Create Cargo.toml
        self.create_cargo_toml(&project_path, options).await?;

        // Create main.rs or lib.rs
        self.create_entry_point(&project_path, options).await?;

        // Create .env file
        self.create_env_file(&project_path, options).await?;

        // Create README.md
        self.create_readme(&project_path, options).await?;

        // Create .gitignore
        self.create_gitignore(&project_path).await?;

        Ok(project_path)
    }

    async fn create_directory_structure(
        &self,
        project_path: &Path,
        options: &ProjectOptions<'_>,
    ) -> ScaffoldResult<()> {
        // Base directories
        fs::create_dir_all(project_path.join("src")).await?;

        match options.project_type {
            ProjectType::Api | ProjectType::FullStack => {
                fs::create_dir_all(project_path.join("src/controllers")).await?;
                fs::create_dir_all(project_path.join("src/models")).await?;
                fs::create_dir_all(project_path.join("src/services")).await?;
                fs::create_dir_all(project_path.join("src/routes")).await?;
            }
            ProjectType::Microservice => {
                fs::create_dir_all(project_path.join("src/handlers")).await?;
                fs::create_dir_all(project_path.join("src/services")).await?;
            }
            ProjectType::Cli => {
                fs::create_dir_all(project_path.join("src/commands")).await?;
            }
        }

        if options.with_database {
            fs::create_dir_all(project_path.join("migrations")).await?;
        }

        if options.with_auth {
            fs::create_dir_all(project_path.join("src/auth")).await?;
        }

        fs::create_dir_all(project_path.join("tests")).await?;

        Ok(())
    }

    async fn create_cargo_toml(
        &self,
        project_path: &Path,
        options: &ProjectOptions<'_>,
    ) -> ScaffoldResult<()> {
        let data = CargoTomlData {
            name: options.name.to_string(),
            project_type: match options.project_type {
                ProjectType::Api => "api",
                ProjectType::FullStack => "full-stack",
                ProjectType::Microservice => "microservice",
                ProjectType::Cli => "cli",
            }
            .to_string(),
            with_database: options.with_database,
            with_auth: options.with_auth,
        };

        let content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = {{ version = "1.37", features = ["macros", "rt-multi-thread"] }}
anyhow = "1.0"
{}{}{}
[dev-dependencies]
tokio = {{ version = "1.37", features = ["macros", "rt-multi-thread"] }}
"#,
            data.name,
            if matches!(
                options.project_type,
                ProjectType::Api | ProjectType::FullStack | ProjectType::Microservice
            ) {
                r#"axum = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#
            } else if options.project_type == ProjectType::Cli {
                r#"clap = { version = "4.5", features = ["derive"] }
"#
            } else {
                ""
            },
            if options.with_database {
                r#"sea-orm = { version = "0.12", features = ["runtime-tokio-rustls", "sqlx-sqlite", "macros"] }
"#
            } else {
                ""
            },
            if options.with_auth {
                r#"jsonwebtoken = "9.2"
argon2 = "0.5"
"#
            } else {
                ""
            }
        );

        let path = project_path.join("Cargo.toml");
        fs::write(&path, content).await?;

        Ok(())
    }

    async fn create_entry_point(
        &self,
        project_path: &Path,
        options: &ProjectOptions<'_>,
    ) -> ScaffoldResult<()> {
        let content = match options.project_type {
            ProjectType::Api | ProjectType::FullStack => API_MAIN_TEMPLATE,
            ProjectType::Microservice => MICROSERVICE_MAIN_TEMPLATE,
            ProjectType::Cli => CLI_MAIN_TEMPLATE,
        };

        let path = project_path.join("src/main.rs");
        fs::write(&path, content).await?;

        Ok(())
    }

    async fn create_env_file(
        &self,
        project_path: &Path,
        options: &ProjectOptions<'_>,
    ) -> ScaffoldResult<()> {
        let mut content = String::from("# Application Configuration\n");
        content.push_str("APP_NAME=my-app\n");
        content.push_str("APP_ENV=development\n");
        content.push_str("HOST=127.0.0.1\n");
        content.push_str("PORT=3000\n\n");

        if options.with_database {
            content.push_str("# Database Configuration\n");
            content.push_str("DATABASE_URL=sqlite://database.db\n\n");
        }

        if options.with_auth {
            content.push_str("# Authentication\n");
            content.push_str("JWT_SECRET=your-secret-key-here\n");
            content.push_str("JWT_EXPIRY=3600\n");
        }

        let path = project_path.join(".env");
        fs::write(&path, content).await?;

        Ok(())
    }

    async fn create_readme(
        &self,
        project_path: &Path,
        options: &ProjectOptions<'_>,
    ) -> ScaffoldResult<()> {
        let content = format!(
            r#"# {}

A RustForge {} project.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo

### Installation

```bash
cargo build
```

### Running

```bash
cargo run
```

### Testing

```bash
cargo test
```

## Project Structure

```
{}
├── src/
│   ├── main.rs
{}{}└── Cargo.toml
```

## License

MIT OR Apache-2.0
"#,
            options.name,
            match options.project_type {
                ProjectType::Api => "API",
                ProjectType::FullStack => "full-stack web",
                ProjectType::Microservice => "microservice",
                ProjectType::Cli => "CLI",
            },
            options.name,
            if options.with_database {
                "│   ├── models/\n│   ├── migrations/\n"
            } else {
                ""
            },
            if options.with_auth {
                "│   ├── auth/\n"
            } else {
                ""
            }
        );

        let path = project_path.join("README.md");
        fs::write(&path, content).await?;

        Ok(())
    }

    async fn create_gitignore(&self, project_path: &Path) -> ScaffoldResult<()> {
        let content = r#"/target
/Cargo.lock
.env
*.db
*.db-shm
*.db-wal
.DS_Store
"#;

        let path = project_path.join(".gitignore");
        fs::write(&path, content).await?;

        Ok(())
    }
}

#[derive(Serialize)]
struct CargoTomlData {
    name: String,
    project_type: String,
    with_database: bool,
    with_auth: bool,
}

const API_MAIN_TEMPLATE: &str = r#"use axum::{
    routing::get,
    Router,
    Json,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await?;

    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Welcome to RustForge API"
    }))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok"
    }))
}
"#;

const MICROSERVICE_MAIN_TEMPLATE: &str = r#"use axum::{
    routing::get,
    Router,
    Json,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await?;

    println!("Microservice running on http://127.0.0.1:3000");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy"
    }))
}

async fn metrics() -> Json<serde_json::Value> {
    Json(json!({
        "requests": 0,
        "uptime": 0
    }))
}
"#;

const CLI_MAIN_TEMPLATE: &str = r#"use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mycli")]
#[command(about = "A RustForge CLI application", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the application
    Run {
        /// Optional argument
        #[arg(short, long)]
        verbose: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { verbose } => {
            if verbose {
                println!("Running in verbose mode...");
            }
            println!("Hello from RustForge CLI!");
        }
    }

    Ok(())
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScaffoldEngine;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_api_project() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();
        let scaffolder = ProjectScaffolder::new(&engine);

        let options = ProjectOptions {
            name: "test-api",
            project_type: ProjectType::Api,
            with_auth: false,
            with_database: true,
        };

        let result = scaffolder.create(&options).await;
        assert!(result.is_ok());

        let project_path = result.unwrap();
        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
        assert!(project_path.join(".env").exists());
        assert!(project_path.join("README.md").exists());
    }

    #[tokio::test]
    async fn test_create_cli_project() {
        let dir = tempdir().unwrap();
        let engine = ScaffoldEngine::new(dir.path()).unwrap();
        let scaffolder = ProjectScaffolder::new(&engine);

        let options = ProjectOptions {
            name: "test-cli",
            project_type: ProjectType::Cli,
            with_auth: false,
            with_database: false,
        };

        let result = scaffolder.create(&options).await;
        assert!(result.is_ok());

        let project_path = result.unwrap();
        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
    }
}
