#![allow(dead_code)] // fields/methods retained for planned functionality, not read internally yet
//! Task definitions

use serde::{Deserialize, Serialize};

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    /// Task name
    pub name: String,
    /// Servers to run on
    pub servers: Vec<String>,
    /// Commands to execute
    pub commands: Vec<String>,
    /// Run on servers in parallel
    pub parallel: bool,
    /// Confirm before running
    pub confirm: bool,
    /// Only run once (on first server)
    pub once: bool,
    /// Hidden from list
    pub hidden: bool,
}

impl TaskDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            servers: Vec::new(),
            commands: Vec::new(),
            parallel: false,
            confirm: false,
            once: false,
            hidden: false,
        }
    }
}

/// Fluent task builder
pub struct TaskBuilder {
    task: TaskDefinition,
}

impl TaskBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            task: TaskDefinition::new(name),
        }
    }

    /// Set the server(s) to run on
    pub fn on(mut self, server: impl Into<String>) -> Self {
        self.task.servers.push(server.into());
        self
    }

    /// Add multiple servers
    pub fn on_servers(mut self, servers: Vec<&str>) -> Self {
        self.task.servers.extend(servers.into_iter().map(|s| s.to_string()));
        self
    }

    /// Add a command to execute
    pub fn run(mut self, command: impl Into<String>) -> Self {
        self.task.commands.push(command.into());
        self
    }

    /// Add multiple commands
    pub fn run_all(mut self, commands: Vec<&str>) -> Self {
        self.task.commands.extend(commands.into_iter().map(|s| s.to_string()));
        self
    }

    /// Run a script file
    pub fn script(mut self, path: impl Into<String>) -> Self {
        self.task.commands.push(format!("bash {}", path.into()));
        self
    }

    /// Execute in parallel on all servers
    pub fn parallel(mut self) -> Self {
        self.task.parallel = true;
        self
    }

    /// Require confirmation before running
    pub fn confirm(mut self) -> Self {
        self.task.confirm = true;
        self
    }

    /// Only run on the first server
    pub fn once(mut self) -> Self {
        self.task.once = true;
        self
    }

    /// Hide from task list
    pub fn hidden(mut self) -> Self {
        self.task.hidden = true;
        self
    }

    /// Build the task definition
    pub fn build(self) -> TaskDefinition {
        self.task
    }
}

/// A higher-level task abstraction
pub struct Task {
    name: String,
    description: Option<String>,
    commands: Vec<String>,
}

impl Task {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            commands: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn run(mut self, command: impl Into<String>) -> Self {
        self.commands.push(command.into());
        self
    }
}

/// Common deployment tasks
pub mod presets {
    use super::TaskBuilder;

    /// Git pull task
    pub fn git_pull(branch: &str) -> TaskBuilder {
        TaskBuilder::new("pull")
            .run(format!("git pull origin {}", branch))
    }

    /// Cargo build task
    pub fn cargo_build(release: bool) -> TaskBuilder {
        let cmd = if release {
            "cargo build --release"
        } else {
            "cargo build"
        };
        TaskBuilder::new("build").run(cmd)
    }

    /// Run migrations
    pub fn migrate() -> TaskBuilder {
        TaskBuilder::new("migrate")
            .run("cargo run --release -- migrate")
    }

    /// Clear caches
    pub fn clear_cache() -> TaskBuilder {
        TaskBuilder::new("clear-cache")
            .run("cargo run --release -- cache:clear")
    }

    /// Restart application (systemd)
    pub fn restart_systemd(service: &str) -> TaskBuilder {
        TaskBuilder::new("restart")
            .run(format!("sudo systemctl restart '{}'", service.replace('\'', "'\\''")))
    }

    /// Check application status
    pub fn status_systemd(service: &str) -> TaskBuilder {
        TaskBuilder::new("status")
            .run(format!("sudo systemctl status '{}'", service.replace('\'', "'\\''")))
    }

    /// View logs
    pub fn logs_systemd(service: &str, lines: u32) -> TaskBuilder {
        TaskBuilder::new("logs")
            .run(format!("sudo journalctl -u '{}' -n {} --no-pager", service.replace('\'', "'\\''"), lines))
    }

    /// Zero-downtime deploy
    pub fn zero_downtime_deploy(app_dir: &str, service: &str, branch: &str) -> TaskBuilder {
        let escaped_dir = app_dir.replace('\'', "'\\''");
        let escaped_branch = branch.replace('\'', "'\\''");
        let escaped_service = service.replace('\'', "'\\''");
        TaskBuilder::new("deploy")
            .run(format!("cd '{}'", escaped_dir))
            .run(format!("git pull origin '{}'", escaped_branch))
            .run("cargo build --release")
            .run("cargo run --release -- migrate")
            .run(format!("sudo systemctl reload '{}'", escaped_service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_builder() {
        let task = TaskBuilder::new("deploy")
            .on("production")
            .run("git pull")
            .run("cargo build --release")
            .parallel()
            .build();

        assert_eq!(task.name, "deploy");
        assert_eq!(task.servers.len(), 1);
        assert_eq!(task.commands.len(), 2);
        assert!(task.parallel);
    }

    #[test]
    fn test_multiple_servers() {
        let task = TaskBuilder::new("deploy")
            .on("web1")
            .on("web2")
            .on("web3")
            .run("systemctl restart app")
            .parallel()
            .build();

        assert_eq!(task.servers.len(), 3);
    }

    #[test]
    fn test_preset_tasks() {
        let pull = presets::git_pull("main").build();
        assert!(pull.commands[0].contains("git pull"));

        let build = presets::cargo_build(true).build();
        assert!(build.commands[0].contains("--release"));
    }
}
