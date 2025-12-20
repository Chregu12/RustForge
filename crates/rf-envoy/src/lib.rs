//! SSH Deployment Task Runner for RustForge
//!
//! This crate provides Laravel Envoy-like functionality for running deployment
//! tasks on remote servers via SSH.
//!
//! # Features
//!
//! - **SSH Task Execution**: Run commands on remote servers
//! - **Task Definition**: Define reusable deployment tasks
//! - **Stories**: Group tasks into deployment workflows
//! - **Parallel Execution**: Run tasks on multiple servers simultaneously
//! - **Templates**: Blade-like variable substitution in scripts
//! - **Notifications**: Slack/Discord notifications on task completion
//!
//! # Quick Start
//!
//! ```ignore
//! use rf_envoy::{Envoy, Task, Server};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rf_envoy::EnvoyError> {
//!     let envoy = Envoy::new()
//!         .server("production", Server::new("deploy@example.com"))
//!         .task("deploy", |t| {
//!             t.on("production")
//!                 .run("cd /var/www/app && git pull origin main")
//!                 .run("cargo build --release")
//!                 .run("systemctl restart myapp")
//!         });
//!
//!     envoy.run("deploy").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Task File Format (Envoy.toml)
//!
//! ```toml
//! [servers]
//! production = "deploy@example.com"
//! staging = "deploy@staging.example.com"
//!
//! [tasks.deploy]
//! servers = ["production"]
//! commands = [
//!     "cd /var/www/app",
//!     "git pull origin main",
//!     "cargo build --release",
//!     "systemctl restart myapp"
//! ]
//!
//! [stories.full-deploy]
//! tasks = ["pull", "build", "restart"]
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

pub mod runner;
pub mod server;
pub mod ssh;
pub mod task;
pub mod notification;
pub mod template;

pub use runner::TaskRunner;
pub use server::{Server, ServerConfig};
pub use task::{Task, TaskBuilder, TaskDefinition};
pub use notification::{NotificationChannel, Notifier};
pub use template::TemplateEngine;

/// Envoy error types
#[derive(Debug, Error)]
pub enum EnvoyError {
    #[error("SSH connection error: {0}")]
    ConnectionError(String),

    #[error("Task execution error: {0}")]
    ExecutionError(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Notification error: {0}")]
    NotificationError(String),
}

pub type EnvoyResult<T> = Result<T, EnvoyError>;

/// Task execution result
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Task name
    pub task: String,
    /// Server that ran the task
    pub server: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: std::time::Duration,
}

/// Main Envoy instance
pub struct Envoy {
    servers: HashMap<String, Server>,
    tasks: HashMap<String, TaskDefinition>,
    stories: HashMap<String, Vec<String>>,
    variables: HashMap<String, String>,
    notifiers: Vec<Arc<dyn Notifier + Send + Sync>>,
    before_hooks: Vec<String>,
    after_hooks: Vec<String>,
}

impl Envoy {
    /// Create a new Envoy instance
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            tasks: HashMap::new(),
            stories: HashMap::new(),
            variables: HashMap::new(),
            notifiers: Vec::new(),
            before_hooks: Vec::new(),
            after_hooks: Vec::new(),
        }
    }

    /// Load configuration from a file
    pub fn from_file(path: impl Into<PathBuf>) -> EnvoyResult<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)?;

        let config: EnvoyConfig = toml::from_str(&content)
            .map_err(|e| EnvoyError::ConfigError(e.to_string()))?;

        let mut envoy = Self::new();

        // Load servers
        for (name, host) in config.servers {
            envoy.servers.insert(name, Server::new(host));
        }

        // Load tasks
        for (name, task_config) in config.tasks {
            let mut task = TaskDefinition::new(&name);
            task.servers = task_config.servers;
            task.commands = task_config.commands;
            task.parallel = task_config.parallel.unwrap_or(false);
            envoy.tasks.insert(name, task);
        }

        // Load stories
        if let Some(stories) = config.stories {
            for (name, story_config) in stories {
                envoy.stories.insert(name, story_config.tasks);
            }
        }

        Ok(envoy)
    }

    /// Add a server
    pub fn server(mut self, name: impl Into<String>, server: Server) -> Self {
        self.servers.insert(name.into(), server);
        self
    }

    /// Define a task
    pub fn task<F>(mut self, name: impl Into<String>, builder: F) -> Self
    where
        F: FnOnce(TaskBuilder) -> TaskBuilder,
    {
        let name = name.into();
        let task_builder = TaskBuilder::new(&name);
        let task = builder(task_builder).build();
        self.tasks.insert(name, task);
        self
    }

    /// Define a story (sequence of tasks)
    pub fn story(mut self, name: impl Into<String>, tasks: Vec<&str>) -> Self {
        self.stories
            .insert(name.into(), tasks.into_iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set a variable
    pub fn variable(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Add a notifier
    pub fn notify(mut self, notifier: Arc<dyn Notifier + Send + Sync>) -> Self {
        self.notifiers.push(notifier);
        self
    }

    /// Add a before hook
    pub fn before(mut self, task: impl Into<String>) -> Self {
        self.before_hooks.push(task.into());
        self
    }

    /// Add an after hook
    pub fn after(mut self, task: impl Into<String>) -> Self {
        self.after_hooks.push(task.into());
        self
    }

    /// Run a task
    pub async fn run(&self, task_name: &str) -> EnvoyResult<Vec<TaskResult>> {
        // Check if it's a story
        if let Some(story_tasks) = self.stories.get(task_name) {
            let mut results = Vec::new();
            for task in story_tasks {
                let task_results = self.run_task(task).await?;
                results.extend(task_results);
            }
            return Ok(results);
        }

        self.run_task(task_name).await
    }

    /// Run a single task
    async fn run_task(&self, task_name: &str) -> EnvoyResult<Vec<TaskResult>> {
        let task = self
            .tasks
            .get(task_name)
            .ok_or_else(|| EnvoyError::TaskNotFound(task_name.to_string()))?;

        // Run before hooks
        for hook in &self.before_hooks {
            if let Some(hook_task) = self.tasks.get(hook) {
                self.execute_task(hook_task).await?;
            }
        }

        // Notify start
        for notifier in &self.notifiers {
            let _ = notifier.on_start(task_name).await;
        }

        // Execute main task
        let results = self.execute_task(task).await?;

        // Check success
        let all_success = results.iter().all(|r| r.success);

        // Notify completion
        for notifier in &self.notifiers {
            if all_success {
                let _ = notifier.on_success(task_name, &results).await;
            } else {
                let _ = notifier.on_failure(task_name, &results).await;
            }
        }

        // Run after hooks
        for hook in &self.after_hooks {
            if let Some(hook_task) = self.tasks.get(hook) {
                self.execute_task(hook_task).await?;
            }
        }

        Ok(results)
    }

    /// Execute a task on its servers
    async fn execute_task(&self, task: &TaskDefinition) -> EnvoyResult<Vec<TaskResult>> {
        let mut results = Vec::new();
        let template_engine = TemplateEngine::new(self.variables.clone());

        if task.parallel {
            // Run on all servers in parallel
            let mut handles = Vec::new();

            for server_name in &task.servers {
                let server = self
                    .servers
                    .get(server_name)
                    .ok_or_else(|| EnvoyError::ServerNotFound(server_name.clone()))?
                    .clone();

                let task_name = task.name.clone();
                let commands: Vec<String> = task
                    .commands
                    .iter()
                    .map(|cmd| template_engine.render(cmd))
                    .collect::<Result<Vec<_>, _>>()?;
                let server_name_clone = server_name.clone();

                handles.push(tokio::spawn(async move {
                    let runner = TaskRunner::new(server);
                    runner.run(&task_name, &commands, &server_name_clone).await
                }));
            }

            for handle in handles {
                match handle.await {
                    Ok(result) => results.push(result?),
                    Err(e) => {
                        return Err(EnvoyError::ExecutionError(e.to_string()));
                    }
                }
            }
        } else {
            // Run on servers sequentially
            for server_name in &task.servers {
                let server = self
                    .servers
                    .get(server_name)
                    .ok_or_else(|| EnvoyError::ServerNotFound(server_name.clone()))?
                    .clone();

                let commands: Vec<String> = task
                    .commands
                    .iter()
                    .map(|cmd| template_engine.render(cmd))
                    .collect::<Result<Vec<_>, _>>()?;

                let runner = TaskRunner::new(server);
                let result = runner.run(&task.name, &commands, server_name).await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// List all available tasks
    pub fn list_tasks(&self) -> Vec<&str> {
        self.tasks.keys().map(|s| s.as_str()).collect()
    }

    /// List all available stories
    pub fn list_stories(&self) -> Vec<&str> {
        self.stories.keys().map(|s| s.as_str()).collect()
    }

    /// List all servers
    pub fn list_servers(&self) -> Vec<&str> {
        self.servers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for Envoy {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration file structure
#[derive(Debug, Deserialize)]
struct EnvoyConfig {
    servers: HashMap<String, String>,
    tasks: HashMap<String, TaskConfig>,
    stories: Option<HashMap<String, StoryConfig>>,
}

#[derive(Debug, Deserialize)]
struct TaskConfig {
    servers: Vec<String>,
    commands: Vec<String>,
    parallel: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StoryConfig {
    tasks: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envoy_creation() {
        let envoy = Envoy::new()
            .server("production", Server::new("deploy@example.com"))
            .variable("branch", "main");

        assert_eq!(envoy.servers.len(), 1);
        assert_eq!(envoy.variables.get("branch"), Some(&"main".to_string()));
    }

    #[test]
    fn test_task_definition() {
        let envoy = Envoy::new()
            .server("production", Server::new("deploy@example.com"))
            .task("deploy", |t| {
                t.on("production")
                    .run("git pull")
                    .run("cargo build --release")
            });

        assert!(envoy.tasks.contains_key("deploy"));
    }

    #[test]
    fn test_story_definition() {
        let envoy = Envoy::new()
            .story("full-deploy", vec!["pull", "build", "restart"]);

        assert!(envoy.stories.contains_key("full-deploy"));
        assert_eq!(envoy.stories.get("full-deploy").unwrap().len(), 3);
    }
}
