//! Container management using bollard

use crate::{SailError, SailResult};
use bollard::container::{LogsOptions, StartContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::Docker;
use futures::StreamExt;

/// Container manager
pub struct ContainerManager {
    docker: Docker,
}

impl ContainerManager {
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// List running containers
    pub async fn list(&self) -> SailResult<Vec<ContainerInfo>> {
        let containers = self
            .docker
            .list_containers::<String>(None)
            .await
            .map_err(|e| SailError::DockerError(e.to_string()))?;

        Ok(containers
            .into_iter()
            .map(|c| ContainerInfo {
                id: c.id.unwrap_or_default(),
                names: c.names.unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
            })
            .collect())
    }

    /// Get container by name
    pub async fn get(&self, name: &str) -> SailResult<Option<ContainerInfo>> {
        let containers = self.list().await?;
        Ok(containers.into_iter().find(|c| {
            c.names
                .iter()
                .any(|n| n.trim_start_matches('/') == name)
        }))
    }

    /// Start a container
    pub async fn start(&self, container_id: &str) -> SailResult<()> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))
    }

    /// Stop a container
    pub async fn stop(&self, container_id: &str) -> SailResult<()> {
        self.docker
            .stop_container(container_id, None)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))
    }

    /// Restart a container
    pub async fn restart(&self, container_id: &str) -> SailResult<()> {
        self.docker
            .restart_container(container_id, None)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))
    }

    /// Remove a container
    pub async fn remove(&self, container_id: &str) -> SailResult<()> {
        self.docker
            .remove_container(container_id, None)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))
    }

    /// Execute a command in a container
    pub async fn exec(&self, container_id: &str, cmd: &[&str]) -> SailResult<ExecResult> {
        let exec = self
            .docker
            .create_exec(
                container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))?;

        let result = self
            .docker
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } = result {
            while let Some(Ok(msg)) = output.next().await {
                match msg {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        // Get exit code
        let inspect = self
            .docker
            .inspect_exec(&exec.id)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))?;

        Ok(ExecResult {
            exit_code: inspect.exit_code.unwrap_or(-1) as i32,
            stdout,
            stderr,
        })
    }

    /// Get container logs
    pub async fn logs(&self, container_id: &str, lines: usize) -> SailResult<String> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: lines.to_string(),
            ..Default::default()
        };

        let mut logs = self.docker.logs(container_id, Some(options));
        let mut output = String::new();

        while let Some(Ok(log)) = logs.next().await {
            match log {
                bollard::container::LogOutput::StdOut { message } => {
                    output.push_str(&String::from_utf8_lossy(&message));
                }
                bollard::container::LogOutput::StdErr { message } => {
                    output.push_str(&String::from_utf8_lossy(&message));
                }
                _ => {}
            }
        }

        Ok(output)
    }

    /// Check if container is running
    pub async fn is_running(&self, container_id: &str) -> SailResult<bool> {
        let inspect = self
            .docker
            .inspect_container(container_id, None)
            .await
            .map_err(|e| SailError::ContainerError(e.to_string()))?;

        Ok(inspect
            .state
            .and_then(|s| s.running)
            .unwrap_or(false))
    }

    /// Wait for container to be ready
    pub async fn wait_for_healthy(
        &self,
        container_id: &str,
        timeout_secs: u64,
    ) -> SailResult<bool> {
        use std::time::Duration;

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            let inspect = self
                .docker
                .inspect_container(container_id, None)
                .await
                .map_err(|e| SailError::ContainerError(e.to_string()))?;

            if let Some(state) = inspect.state {
                if let Some(health) = state.health {
                    if health.status == Some(bollard::models::HealthStatusEnum::HEALTHY) {
                        return Ok(true);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(false)
    }
}

/// Container information
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_result() {
        let result = ExecResult {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };

        assert!(result.success());
    }
}
