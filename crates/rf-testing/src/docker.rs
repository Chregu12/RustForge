//! Docker integration for integration tests
//!
//! Provides utilities for running services in Docker containers for testing.
//! Includes helpers for Redis, PostgreSQL, and other common services.
//!
//! # Example
//!
//! ```no_run
//! use rf_testing::docker::{DockerCompose, Service};
//!
//! # async fn example() {
//! // Start Docker Compose services
//! let compose = DockerCompose::new();
//! compose.up().await.expect("Failed to start services");
//!
//! // Get service URLs
//! let redis_url = compose.service_url(Service::Redis)
//!     .expect("Redis not available");
//!
//! // ... run tests ...
//!
//! compose.down().await.ok();
//! # }
//! ```

use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Available services in docker-compose.test.yml
#[derive(Debug, Clone, Copy)]
pub enum Service {
    /// Redis caching and queue service
    Redis,
    /// PostgreSQL database service
    Postgres,
    /// MailHog email testing service
    MailHog,
    /// MinIO S3-compatible storage service
    MinIO,
}

impl Service {
    /// Get the container name for the service
    pub fn container_name(self) -> &'static str {
        match self {
            Service::Redis => "rustforge_redis_test",
            Service::Postgres => "rustforge_postgres_test",
            Service::MailHog => "rustforge_mailhog_test",
            Service::MinIO => "rustforge_minio_test",
        }
    }

    /// Get the port for the service
    pub fn port(self) -> u16 {
        match self {
            Service::Redis => 6379,
            Service::Postgres => 5432,
            Service::MailHog => 1025,
            Service::MinIO => 9000,
        }
    }
}

/// Docker Compose test manager
///
/// Handles starting/stopping services defined in docker-compose.test.yml
pub struct DockerCompose {
    compose_file: String,
    project_name: String,
}

impl DockerCompose {
    /// Creates a new DockerCompose instance
    ///
    /// # Arguments
    ///
    /// * `compose_file` - Path to docker-compose.test.yml (relative to workspace root)
    /// * `project_name` - Docker Compose project name
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_testing::docker::DockerCompose;
    ///
    /// let compose = DockerCompose::new();
    /// ```
    pub fn new() -> Self {
        Self {
            compose_file: "docker-compose.test.yml".to_string(),
            project_name: "rustforge_test".to_string(),
        }
    }

    /// Set the compose file path
    pub fn with_compose_file(mut self, file: String) -> Self {
        self.compose_file = file;
        self
    }

    /// Set the project name
    pub fn with_project_name(mut self, name: String) -> Self {
        self.project_name = name;
        self
    }

    /// Start all services
    pub async fn up(&self) -> Result<(), String> {
        let output = Command::new("docker-compose")
            .arg("-f")
            .arg(&self.compose_file)
            .arg("-p")
            .arg(&self.project_name)
            .arg("up")
            .arg("-d")
            .output()
            .map_err(|e| format!("Failed to run docker-compose up: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("docker-compose up failed: {}", stderr));
        }

        // Wait for services to be healthy
        self.wait_for_services().await?;

        Ok(())
    }

    /// Stop all services
    pub async fn down(&self) -> Result<(), String> {
        let output = Command::new("docker-compose")
            .arg("-f")
            .arg(&self.compose_file)
            .arg("-p")
            .arg(&self.project_name)
            .arg("down")
            .output()
            .map_err(|e| format!("Failed to run docker-compose down: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("docker-compose down failed: {}", stderr));
        }

        Ok(())
    }

    /// Get the connection URL for a service
    pub fn service_url(&self, service: Service) -> Option<String> {
        match service {
            Service::Redis => Some(format!("redis://localhost:{}", Service::Redis.port())),
            Service::Postgres => Some(format!(
                "postgresql://rustforge:testpass@localhost:{}/rustforge_test",
                Service::Postgres.port()
            )),
            Service::MailHog => Some(format!("http://localhost:8025")),
            Service::MinIO => Some(format!("http://localhost:{}", Service::MinIO.port())),
        }
    }

    /// Wait for services to be healthy
    async fn wait_for_services(&self) -> Result<(), String> {
        let services = vec![
            Service::Redis,
            Service::Postgres,
            Service::MailHog,
            Service::MinIO,
        ];

        for service in services {
            self.wait_for_service(service).await?;
        }

        Ok(())
    }

    /// Wait for a specific service to be healthy
    async fn wait_for_service(&self, service: Service) -> Result<(), String> {
        let container_name = service.container_name();
        let max_attempts = 30;
        let mut attempts = 0;

        loop {
            attempts += 1;

            let output = Command::new("docker")
                .arg("inspect")
                .arg("--format='{{.State.Health.Status}}'")
                .arg(container_name)
                .output();

            match output {
                Ok(output) if output.status.success() => {
                    let status = String::from_utf8_lossy(&output.stdout);
                    if status.contains("healthy") || status.contains("running") {
                        return Ok(());
                    }
                }
                _ => {}
            }

            if attempts >= max_attempts {
                return Err(format!("Service {} did not become healthy", container_name));
            }

            sleep(Duration::from_millis(500)).await;
        }
    }
}

impl Default for DockerCompose {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if Redis is available for testing
///
/// Returns true if a Redis connection can be established
///
/// # Example
///
/// ```no_run
/// use rf_testing::docker::redis_available;
///
/// # async fn example() {
/// if redis_available().await {
///     println!("Redis is available for testing");
/// } else {
///     println!("Skipping test - Redis not available");
///     return;
/// }
/// # }
/// ```
pub async fn redis_available() -> bool {
    let client = match redis::Client::open("redis://localhost:6379") {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut conn = match client.get_async_connection().await {
        Ok(c) => c,
        Err(_) => return false,
    };

    redis::cmd("PING")
        .query_async::<_, String>(&mut conn)
        .await
        .is_ok()
}

/// Check if PostgreSQL is available for testing
///
/// Returns true if a database connection can be established
///
/// # Example
///
/// ```no_run
/// use rf_testing::docker::postgres_available;
///
/// # async fn example() {
/// if postgres_available().await {
///     println!("PostgreSQL is available for testing");
/// } else {
///     println!("Skipping test - PostgreSQL not available");
///     return;
/// }
/// # }
/// ```
pub async fn postgres_available() -> bool {
    use std::net::TcpStream;
    use std::time::Duration;

    TcpStream::connect_timeout(&"127.0.0.1:5432".parse().unwrap(), Duration::from_secs(1)).is_ok()
}

/// Check if MinIO/S3 is available for testing
///
/// Returns true if MinIO service is responding
///
/// # Example
///
/// ```no_run
/// use rf_testing::docker::s3_available;
///
/// # async fn example() {
/// if s3_available().await {
///     println!("MinIO is available for testing");
/// } else {
///     println!("Skipping test - MinIO not available");
///     return;
/// }
/// # }
/// ```
pub async fn s3_available() -> bool {
    use std::net::TcpStream;
    use std::time::Duration;

    TcpStream::connect_timeout(&"127.0.0.1:9000".parse().unwrap(), Duration::from_secs(1)).is_ok()
}

/// Check if MailHog SMTP is available for testing
///
/// Returns true if MailHog SMTP port is accessible
///
/// # Example
///
/// ```no_run
/// use rf_testing::docker::mailhog_available;
///
/// # async fn example() {
/// if mailhog_available().await {
///     println!("MailHog is available for testing");
/// } else {
///     println!("Skipping test - MailHog not available");
///     return;
/// }
/// # }
/// ```
pub async fn mailhog_available() -> bool {
    use std::net::TcpStream;
    use std::time::Duration;

    TcpStream::connect_timeout(&"127.0.0.1:1025".parse().unwrap(), Duration::from_secs(1)).is_ok()
}

/// Check if the database is available for testing (alias for postgres_available)
pub async fn database_available() -> bool {
    postgres_available().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_container_names() {
        assert_eq!(Service::Redis.container_name(), "rustforge_redis_test");
        assert_eq!(
            Service::Postgres.container_name(),
            "rustforge_postgres_test"
        );
        assert_eq!(Service::MailHog.container_name(), "rustforge_mailhog_test");
        assert_eq!(Service::MinIO.container_name(), "rustforge_minio_test");
    }

    #[test]
    fn service_ports() {
        assert_eq!(Service::Redis.port(), 6379);
        assert_eq!(Service::Postgres.port(), 5432);
        assert_eq!(Service::MailHog.port(), 1025);
        assert_eq!(Service::MinIO.port(), 9000);
    }

    #[test]
    fn docker_compose_initialization() {
        let compose = DockerCompose::new();
        assert_eq!(compose.compose_file, "docker-compose.test.yml");
        assert_eq!(compose.project_name, "rustforge_test");
    }

    #[test]
    fn docker_compose_with_custom_file() {
        let compose =
            DockerCompose::new().with_compose_file("docker-compose.custom.yml".to_string());
        assert_eq!(compose.compose_file, "docker-compose.custom.yml");
    }

    #[test]
    fn docker_compose_with_custom_project_name() {
        let compose = DockerCompose::new().with_project_name("custom_project".to_string());
        assert_eq!(compose.project_name, "custom_project");
    }

    #[test]
    fn service_urls() {
        let compose = DockerCompose::new();

        let redis_url = compose.service_url(Service::Redis).unwrap();
        assert!(redis_url.contains("redis://localhost"));

        let postgres_url = compose.service_url(Service::Postgres).unwrap();
        assert!(postgres_url.contains("postgresql://"));

        let mailhog_url = compose.service_url(Service::MailHog).unwrap();
        assert!(mailhog_url.contains("http://localhost:8025"));

        let minio_url = compose.service_url(Service::MinIO).unwrap();
        assert!(minio_url.contains("http://localhost:9000"));
    }
}
