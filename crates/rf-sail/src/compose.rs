//! Docker Compose file generation

use crate::{SailConfig, SailError, SailResult, Service};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Docker Compose configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeConfig {
    pub version: String,
    pub services: HashMap<String, ComposeService>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<HashMap<String, serde_yaml::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<HashMap<String, serde_yaml::Value>>,
}

/// Docker Compose service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<ComposeBuild>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<HealthCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeBuild {
    pub context: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub test: Vec<String>,
    pub interval: String,
    pub timeout: String,
    pub retries: u32,
}

/// Docker Compose generator
pub struct ComposeGenerator<'a> {
    config: &'a SailConfig,
}

impl<'a> ComposeGenerator<'a> {
    pub fn new(config: &'a SailConfig) -> Self {
        Self { config }
    }

    /// Generate the docker-compose.yml content
    pub fn generate(&self) -> SailResult<String> {
        let mut services = HashMap::new();
        let mut volumes = HashMap::new();

        // Generate app service
        services.insert(self.config.app_name.clone(), self.generate_app_service());

        // Generate service dependencies
        let mut depends_on = Vec::new();

        for service in &self.config.services {
            let (svc_name, svc_config, svc_volumes) = self.generate_service(service);
            services.insert(svc_name.clone(), svc_config);
            depends_on.push(svc_name);

            for (vol_name, _) in svc_volumes {
                volumes.insert(vol_name.to_string(), serde_yaml::Value::Null);
            }
        }

        // Update app service with dependencies
        if let Some(app_service) = services.get_mut(&self.config.app_name) {
            app_service.depends_on = Some(depends_on);
        }

        let compose = ComposeConfig {
            version: "3.8".to_string(),
            services,
            volumes: if volumes.is_empty() {
                None
            } else {
                Some(volumes)
            },
            networks: None,
        };

        serde_yaml::to_string(&compose).map_err(|e| SailError::ConfigError(e.to_string()))
    }

    /// Generate the app service configuration
    fn generate_app_service(&self) -> ComposeService {
        let mut environment = vec![
            format!("RUST_LOG=debug"),
            format!("PORT={}", self.config.app_port),
        ];

        // Add service connection URLs
        for service in &self.config.services {
            if let (Some(env_var), Some(url)) = (
                service.connection_env_var(),
                service.connection_url(service.name()),
            ) {
                environment.push(format!("{}={}", env_var, url));
            }
        }

        // Add custom environment variables
        for (key, value) in &self.config.environment {
            environment.push(format!("{}={}", key, value));
        }

        ComposeService {
            image: None,
            build: Some(ComposeBuild {
                context: ".".to_string(),
                dockerfile: Some(self.config.dockerfile.clone()),
                target: self.config.build_target.clone(),
                args: None,
            }),
            ports: Some(vec![format!(
                "{}:{}",
                self.config.app_port, self.config.app_port
            )]),
            environment: Some(environment),
            volumes: Some(vec![
                ".:/app".to_string(),
                "cargo_cache:/usr/local/cargo/registry".to_string(),
                "target_cache:/app/target".to_string(),
            ]),
            depends_on: None,
            command: None,
            healthcheck: None,
            restart: Some("unless-stopped".to_string()),
            networks: None,
        }
    }

    /// Generate a service configuration
    fn generate_service(
        &self,
        service: &Service,
    ) -> (String, ComposeService, Vec<(&'static str, &'static str)>) {
        let name = service.name().to_string();

        let ports: Vec<String> = service
            .ports()
            .iter()
            .map(|(host, container)| format!("{}:{}", host, container))
            .collect();

        let environment: Vec<String> = service
            .environment()
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let volume_mounts: Vec<String> = service
            .volumes()
            .iter()
            .map(|(name, path)| format!("{}:{}", name, path))
            .collect();

        let healthcheck = service.healthcheck().map(|cmd| HealthCheck {
            test: vec!["CMD-SHELL".to_string(), cmd.to_string()],
            interval: "10s".to_string(),
            timeout: "5s".to_string(),
            retries: 3,
        });

        let svc = ComposeService {
            image: Some(service.image().to_string()),
            build: None,
            ports: if ports.is_empty() { None } else { Some(ports) },
            environment: if environment.is_empty() {
                None
            } else {
                Some(environment)
            },
            volumes: if volume_mounts.is_empty() {
                None
            } else {
                Some(volume_mounts)
            },
            depends_on: None,
            command: service.command().map(|s| s.to_string()),
            healthcheck,
            restart: Some("unless-stopped".to_string()),
            networks: None,
        };

        (name, svc, service.volumes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_compose() {
        let config = SailConfig::default();
        let generator = ComposeGenerator::new(&config);
        let yaml = generator.generate().unwrap();

        assert!(yaml.contains("version:"));
        assert!(yaml.contains("services:"));
        assert!(yaml.contains("postgres:"));
        assert!(yaml.contains("redis:"));
    }

    #[test]
    fn test_app_service() {
        let config = SailConfig::new("myapp");
        let generator = ComposeGenerator::new(&config);
        let svc = generator.generate_app_service();

        assert!(svc.build.is_some());
        assert!(svc.ports.is_some());
    }
}
