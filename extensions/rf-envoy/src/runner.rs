//! Task execution runner

use crate::{EnvoyError, EnvoyResult, Server, TaskResult};
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

/// Task runner executes commands on servers
pub struct TaskRunner {
    server: Server,
}

impl TaskRunner {
    pub fn new(server: Server) -> Self {
        Self { server }
    }

    /// Run commands on the server
    pub async fn run(
        &self,
        task_name: &str,
        commands: &[String],
        server_name: &str,
    ) -> EnvoyResult<TaskResult> {
        let start = Instant::now();

        // Build the SSH command
        let script = self.build_script(commands);
        let result = self.execute_ssh(&script).await?;

        Ok(TaskResult {
            task: task_name.to_string(),
            server: server_name.to_string(),
            success: result.0 == 0,
            exit_code: result.0,
            stdout: result.1,
            stderr: result.2,
            duration: start.elapsed(),
        })
    }

    /// Build a bash script from commands
    fn build_script(&self, commands: &[String]) -> String {
        let mut script = String::from("set -e\n");

        // Add working directory change if configured (quote path to prevent injection)
        if let Some(ref dir) = self.server.working_dir {
            let escaped_dir = dir.replace('\'', "'\\''");
            script.push_str(&format!("cd '{}'\n", escaped_dir));
        }

        // Add each command
        for cmd in commands {
            script.push_str(cmd);
            script.push('\n');
        }

        script
    }

    /// Execute script via SSH
    async fn execute_ssh(&self, script: &str) -> EnvoyResult<(i32, String, String)> {
        let mut cmd = Command::new("ssh");

        // Add SSH options
        cmd.arg("-o").arg("BatchMode=yes");
        cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");
        cmd.arg("-p").arg(self.server.port.to_string());

        // Add identity file if specified
        if let Some(ref key) = self.server.identity_file {
            cmd.arg("-i").arg(key);
        }

        // Add jump host if specified
        if let Some(ref jump) = self.server.jump_host {
            cmd.arg("-J").arg(jump);
        }

        // Add host
        cmd.arg(self.server.ssh_host());

        // Add the script to execute
        cmd.arg("bash").arg("-s");

        // Set up stdin for script
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| EnvoyError::ConnectionError(e.to_string()))?;

        // Write script to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(script.as_bytes())
                .await
                .map_err(|e| EnvoyError::ExecutionError(e.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| EnvoyError::ExecutionError(e.to_string()))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((exit_code, stdout, stderr))
    }

    /// Run a single command and return output
    pub async fn run_command(&self, command: &str) -> EnvoyResult<String> {
        let result = self.execute_ssh(command).await?;

        if result.0 != 0 {
            return Err(EnvoyError::ExecutionError(format!(
                "Command failed with exit code {}: {}",
                result.0, result.2
            )));
        }

        Ok(result.1)
    }

    /// Test SSH connection
    pub async fn test_connection(&self) -> EnvoyResult<bool> {
        match self.run_command("echo 'connected'").await {
            Ok(output) => Ok(output.trim() == "connected"),
            Err(_) => Ok(false),
        }
    }
}

/// Execute a local command
pub async fn run_local(commands: &[String]) -> EnvoyResult<TaskResult> {
    let start = Instant::now();
    let script = commands.join(" && ");

    let output = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .output()
        .await
        .map_err(|e| EnvoyError::ExecutionError(e.to_string()))?;

    let exit_code = output.status.code().unwrap_or(-1);

    Ok(TaskResult {
        task: "local".to_string(),
        server: "localhost".to_string(),
        success: exit_code == 0,
        exit_code,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration: start.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_script() {
        let server = Server::new("example.com").working_dir("/var/www");
        let runner = TaskRunner::new(server);

        let script = runner.build_script(&["git pull".to_string(), "cargo build".to_string()]);

        assert!(script.contains("set -e"));
        assert!(script.contains("cd '/var/www'"));
        assert!(script.contains("git pull"));
        assert!(script.contains("cargo build"));
    }

    #[tokio::test]
    async fn test_run_local() {
        let result = run_local(&["echo 'hello'".to_string()]).await.unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("hello"));
    }
}
