//! Docker Development Environment for RustForge
//!
//! This crate provides Laravel Sail-like functionality for managing Docker
//! development environments for Rust applications.
//!
//! # Features
//!
//! - **Docker Compose Management**: Start, stop, and manage services
//! - **Service Templates**: Pre-configured services (Postgres, Redis, etc.)
//! - **Container Execution**: Run commands inside containers
//! - **File Watching**: Auto-rebuild on file changes
//! - **Service Health Checks**: Wait for services to be ready
//! - **Environment Configuration**: Generate docker-compose.yml from config
//!
//! # Quick Start
//!
//! ```ignore
//! use rf_sail::{Sail, SailConfig, Service};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rf_sail::SailError> {
//!     let sail = Sail::new(SailConfig::default())
//!         .with_service(Service::Postgres)
//!         .with_service(Service::Redis)
//!         .with_service(Service::Mailhog);
//!
//!     // Start all services
//!     sail.up().await?;
//!
//!     // Run a command in the app container
//!     sail.exec("cargo test").await?;
//!
//!     // Stop all services
//!     sail.down().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration (sail.toml)
//!
//! ```toml
//! [app]
//! name = "my-app"
//! port = 8000
//!
//! [services]
//! postgres = true
//! redis = true
//! mailhog = true
//!
//! [build]
//! dockerfile = "Dockerfile.dev"
//! target = "development"
//! ```

use async_trait::async_trait;
use bollard::Docker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

pub mod compose;
pub mod container;
pub mod services;
pub mod watcher;

pub use compose::{ComposeConfig, ComposeGenerator};
pub use container::ContainerManager;
pub use services::Service;
pub use watcher::FileWatcher;

/// Sail error types
#[derive(Debug, Error)]
pub enum SailError {
    #[error("Docker error: {0}")]
    DockerError(String),

    #[error("Container error: {0}")]
    ContainerError(String),

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Command error: {0}")]
    CommandError(String),
}

pub type SailResult<T> = Result<T, SailError>;

/// Sail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SailConfig {
    /// Application name
    pub app_name: String,
    /// Application port
    pub app_port: u16,
    /// Project directory
    pub project_dir: PathBuf,
    /// Docker Compose file path
    pub compose_file: PathBuf,
    /// Dockerfile for the app
    pub dockerfile: String,
    /// Build target (e.g., "development", "production")
    pub build_target: Option<String>,
    /// Extra environment variables
    pub environment: HashMap<String, String>,
    /// Services to enable
    pub services: Vec<Service>,
}

impl Default for SailConfig {
    fn default() -> Self {
        Self {
            app_name: "rustforge-app".to_string(),
            app_port: 8000,
            project_dir: PathBuf::from("."),
            compose_file: PathBuf::from("docker-compose.yml"),
            dockerfile: "Dockerfile".to_string(),
            build_target: Some("development".to_string()),
            environment: HashMap::new(),
            services: vec![Service::Postgres, Service::Redis],
        }
    }
}

impl SailConfig {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            ..Default::default()
        }
    }

    /// Load configuration from a file
    pub fn from_file(path: impl Into<PathBuf>) -> SailResult<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;

        toml::from_str(&content).map_err(|e| SailError::ConfigError(e.to_string()))
    }

    pub fn app_port(mut self, port: u16) -> Self {
        self.app_port = port;
        self
    }

    pub fn dockerfile(mut self, dockerfile: impl Into<String>) -> Self {
        self.dockerfile = dockerfile.into();
        self
    }

    pub fn build_target(mut self, target: impl Into<String>) -> Self {
        self.build_target = Some(target.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }
}

/// Main Sail instance
pub struct Sail {
    config: SailConfig,
    docker: Option<Docker>,
    container_manager: Arc<RwLock<Option<ContainerManager>>>,
}

impl Sail {
    /// Create a new Sail instance
    pub fn new(config: SailConfig) -> Self {
        Self {
            config,
            docker: None,
            container_manager: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a service
    pub fn with_service(mut self, service: Service) -> Self {
        if !self.config.services.contains(&service) {
            self.config.services.push(service);
        }
        self
    }

    /// Connect to Docker
    pub async fn connect(&mut self) -> SailResult<()> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| SailError::DockerError(e.to_string()))?;

        // Verify connection
        docker
            .ping()
            .await
            .map_err(|e| SailError::DockerError(e.to_string()))?;

        self.docker = Some(docker.clone());

        let mut manager = self.container_manager.write().await;
        *manager = Some(ContainerManager::new(docker));

        Ok(())
    }

    /// Generate docker-compose.yml
    pub fn generate_compose(&self) -> SailResult<String> {
        let generator = ComposeGenerator::new(&self.config);
        generator.generate()
    }

    /// Write docker-compose.yml to file
    pub fn write_compose(&self) -> SailResult<()> {
        let content = self.generate_compose()?;
        std::fs::write(&self.config.compose_file, content)?;
        Ok(())
    }

    /// Start all services
    pub async fn up(&self) -> SailResult<()> {
        self.docker_compose(&["up", "-d"]).await
    }

    /// Start services and build
    pub async fn up_build(&self) -> SailResult<()> {
        self.docker_compose(&["up", "-d", "--build"]).await
    }

    /// Stop all services
    pub async fn down(&self) -> SailResult<()> {
        self.docker_compose(&["down"]).await
    }

    /// Stop and remove volumes
    pub async fn down_volumes(&self) -> SailResult<()> {
        self.docker_compose(&["down", "-v"]).await
    }

    /// Restart services
    pub async fn restart(&self) -> SailResult<()> {
        self.docker_compose(&["restart"]).await
    }

    /// Build containers
    pub async fn build(&self) -> SailResult<()> {
        self.docker_compose(&["build"]).await
    }

    /// Pull latest images
    pub async fn pull(&self) -> SailResult<()> {
        self.docker_compose(&["pull"]).await
    }

    /// Show logs
    pub async fn logs(&self, service: Option<&str>, follow: bool) -> SailResult<()> {
        let mut args = vec!["logs"];
        if follow {
            args.push("-f");
        }
        if let Some(svc) = service {
            args.push(svc);
        }
        self.docker_compose(&args).await
    }

    /// Show service status
    pub async fn ps(&self) -> SailResult<String> {
        self.docker_compose_output(&["ps"]).await
    }

    /// Execute a command in the app container
    pub async fn exec(&self, command: &str) -> SailResult<String> {
        let args = ["exec", "-T", &self.config.app_name, "bash", "-c", command];
        self.docker_compose_output(&args).await
    }

    /// Run a one-off command
    pub async fn run(&self, command: &str) -> SailResult<String> {
        let args = ["run", "--rm", &self.config.app_name, "bash", "-c", command];
        self.docker_compose_output(&args).await
    }

    /// Run cargo command
    pub async fn cargo(&self, args: &str) -> SailResult<String> {
        self.exec(&format!("cargo {}", args)).await
    }

    /// Run cargo test
    pub async fn test(&self) -> SailResult<String> {
        self.cargo("test").await
    }

    /// Run cargo build
    pub async fn cargo_build(&self, release: bool) -> SailResult<String> {
        let cmd = if release {
            "build --release"
        } else {
            "build"
        };
        self.cargo(cmd).await
    }

    /// Open a shell in the app container
    pub async fn shell(&self) -> SailResult<()> {
        let args = ["exec", &self.config.app_name, "bash"];
        self.docker_compose_interactive(&args).await
    }

    /// Run database migrations
    pub async fn migrate(&self) -> SailResult<String> {
        self.exec("cargo run --release -- migrate").await
    }

    /// Open database shell
    pub async fn db_shell(&self) -> SailResult<()> {
        if self.config.services.contains(&Service::Postgres) {
            self.docker_compose_interactive(&["exec", "postgres", "psql", "-U", "postgres"])
                .await
        } else if self.config.services.contains(&Service::Mysql) {
            self.docker_compose_interactive(&["exec", "mysql", "mysql", "-u", "root", "-p"])
                .await
        } else {
            Err(SailError::ServiceError("No database service configured".to_string()))
        }
    }

    /// Open Redis CLI
    pub async fn redis_cli(&self) -> SailResult<()> {
        if self.config.services.contains(&Service::Redis) {
            self.docker_compose_interactive(&["exec", "redis", "redis-cli"])
                .await
        } else {
            Err(SailError::ServiceError("Redis not configured".to_string()))
        }
    }

    /// Wait for all services to be healthy
    pub async fn wait_for_services(&self) -> SailResult<()> {
        use indicatif::{ProgressBar, ProgressStyle};
        use std::time::Duration;

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );

        for service in &self.config.services {
            pb.set_message(format!("Waiting for {}...", service.name()));

            let max_attempts = 30;
            for attempt in 0..max_attempts {
                if self.check_service_health(service).await? {
                    break;
                }

                if attempt == max_attempts - 1 {
                    pb.finish_with_message(format!("❌ {} failed to start", service.name()));
                    return Err(SailError::ServiceError(format!(
                        "{} did not become healthy",
                        service.name()
                    )));
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            pb.println(format!("✅ {} is ready", service.name()));
        }

        pb.finish_with_message("All services are ready!");
        Ok(())
    }

    /// Check if a service is healthy
    async fn check_service_health(&self, service: &Service) -> SailResult<bool> {
        let check_cmd = match service {
            Service::Postgres => "pg_isready -U postgres",
            Service::Mysql => "mysqladmin ping -h localhost -u root --password=",
            Service::Redis => "redis-cli ping",
            Service::Mongodb => "mongosh --eval 'db.runCommand({ ping: 1 })'",
            Service::Meilisearch => "curl -s http://localhost:7700/health",
            _ => return Ok(true), // Assume other services are ready
        };

        let result = self
            .docker_compose_output(&["exec", "-T", service.name(), "sh", "-c", check_cmd])
            .await;

        Ok(result.is_ok())
    }

    /// Run docker-compose command
    async fn docker_compose(&self, args: &[&str]) -> SailResult<()> {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("compose");
        cmd.arg("-f").arg(&self.config.compose_file);
        cmd.args(args);
        cmd.current_dir(&self.config.project_dir);

        let status = cmd
            .status()
            .await
            .map_err(|e| SailError::CommandError(e.to_string()))?;

        if !status.success() {
            return Err(SailError::CommandError(format!(
                "docker compose {} failed",
                args.join(" ")
            )));
        }

        Ok(())
    }

    /// Run docker-compose and capture output
    async fn docker_compose_output(&self, args: &[&str]) -> SailResult<String> {
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("compose");
        cmd.arg("-f").arg(&self.config.compose_file);
        cmd.args(args);
        cmd.current_dir(&self.config.project_dir);

        let output = cmd
            .output()
            .await
            .map_err(|e| SailError::CommandError(e.to_string()))?;

        if !output.status.success() {
            return Err(SailError::CommandError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run interactive docker-compose command
    async fn docker_compose_interactive(&self, args: &[&str]) -> SailResult<()> {
        let mut cmd = std::process::Command::new("docker");
        cmd.arg("compose");
        cmd.arg("-f").arg(&self.config.compose_file);
        cmd.args(args);
        cmd.current_dir(&self.config.project_dir);

        let status = cmd
            .status()
            .map_err(|e| SailError::CommandError(e.to_string()))?;

        if !status.success() {
            return Err(SailError::CommandError("Interactive command failed".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sail_config_default() {
        let config = SailConfig::default();
        assert_eq!(config.app_port, 8000);
        assert!(!config.services.is_empty());
    }

    #[test]
    fn test_sail_with_services() {
        let sail = Sail::new(SailConfig::default())
            .with_service(Service::Postgres)
            .with_service(Service::Redis)
            .with_service(Service::Mailhog);

        assert!(sail.config.services.contains(&Service::Mailhog));
    }

    #[test]
    fn test_generate_compose() {
        let sail = Sail::new(SailConfig::default());
        let compose = sail.generate_compose().unwrap();

        assert!(compose.contains("version:"));
        assert!(compose.contains("services:"));
    }
}
