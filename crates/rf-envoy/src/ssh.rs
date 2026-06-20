//! SSH connection handling using russh

use crate::{EnvoyError, EnvoyResult, Server};

/// SSH session wrapper
pub struct SshSession {
    server: Server,
    connected: bool,
}

impl SshSession {
    pub fn new(server: Server) -> Self {
        Self {
            server,
            connected: false,
        }
    }

    /// Connect to the server
    pub async fn connect(&mut self) -> EnvoyResult<()> {
        // Note: Full russh implementation would go here
        // For now, we use the CLI-based approach in TaskRunner
        self.connected = true;
        Ok(())
    }

    /// Execute a command
    pub async fn exec(&self, command: &str) -> EnvoyResult<SshOutput> {
        if !self.connected {
            return Err(EnvoyError::ConnectionError("Not connected".to_string()));
        }

        // Use tokio process for now
        let output = tokio::process::Command::new("ssh")
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-p")
            .arg(self.server.port.to_string())
            .arg(self.server.ssh_host())
            .arg(command)
            .output()
            .await
            .map_err(|e| EnvoyError::ExecutionError(e.to_string()))?;

        Ok(SshOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Close the connection
    pub async fn disconnect(&mut self) -> EnvoyResult<()> {
        self.connected = false;
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// SSH command output
#[derive(Debug, Clone)]
pub struct SshOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl SshOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// SSH key types
#[derive(Debug, Clone, Copy)]
pub enum KeyType {
    Rsa,
    Ed25519,
    Ecdsa,
}

/// Generate an SSH key pair
pub async fn generate_key_pair(key_type: KeyType, comment: &str) -> EnvoyResult<(String, String)> {
    let key_type_arg = match key_type {
        KeyType::Rsa => "rsa",
        KeyType::Ed25519 => "ed25519",
        KeyType::Ecdsa => "ecdsa",
    };

    let temp_dir = tempfile::tempdir()
        .map_err(|e| EnvoyError::IoError(std::io::Error::other(e)))?;
    let key_path = temp_dir.path().join("key");

    let output = tokio::process::Command::new("ssh-keygen")
        .arg("-t")
        .arg(key_type_arg)
        .arg("-C")
        .arg(comment)
        .arg("-f")
        .arg(&key_path)
        .arg("-N")
        .arg("")
        .output()
        .await
        .map_err(|e| EnvoyError::ExecutionError(e.to_string()))?;

    if !output.status.success() {
        return Err(EnvoyError::ExecutionError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let private_key = std::fs::read_to_string(&key_path)?;
    let public_key = std::fs::read_to_string(format!("{}.pub", key_path.display()))?;

    Ok((private_key, public_key))
}

/// Add public key to authorized_keys on remote server
pub async fn authorize_key(server: &Server, public_key: &str) -> EnvoyResult<()> {
    let session = SshSession::new(server.clone());

    // Escape single quotes in the public key to prevent shell injection
    let escaped_key = public_key.trim().replace('\'', "'\\''");
    let command = format!(
        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && echo '{}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys",
        escaped_key
    );

    session.exec(&command).await?;

    Ok(())
}

/// Copy a file to the remote server
pub async fn scp_upload(
    server: &Server,
    local_path: &str,
    remote_path: &str,
) -> EnvoyResult<()> {
    let remote = format!("{}:{}", server.ssh_host(), remote_path);

    let mut cmd = tokio::process::Command::new("scp");
    cmd.arg("-P").arg(server.port.to_string());

    if let Some(ref key) = server.identity_file {
        cmd.arg("-i").arg(key);
    }

    cmd.arg(local_path).arg(&remote);

    let output = cmd
        .output()
        .await
        .map_err(|e: std::io::Error| EnvoyError::ExecutionError(e.to_string()))?;

    if !output.status.success() {
        return Err(EnvoyError::ExecutionError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

/// Download a file from the remote server
pub async fn scp_download(
    server: &Server,
    remote_path: &str,
    local_path: &str,
) -> EnvoyResult<()> {
    let remote = format!("{}:{}", server.ssh_host(), remote_path);

    let mut cmd = tokio::process::Command::new("scp");
    cmd.arg("-P").arg(server.port.to_string());

    if let Some(ref key) = server.identity_file {
        cmd.arg("-i").arg(key);
    }

    cmd.arg(&remote).arg(local_path);

    let output = cmd
        .output()
        .await
        .map_err(|e: std::io::Error| EnvoyError::ExecutionError(e.to_string()))?;

    if !output.status.success() {
        return Err(EnvoyError::ExecutionError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_output() {
        let output = SshOutput {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };

        assert!(output.success());
    }
}
