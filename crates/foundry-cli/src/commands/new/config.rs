use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project template types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateType {
    ApiRest,
    FullStackReact,
    FullStackLeptos,
    CliTool,
}

impl TemplateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateType::ApiRest => "API (REST/GraphQL Backend)",
            TemplateType::FullStackReact => "Full-Stack (React + Axum)",
            TemplateType::FullStackLeptos => "Full-Stack (Leptos WASM)",
            TemplateType::CliTool => "CLI Tool",
        }
    }

    pub fn all() -> Vec<TemplateType> {
        vec![
            TemplateType::ApiRest,
            TemplateType::FullStackReact,
            TemplateType::FullStackLeptos,
            TemplateType::CliTool,
        ]
    }
}

/// Available project features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    Authentication,
    Database,
    Redis,
    Email,
    Tests,
}

impl Feature {
    pub fn as_str(&self) -> &'static str {
        match self {
            Feature::Authentication => "Authentication (JWT)",
            Feature::Database => "Database (PostgreSQL)",
            Feature::Redis => "Redis Cache",
            Feature::Email => "Email (SMTP)",
            Feature::Tests => "Tests & Fixtures",
        }
    }

    pub fn all() -> Vec<Feature> {
        vec![
            Feature::Authentication,
            Feature::Database,
            Feature::Redis,
            Feature::Email,
            Feature::Tests,
        ]
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            name: "app_dev".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            password: "postgres".to_string(),
        }
    }
}

impl DatabaseConfig {
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.name
        )
    }

    pub fn connection_url_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/postgres",
            self.username, self.password, self.host, self.port
        )
    }
}

/// Complete project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub path: PathBuf,
    pub template: TemplateType,
    pub features: Vec<Feature>,
    pub database: Option<DatabaseConfig>,
}

impl ProjectConfig {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            template: TemplateType::ApiRest,
            features: Vec::new(),
            database: None,
        }
    }

    pub fn has_feature(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    pub fn has_database(&self) -> bool {
        self.has_feature(Feature::Database)
    }

    pub fn has_auth(&self) -> bool {
        self.has_feature(Feature::Authentication)
    }

    pub fn has_redis(&self) -> bool {
        self.has_feature(Feature::Redis)
    }

    pub fn has_email(&self) -> bool {
        self.has_feature(Feature::Email)
    }

    pub fn has_tests(&self) -> bool {
        self.has_feature(Feature::Tests)
    }

    pub fn sanitized_name(&self) -> String {
        self.name
            .to_lowercase()
            .replace('-', "_")
            .replace(' ', "_")
    }

    pub fn db_name(&self) -> String {
        format!("{}_dev", self.sanitized_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_conversion() {
        assert_eq!(TemplateType::ApiRest.as_str(), "API (REST/GraphQL Backend)");
        assert_eq!(TemplateType::all().len(), 4);
    }

    #[test]
    fn test_feature_detection() {
        let mut config = ProjectConfig::new("test".to_string(), PathBuf::from("/tmp/test"));
        assert!(!config.has_database());

        config.features.push(Feature::Database);
        assert!(config.has_database());
    }

    #[test]
    fn test_sanitized_name() {
        let config = ProjectConfig::new("My-Cool App".to_string(), PathBuf::from("/tmp"));
        assert_eq!(config.sanitized_name(), "my_cool_app");
    }

    #[test]
    fn test_db_config_urls() {
        let db_config = DatabaseConfig::default();
        assert!(db_config.connection_url().contains("postgres://"));
        assert!(db_config.connection_url().contains("app_dev"));
    }
}
