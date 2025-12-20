//! Server configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SSH server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// SSH connection string (user@host)
    pub host: String,
    /// SSH port (default: 22)
    pub port: u16,
    /// SSH user
    pub user: Option<String>,
    /// Path to SSH private key
    pub identity_file: Option<PathBuf>,
    /// SSH password (not recommended, use keys)
    pub password: Option<String>,
    /// Jump host for bastion/proxy
    pub jump_host: Option<String>,
    /// Working directory on the server
    pub working_dir: Option<String>,
}

impl Server {
    /// Create a new server from connection string
    pub fn new(connection: impl Into<String>) -> Self {
        let conn = connection.into();
        let (user, host, port) = Self::parse_connection(&conn);

        Self {
            host,
            port,
            user,
            identity_file: None,
            password: None,
            jump_host: None,
            working_dir: None,
        }
    }

    /// Parse connection string (user@host:port)
    fn parse_connection(conn: &str) -> (Option<String>, String, u16) {
        let mut user = None;
        let mut host = conn.to_string();
        let mut port = 22;

        // Extract user
        if let Some(at_pos) = conn.find('@') {
            user = Some(conn[..at_pos].to_string());
            host = conn[at_pos + 1..].to_string();
        }

        // Extract port
        if let Some(colon_pos) = host.rfind(':') {
            if let Ok(p) = host[colon_pos + 1..].parse() {
                port = p;
                host = host[..colon_pos].to_string();
            }
        }

        (user, host, port)
    }

    /// Set SSH port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set SSH user
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Set identity file (private key)
    pub fn identity_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.identity_file = Some(path.into());
        self
    }

    /// Set password (not recommended)
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set jump host for bastion
    pub fn jump_host(mut self, host: impl Into<String>) -> Self {
        self.jump_host = Some(host.into());
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Get the full SSH host string
    pub fn ssh_host(&self) -> String {
        match &self.user {
            Some(user) => format!("{}@{}", user, self.host),
            None => self.host.clone(),
        }
    }

    /// Get SSH command args
    pub fn ssh_args(&self) -> Vec<String> {
        let mut args = vec!["-p".to_string(), self.port.to_string()];

        if let Some(ref key) = self.identity_file {
            args.push("-i".to_string());
            args.push(key.display().to_string());
        }

        if let Some(ref jump) = self.jump_host {
            args.push("-J".to_string());
            args.push(jump.clone());
        }

        args.push(self.ssh_host());

        args
    }
}

/// Server configuration from config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub user: Option<String>,
    pub identity_file: Option<String>,
    pub jump_host: Option<String>,
    pub working_dir: Option<String>,
}

fn default_port() -> u16 {
    22
}

impl From<ServerConfig> for Server {
    fn from(config: ServerConfig) -> Self {
        let mut server = Server::new(&config.host);
        server.port = config.port;
        server.user = config.user;
        server.identity_file = config.identity_file.map(PathBuf::from);
        server.jump_host = config.jump_host;
        server.working_dir = config.working_dir;
        server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let server = Server::new("example.com");
        assert_eq!(server.host, "example.com");
        assert_eq!(server.port, 22);
        assert!(server.user.is_none());
    }

    #[test]
    fn test_parse_with_user() {
        let server = Server::new("deploy@example.com");
        assert_eq!(server.host, "example.com");
        assert_eq!(server.user, Some("deploy".to_string()));
    }

    #[test]
    fn test_parse_with_port() {
        let server = Server::new("deploy@example.com:2222");
        assert_eq!(server.host, "example.com");
        assert_eq!(server.port, 2222);
        assert_eq!(server.user, Some("deploy".to_string()));
    }

    #[test]
    fn test_ssh_host() {
        let server = Server::new("deploy@example.com");
        assert_eq!(server.ssh_host(), "deploy@example.com");
    }

    #[test]
    fn test_builder_methods() {
        let server = Server::new("example.com")
            .user("admin")
            .port(2222)
            .working_dir("/var/www");

        assert_eq!(server.user, Some("admin".to_string()));
        assert_eq!(server.port, 2222);
        assert_eq!(server.working_dir, Some("/var/www".to_string()));
    }
}
