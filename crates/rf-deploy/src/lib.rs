//! Deployment Helpers for RustForge
//!
//! This crate provides code generation for deployment configurations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Deployment errors
#[derive(Debug, Error)]
pub enum DeployError {
    #[error("Generation error: {0}")]
    GenerationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type DeployResult<T> = Result<T, DeployError>;

/// Dockerfile builder
pub struct DockerfileBuilder {
    rust_version: String,
    features: Vec<String>,
    optimize_size: bool,
    port: u16,
}

impl DockerfileBuilder {
    /// Create a new Dockerfile builder
    pub fn new() -> Self {
        Self {
            rust_version: "1.75".to_string(),
            features: Vec::new(),
            optimize_size: false,
            port: 8000,
        }
    }

    /// Set Rust version
    pub fn rust_version(mut self, version: impl Into<String>) -> Self {
        self.rust_version = version.into();
        self
    }

    /// Add a feature
    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Optimize for binary size
    pub fn optimize_for_size(mut self) -> Self {
        self.optimize_size = true;
        self
    }

    /// Set exposed port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Build the Dockerfile
    pub fn build(&self) -> DeployResult<String> {
        let mut dockerfile = String::new();

        // Build stage
        dockerfile.push_str(&format!(
            "# Build stage\nFROM rust:{} as builder\n\n",
            self.rust_version
        ));
        dockerfile.push_str("WORKDIR /app\n\n");
        dockerfile.push_str("# Copy manifests\n");
        dockerfile.push_str("COPY Cargo.toml Cargo.lock ./\n");
        dockerfile.push_str("COPY crates ./crates\n\n");

        dockerfile.push_str("# Build application\n");
        let mut build_cmd = "RUN cargo build --release".to_string();
        if !self.features.is_empty() {
            build_cmd.push_str(&format!(" --features {}", self.features.join(",")));
        }
        dockerfile.push_str(&build_cmd);
        dockerfile.push_str("\n\n");

        if self.optimize_size {
            dockerfile.push_str("# Strip binary\n");
            dockerfile.push_str("RUN strip target/release/app\n\n");
        }

        // Runtime stage
        dockerfile.push_str("# Runtime stage\n");
        dockerfile.push_str("FROM debian:bookworm-slim\n\n");
        dockerfile.push_str("# Install runtime dependencies\n");
        dockerfile.push_str("RUN apt-get update && apt-get install -y \\\n");
        dockerfile.push_str("    ca-certificates \\\n");
        dockerfile.push_str("    libssl3 \\\n");
        dockerfile.push_str("    && rm -rf /var/lib/apt/lists/*\n\n");

        dockerfile.push_str("WORKDIR /app\n\n");
        dockerfile.push_str("# Copy binary from builder\n");
        dockerfile.push_str("COPY --from=builder /app/target/release/app /app/app\n\n");

        dockerfile.push_str(&format!("EXPOSE {}\n\n", self.port));
        dockerfile.push_str("CMD [\"/app/app\"]\n");

        Ok(dockerfile)
    }
}

impl Default for DockerfileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Docker Compose service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    pub image: Option<String>,
    pub build: Option<String>,
    pub ports: Vec<String>,
    pub environment: Vec<String>,
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Docker Compose configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerCompose {
    pub version: String,
    pub services: std::collections::HashMap<String, ComposeService>,
}

/// Docker Compose builder
pub struct DockerComposeBuilder {
    services: std::collections::HashMap<String, ComposeService>,
    app_name: String,
}

impl DockerComposeBuilder {
    /// Create a new Docker Compose builder
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
            app_name: "app".to_string(),
        }
    }

    /// Set application name
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Add application service
    pub fn app_service(mut self, name: impl Into<String>, port: u16) -> Self {
        let name = name.into();
        let service = ComposeService {
            build: Some(".".to_string()),
            image: None,
            ports: vec![format!("{}:{}", port, port)],
            environment: vec![
                "RUST_LOG=info".to_string(),
                format!("PORT={}", port),
            ],
            depends_on: Vec::new(),
            volumes: None,
            command: None,
        };

        self.services.insert(name, service);
        self
    }

    /// Add PostgreSQL service
    pub fn postgres_service(mut self, version: impl Into<String>) -> Self {
        let service = ComposeService {
            image: Some(format!("postgres:{}", version.into())),
            build: None,
            ports: vec!["5432:5432".to_string()],
            environment: vec![
                "POSTGRES_USER=postgres".to_string(),
                "POSTGRES_PASSWORD=postgres".to_string(),
                "POSTGRES_DB=app".to_string(),
            ],
            depends_on: Vec::new(),
            volumes: Some(vec!["postgres_data:/var/lib/postgresql/data".to_string()]),
            command: None,
        };

        self.services.insert("postgres".to_string(), service);

        // Add postgres as dependency to app
        if let Some(app) = self.services.get_mut(&self.app_name) {
            app.depends_on.push("postgres".to_string());
            app.environment.push("DATABASE_URL=postgres://postgres:postgres@postgres:5432/app".to_string());
        }

        self
    }

    /// Add Redis service
    pub fn redis_service(mut self) -> Self {
        let service = ComposeService {
            image: Some("redis:7-alpine".to_string()),
            build: None,
            ports: vec!["6379:6379".to_string()],
            environment: Vec::new(),
            depends_on: Vec::new(),
            volumes: Some(vec!["redis_data:/data".to_string()]),
            command: None,
        };

        self.services.insert("redis".to_string(), service);

        // Add redis as dependency to app
        if let Some(app) = self.services.get_mut(&self.app_name) {
            app.depends_on.push("redis".to_string());
            app.environment.push("REDIS_URL=redis://redis:6379".to_string());
        }

        self
    }

    /// Build the Docker Compose configuration
    pub fn build(&self) -> DeployResult<String> {
        let compose = DockerCompose {
            version: "3.8".to_string(),
            services: self.services.clone(),
        };

        let mut yaml = serde_yaml::to_string(&compose)
            .map_err(|e| DeployError::SerializationError(e.to_string()))?;

        // Add volumes section if needed
        let has_postgres = self.services.contains_key("postgres");
        let has_redis = self.services.contains_key("redis");

        if has_postgres || has_redis {
            yaml.push_str("\nvolumes:\n");
            if has_postgres {
                yaml.push_str("  postgres_data:\n");
            }
            if has_redis {
                yaml.push_str("  redis_data:\n");
            }
        }

        Ok(yaml)
    }
}

impl Default for DockerComposeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Kubernetes deployment configuration
pub struct KubernetesBuilder {
    app_name: String,
    namespace: String,
    replicas: u32,
    image: String,
    port: u16,
}

impl KubernetesBuilder {
    /// Create a new Kubernetes builder
    pub fn new(app_name: impl Into<String>, image: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            namespace: "default".to_string(),
            replicas: 3,
            image: image.into(),
            port: 8000,
        }
    }

    /// Set namespace
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Set number of replicas
    pub fn replicas(mut self, replicas: u32) -> Self {
        self.replicas = replicas;
        self
    }

    /// Set port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Build the Kubernetes deployment manifest
    pub fn build_deployment(&self) -> DeployResult<String> {
        let mut yaml = String::new();

        yaml.push_str("apiVersion: apps/v1\n");
        yaml.push_str("kind: Deployment\n");
        yaml.push_str("metadata:\n");
        yaml.push_str(&format!("  name: {}\n", self.app_name));
        yaml.push_str(&format!("  namespace: {}\n", self.namespace));
        yaml.push_str("spec:\n");
        yaml.push_str(&format!("  replicas: {}\n", self.replicas));
        yaml.push_str("  selector:\n");
        yaml.push_str("    matchLabels:\n");
        yaml.push_str(&format!("      app: {}\n", self.app_name));
        yaml.push_str("  template:\n");
        yaml.push_str("    metadata:\n");
        yaml.push_str("      labels:\n");
        yaml.push_str(&format!("        app: {}\n", self.app_name));
        yaml.push_str("    spec:\n");
        yaml.push_str("      containers:\n");
        yaml.push_str(&format!("      - name: {}\n", self.app_name));
        yaml.push_str(&format!("        image: {}\n", self.image));
        yaml.push_str("        ports:\n");
        yaml.push_str(&format!("        - containerPort: {}\n", self.port));
        yaml.push_str("        env:\n");
        yaml.push_str("        - name: RUST_LOG\n");
        yaml.push_str("          value: \"info\"\n");
        yaml.push_str("        livenessProbe:\n");
        yaml.push_str("          httpGet:\n");
        yaml.push_str("            path: /health/live\n");
        yaml.push_str(&format!("            port: {}\n", self.port));
        yaml.push_str("          initialDelaySeconds: 30\n");
        yaml.push_str("          periodSeconds: 10\n");
        yaml.push_str("        readinessProbe:\n");
        yaml.push_str("          httpGet:\n");
        yaml.push_str("            path: /health/ready\n");
        yaml.push_str(&format!("            port: {}\n", self.port));
        yaml.push_str("          initialDelaySeconds: 5\n");
        yaml.push_str("          periodSeconds: 5\n");

        Ok(yaml)
    }

    /// Build the Kubernetes service manifest
    pub fn build_service(&self) -> DeployResult<String> {
        let mut yaml = String::new();

        yaml.push_str("apiVersion: v1\n");
        yaml.push_str("kind: Service\n");
        yaml.push_str("metadata:\n");
        yaml.push_str(&format!("  name: {}\n", self.app_name));
        yaml.push_str(&format!("  namespace: {}\n", self.namespace));
        yaml.push_str("spec:\n");
        yaml.push_str("  selector:\n");
        yaml.push_str(&format!("    app: {}\n", self.app_name));
        yaml.push_str("  ports:\n");
        yaml.push_str("  - protocol: TCP\n");
        yaml.push_str(&format!("    port: {}\n", self.port));
        yaml.push_str(&format!("    targetPort: {}\n", self.port));
        yaml.push_str("  type: LoadBalancer\n");

        Ok(yaml)
    }
}

/// Environment file generator
pub struct EnvFileBuilder {
    vars: std::collections::HashMap<String, String>,
}

impl EnvFileBuilder {
    /// Create a new environment file builder
    pub fn new() -> Self {
        Self {
            vars: std::collections::HashMap::new(),
        }
    }

    /// Add a variable
    pub fn var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Add database URL
    pub fn database(mut self, url: impl Into<String>) -> Self {
        self.vars.insert("DATABASE_URL".to_string(), url.into());
        self
    }

    /// Add Redis URL
    pub fn redis(mut self, url: impl Into<String>) -> Self {
        self.vars.insert("REDIS_URL".to_string(), url.into());
        self
    }

    /// Build the .env file
    pub fn build(&self) -> DeployResult<String> {
        let mut env = String::new();

        let mut keys: Vec<_> = self.vars.keys().collect();
        keys.sort();

        for key in keys {
            if let Some(value) = self.vars.get(key) {
                env.push_str(&format!("{}={}\n", key, value));
            }
        }

        Ok(env)
    }
}

impl Default for EnvFileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dockerfile_builder() {
        let dockerfile = DockerfileBuilder::new()
            .rust_version("1.75")
            .with_feature("postgres")
            .optimize_for_size()
            .port(3000)
            .build()
            .unwrap();

        assert!(dockerfile.contains("FROM rust:1.75 as builder"));
        assert!(dockerfile.contains("--features postgres"));
        assert!(dockerfile.contains("strip target/release/app"));
        assert!(dockerfile.contains("EXPOSE 3000"));
    }

    #[test]
    fn test_docker_compose_builder() {
        let compose = DockerComposeBuilder::new()
            .app_name("my-app")
            .app_service("my-app", 3000)
            .postgres_service("15")
            .redis_service()
            .build()
            .unwrap();

        assert!(compose.contains("version:"));
        assert!(compose.contains("my-app:"));
        assert!(compose.contains("postgres:"));
        assert!(compose.contains("redis:"));
        assert!(compose.contains("postgres_data:"));
        assert!(compose.contains("redis_data:"));
    }

    #[test]
    fn test_kubernetes_deployment() {
        let k8s = KubernetesBuilder::new("my-app", "my-app:latest")
            .namespace("production")
            .replicas(5)
            .port(8000);

        let deployment = k8s.build_deployment().unwrap();

        assert!(deployment.contains("kind: Deployment"));
        assert!(deployment.contains("name: my-app"));
        assert!(deployment.contains("namespace: production"));
        assert!(deployment.contains("replicas: 5"));
        assert!(deployment.contains("image: my-app:latest"));
        assert!(deployment.contains("containerPort: 8000"));
        assert!(deployment.contains("/health/live"));
        assert!(deployment.contains("/health/ready"));
    }

    #[test]
    fn test_kubernetes_service() {
        let k8s = KubernetesBuilder::new("my-app", "my-app:latest")
            .port(8000);

        let service = k8s.build_service().unwrap();

        assert!(service.contains("kind: Service"));
        assert!(service.contains("name: my-app"));
        assert!(service.contains("port: 8000"));
        assert!(service.contains("type: LoadBalancer"));
    }

    #[test]
    fn test_env_file_builder() {
        let env = EnvFileBuilder::new()
            .var("APP_NAME", "my-app")
            .var("PORT", "8000")
            .database("postgres://localhost/db")
            .redis("redis://localhost:6379")
            .build()
            .unwrap();

        assert!(env.contains("APP_NAME=my-app"));
        assert!(env.contains("PORT=8000"));
        assert!(env.contains("DATABASE_URL=postgres://localhost/db"));
        assert!(env.contains("REDIS_URL=redis://localhost:6379"));
    }

    #[test]
    fn test_dockerfile_without_optimization() {
        let dockerfile = DockerfileBuilder::new().build().unwrap();
        assert!(!dockerfile.contains("strip"));
    }

    #[test]
    fn test_docker_compose_without_databases() {
        let compose = DockerComposeBuilder::new()
            .app_service("my-app", 3000)
            .build()
            .unwrap();

        assert!(compose.contains("my-app:"));
        assert!(!compose.contains("postgres:"));
        assert!(!compose.contains("redis:"));
    }
}
