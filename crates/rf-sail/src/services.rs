//! Pre-configured service definitions

use serde::{Deserialize, Serialize};

/// Available services
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Service {
    /// PostgreSQL database
    Postgres,
    /// MySQL database
    Mysql,
    /// MariaDB database
    Mariadb,
    /// Redis cache
    Redis,
    /// Memcached cache
    Memcached,
    /// MongoDB
    Mongodb,
    /// Meilisearch
    Meilisearch,
    /// Mailhog (email testing)
    Mailhog,
    /// MinIO (S3 compatible storage)
    Minio,
    /// Selenium (browser testing)
    Selenium,
    /// Soketi (WebSocket server)
    Soketi,
}

impl Service {
    /// Get the service name
    pub fn name(&self) -> &'static str {
        match self {
            Service::Postgres => "postgres",
            Service::Mysql => "mysql",
            Service::Mariadb => "mariadb",
            Service::Redis => "redis",
            Service::Memcached => "memcached",
            Service::Mongodb => "mongodb",
            Service::Meilisearch => "meilisearch",
            Service::Mailhog => "mailhog",
            Service::Minio => "minio",
            Service::Selenium => "selenium",
            Service::Soketi => "soketi",
        }
    }

    /// Get the Docker image
    pub fn image(&self) -> &'static str {
        match self {
            Service::Postgres => "postgres:16-alpine",
            Service::Mysql => "mysql:8.0",
            Service::Mariadb => "mariadb:10.11",
            Service::Redis => "redis:7-alpine",
            Service::Memcached => "memcached:1.6-alpine",
            Service::Mongodb => "mongo:7",
            Service::Meilisearch => "getmeili/meilisearch:v1.6",
            Service::Mailhog => "mailhog/mailhog:latest",
            Service::Minio => "minio/minio:latest",
            Service::Selenium => "selenium/standalone-chrome:latest",
            Service::Soketi => "quay.io/soketi/soketi:1.4-16-alpine",
        }
    }

    /// Get default ports (host:container)
    pub fn ports(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Service::Postgres => vec![("5432", "5432")],
            Service::Mysql => vec![("3306", "3306")],
            Service::Mariadb => vec![("3306", "3306")],
            Service::Redis => vec![("6379", "6379")],
            Service::Memcached => vec![("11211", "11211")],
            Service::Mongodb => vec![("27017", "27017")],
            Service::Meilisearch => vec![("7700", "7700")],
            Service::Mailhog => vec![("1025", "1025"), ("8025", "8025")],
            Service::Minio => vec![("9000", "9000"), ("9001", "9001")],
            Service::Selenium => vec![("4444", "4444"), ("5900", "5900")],
            Service::Soketi => vec![("6001", "6001"), ("9601", "9601")],
        }
    }

    /// Get default environment variables
    pub fn environment(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Service::Postgres => vec![
                ("POSTGRES_USER", "postgres"),
                ("POSTGRES_PASSWORD", "postgres"),
                ("POSTGRES_DB", "app"),
            ],
            Service::Mysql => vec![
                ("MYSQL_ROOT_PASSWORD", "root"),
                ("MYSQL_DATABASE", "app"),
                ("MYSQL_USER", "app"),
                ("MYSQL_PASSWORD", "app"),
            ],
            Service::Mariadb => vec![
                ("MARIADB_ROOT_PASSWORD", "root"),
                ("MARIADB_DATABASE", "app"),
                ("MARIADB_USER", "app"),
                ("MARIADB_PASSWORD", "app"),
            ],
            Service::Redis => vec![],
            Service::Memcached => vec![],
            Service::Mongodb => vec![
                ("MONGO_INITDB_ROOT_USERNAME", "root"),
                ("MONGO_INITDB_ROOT_PASSWORD", "root"),
            ],
            Service::Meilisearch => vec![
                ("MEILI_MASTER_KEY", "masterKey"),
                ("MEILI_ENV", "development"),
            ],
            Service::Mailhog => vec![],
            Service::Minio => vec![
                ("MINIO_ROOT_USER", "minioadmin"),
                ("MINIO_ROOT_PASSWORD", "minioadmin"),
            ],
            Service::Selenium => vec![],
            Service::Soketi => vec![
                ("SOKETI_DEBUG", "1"),
                ("SOKETI_DEFAULT_APP_ID", "app-id"),
                ("SOKETI_DEFAULT_APP_KEY", "app-key"),
                ("SOKETI_DEFAULT_APP_SECRET", "app-secret"),
            ],
        }
    }

    /// Get volumes
    pub fn volumes(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Service::Postgres => vec![("postgres_data", "/var/lib/postgresql/data")],
            Service::Mysql => vec![("mysql_data", "/var/lib/mysql")],
            Service::Mariadb => vec![("mariadb_data", "/var/lib/mysql")],
            Service::Redis => vec![("redis_data", "/data")],
            Service::Mongodb => vec![("mongodb_data", "/data/db")],
            Service::Meilisearch => vec![("meilisearch_data", "/meili_data")],
            Service::Minio => vec![("minio_data", "/data")],
            _ => vec![],
        }
    }

    /// Get command override
    pub fn command(&self) -> Option<&'static str> {
        match self {
            Service::Minio => Some("server /data --console-address ':9001'"),
            _ => None,
        }
    }

    /// Get health check command
    pub fn healthcheck(&self) -> Option<&'static str> {
        match self {
            Service::Postgres => Some("pg_isready -U postgres"),
            Service::Mysql => Some("mysqladmin ping -h localhost"),
            Service::Redis => Some("redis-cli ping"),
            Service::Mongodb => Some("mongosh --eval 'db.runCommand({ping:1})'"),
            _ => None,
        }
    }

    /// Get the connection URL environment variable name
    pub fn connection_env_var(&self) -> Option<&'static str> {
        match self {
            Service::Postgres => Some("DATABASE_URL"),
            Service::Mysql | Service::Mariadb => Some("DATABASE_URL"),
            Service::Redis => Some("REDIS_URL"),
            Service::Mongodb => Some("MONGODB_URL"),
            Service::Meilisearch => Some("MEILISEARCH_URL"),
            _ => None,
        }
    }

    /// Get the default connection URL
    pub fn connection_url(&self, host: &str) -> Option<String> {
        match self {
            Service::Postgres => Some(format!("postgres://postgres:postgres@{}:5432/app", host)),
            Service::Mysql => Some(format!("mysql://app:app@{}:3306/app", host)),
            Service::Mariadb => Some(format!("mysql://app:app@{}:3306/app", host)),
            Service::Redis => Some(format!("redis://{}:6379", host)),
            Service::Mongodb => Some(format!("mongodb://root:root@{}:27017", host)),
            Service::Meilisearch => Some(format!("http://{}:7700", host)),
            _ => None,
        }
    }
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub image: Option<String>,
    pub ports: Option<Vec<String>>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub volumes: Option<Vec<String>>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            image: None,
            ports: None,
            environment: None,
            volumes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_name() {
        assert_eq!(Service::Postgres.name(), "postgres");
        assert_eq!(Service::Redis.name(), "redis");
    }

    #[test]
    fn test_service_image() {
        assert!(Service::Postgres.image().contains("postgres"));
        assert!(Service::Redis.image().contains("redis"));
    }

    #[test]
    fn test_service_ports() {
        let ports = Service::Postgres.ports();
        assert!(!ports.is_empty());
        assert_eq!(ports[0], ("5432", "5432"));
    }

    #[test]
    fn test_connection_url() {
        let url = Service::Postgres.connection_url("localhost").unwrap();
        assert!(url.contains("postgres://"));
    }
}
