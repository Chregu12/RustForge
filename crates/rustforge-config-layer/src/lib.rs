use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Global config instance
static CONFIG: Lazy<RwLock<Config>> = Lazy::new(|| {
    RwLock::new(Config::default())
});

/// Typed configuration system with Laravel-like API
///
/// Usage:
/// ```rust
/// config::app().name
/// config::database().host
/// config::get("app.name")
/// config::set("app.debug", true)
/// ```
pub struct Config {
    app: AppConfig,
    database: DatabaseConfig,
    cache: CacheConfig,
    queue: QueueConfig,
    mail: MailConfig,
    auth: AuthConfig,
    services: HashMap<String, ServiceConfig>,
    custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub env: Environment,
    pub debug: bool,
    pub url: String,
    pub port: u16,
    pub key: String,
    pub cipher: String,
    pub timezone: String,
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Environment {
    Local,
    Development,
    Staging,
    Production,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub default: String,
    pub connections: HashMap<String, DatabaseConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnection {
    pub driver: DatabaseDriver,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub charset: String,
    pub collation: Option<String>,
    pub prefix: Option<String>,
    pub pool: PoolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseDriver {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub min: u32,
    pub max: u32,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub default: String,
    pub stores: HashMap<String, CacheStore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStore {
    pub driver: CacheDriver,
    pub connection: Option<String>,
    pub table: Option<String>,
    pub prefix: String,
    pub ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheDriver {
    Redis,
    Memcached,
    File,
    Database,
    Array,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub default: String,
    pub connections: HashMap<String, QueueConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConnection {
    pub driver: QueueDriver,
    pub connection: Option<String>,
    pub queue: String,
    pub retry_after: u64,
    pub block_for: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueDriver {
    Sync,
    Database,
    Redis,
    SQS,
    RabbitMQ,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
    pub default: String,
    pub mailers: HashMap<String, Mailer>,
    pub from: MailAddress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailer {
    pub transport: MailTransport,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub encryption: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MailTransport {
    SMTP,
    Sendmail,
    Mailgun,
    SES,
    Postmark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAddress {
    pub address: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub defaults: AuthDefaults,
    pub guards: HashMap<String, AuthGuard>,
    pub providers: HashMap<String, AuthProvider>,
    pub passwords: HashMap<String, PasswordReset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthDefaults {
    pub guard: String,
    pub passwords: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthGuard {
    pub driver: AuthDriver,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthDriver {
    Session,
    Token,
    JWT,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProvider {
    pub driver: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordReset {
    pub provider: String,
    pub table: String,
    pub expire: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub options: HashMap<String, serde_json::Value>,
}

impl Config {
    /// Load configuration from directory
    pub fn load_from_dir(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut config = Self::default();

        // Load all config files
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let file_name = file_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                let contents = std::fs::read_to_string(&file_path)?;

                match file_name {
                    "app" => config.app = toml::from_str(&contents)?,
                    "database" => config.database = toml::from_str(&contents)?,
                    "cache" => config.cache = toml::from_str(&contents)?,
                    "queue" => config.queue = toml::from_str(&contents)?,
                    "mail" => config.mail = toml::from_str(&contents)?,
                    "auth" => config.auth = toml::from_str(&contents)?,
                    _ => {
                        // Load as service config
                        let service_config: ServiceConfig = toml::from_str(&contents)?;
                        config.services.insert(file_name.to_string(), service_config);
                    }
                }
            }
        }

        // Apply environment overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // Override with environment variables
        if let Ok(name) = std::env::var("APP_NAME") {
            self.app.name = name;
        }
        if let Ok(debug) = std::env::var("APP_DEBUG") {
            self.app.debug = debug.parse().unwrap_or(false);
        }
        if let Ok(url) = std::env::var("APP_URL") {
            self.app.url = url;
        }
        if let Ok(port) = std::env::var("APP_PORT") {
            if let Ok(port) = port.parse() {
                self.app.port = port;
            }
        }

        // Database overrides
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            // Parse DATABASE_URL and update config
            // This is simplified - real implementation would parse the URL properly
        }
    }

    /// Cache configuration for production
    pub fn cache(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }

    /// Load cached configuration
    pub fn from_cache(data: &[u8]) -> Result<Self> {
        Ok(bincode::deserialize(data)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app: AppConfig {
                name: "RustForge".to_string(),
                env: Environment::Development,
                debug: true,
                url: "http://localhost:3000".to_string(),
                port: 3000,
                key: "base64:generated-key-here".to_string(),
                cipher: "AES-256-CBC".to_string(),
                timezone: "UTC".to_string(),
                locale: "en".to_string(),
            },
            database: DatabaseConfig {
                default: "postgres".to_string(),
                connections: HashMap::new(),
            },
            cache: CacheConfig {
                default: "redis".to_string(),
                stores: HashMap::new(),
            },
            queue: QueueConfig {
                default: "sync".to_string(),
                connections: HashMap::new(),
            },
            mail: MailConfig {
                default: "smtp".to_string(),
                mailers: HashMap::new(),
                from: MailAddress {
                    address: "noreply@example.com".to_string(),
                    name: "RustForge".to_string(),
                },
            },
            auth: AuthConfig {
                defaults: AuthDefaults {
                    guard: "web".to_string(),
                    passwords: "users".to_string(),
                },
                guards: HashMap::new(),
                providers: HashMap::new(),
                passwords: HashMap::new(),
            },
            services: HashMap::new(),
            custom: HashMap::new(),
        }
    }
}

// Public API functions (Laravel-style)

/// Get app configuration
pub fn app() -> AppConfig {
    CONFIG.read().unwrap().app.clone()
}

/// Get database configuration
pub fn database() -> DatabaseConfig {
    CONFIG.read().unwrap().database.clone()
}

/// Get cache configuration
pub fn cache() -> CacheConfig {
    CONFIG.read().unwrap().cache.clone()
}

/// Get queue configuration
pub fn queue() -> QueueConfig {
    CONFIG.read().unwrap().queue.clone()
}

/// Get mail configuration
pub fn mail() -> MailConfig {
    CONFIG.read().unwrap().mail.clone()
}

/// Get auth configuration
pub fn auth() -> AuthConfig {
    CONFIG.read().unwrap().auth.clone()
}

/// Get configuration value by key (dot notation)
pub fn get(key: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let config = CONFIG.read().unwrap();

    match parts[0] {
        "app" => match parts.get(1) {
            Some(&"name") => Some(json!(config.app.name)),
            Some(&"debug") => Some(json!(config.app.debug)),
            Some(&"url") => Some(json!(config.app.url)),
            Some(&"port") => Some(json!(config.app.port)),
            _ => None,
        },
        "database" => match parts.get(1) {
            Some(&"default") => Some(json!(config.database.default)),
            _ => None,
        },
        _ => config.custom.get(key).cloned(),
    }
}

/// Set configuration value by key
pub fn set(key: &str, value: impl Serialize) -> Result<()> {
    let mut config = CONFIG.write().unwrap();
    let json_value = serde_json::to_value(value)?;

    let parts: Vec<&str> = key.split('.').collect();

    match parts[0] {
        "app" => match parts.get(1) {
            Some(&"name") => {
                if let Some(s) = json_value.as_str() {
                    config.app.name = s.to_string();
                }
            },
            Some(&"debug") => {
                if let Some(b) = json_value.as_bool() {
                    config.app.debug = b;
                }
            },
            _ => {}
        },
        _ => {
            config.custom.insert(key.to_string(), json_value);
        }
    }

    Ok(())
}

/// Check if configuration key exists
pub fn has(key: &str) -> bool {
    get(key).is_some()
}

/// Initialize configuration from directory
pub fn init(path: impl AsRef<Path>) -> Result<()> {
    let config = Config::load_from_dir(path)?;
    *CONFIG.write().unwrap() = config;
    Ok(())
}

/// Environment check helpers
pub fn is_production() -> bool {
    matches!(CONFIG.read().unwrap().app.env, Environment::Production)
}

pub fn is_development() -> bool {
    matches!(CONFIG.read().unwrap().app.env, Environment::Development)
}

pub fn is_local() -> bool {
    matches!(CONFIG.read().unwrap().app.env, Environment::Local)
}

pub fn environment() -> Environment {
    CONFIG.read().unwrap().app.env.clone()
}

// Macro for easy config access
#[macro_export]
macro_rules! config {
    ($key:expr) => {
        $crate::get($key)
    };
    ($key:expr, $default:expr) => {
        $crate::get($key).unwrap_or($default)
    };
}

// Helper trait for config builders
pub trait ConfigBuilder {
    fn from_env() -> Self;
    fn from_file(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized;
}

impl ConfigBuilder for AppConfig {
    fn from_env() -> Self {
        Self {
            name: std::env::var("APP_NAME").unwrap_or_else(|_| "RustForge".to_string()),
            env: std::env::var("APP_ENV")
                .ok()
                .and_then(|e| match e.as_str() {
                    "production" => Some(Environment::Production),
                    "staging" => Some(Environment::Staging),
                    "development" => Some(Environment::Development),
                    "local" => Some(Environment::Local),
                    _ => None,
                })
                .unwrap_or(Environment::Development),
            debug: std::env::var("APP_DEBUG")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(true),
            url: std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
            port: std::env::var("APP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            key: std::env::var("APP_KEY").unwrap_or_else(|_| "".to_string()),
            cipher: "AES-256-CBC".to_string(),
            timezone: std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string()),
            locale: std::env::var("APP_LOCALE").unwrap_or_else(|_| "en".to_string()),
        }
    }

    fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_get_set() {
        set("custom.key", "value").unwrap();
        assert_eq!(get("custom.key"), Some(json!("value")));
    }

    #[test]
    fn test_config_macro() {
        set("test.value", 42).unwrap();
        assert_eq!(config!("test.value"), Some(json!(42)));
        assert_eq!(config!("missing.value", json!("default")), json!("default"));
    }

    #[test]
    fn test_environment_helpers() {
        let mut config = CONFIG.write().unwrap();
        config.app.env = Environment::Production;
        drop(config);

        assert!(is_production());
        assert!(!is_development());
    }
}

// Re-exports for convenience
pub use serde_json::json;