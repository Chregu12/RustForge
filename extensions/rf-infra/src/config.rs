use dotenvy::from_path_iter;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{io::ErrorKind, path::PathBuf, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("konfiguration konnte nicht geladen werden: {0}")]
    LoadFailed(String),
    #[error("konfiguration nicht gefunden")]
    NotFound,
    #[error("konfigurationsschlüssel '{0}' fehlt")]
    Missing(String),
    #[error("ungültiger wert für schlüssel '{0}': {1}")]
    Invalid(String, String),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseDriver {
    Sqlite,
    Postgres,
}

impl FromStr for DatabaseDriver {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sqlite" => Ok(Self::Sqlite),
            "postgres" => Ok(Self::Postgres),
            _ => Err(ConfigError::Invalid(
                "DB_CONNECTION".into(),
                format!("unbekannter treiber `{s}`"),
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub url: String,
}

impl TryFrom<Value> for DatabaseConfig {
    type Error = ConfigError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let map = value
            .as_object()
            .ok_or_else(|| ConfigError::Invalid("config".into(), "muss ein objekt sein".into()))?;

        let driver_str = map
            .get("DB_CONNECTION")
            .and_then(Value::as_str)
            .ok_or_else(|| ConfigError::Missing("DB_CONNECTION".into()))?;

        let driver = DatabaseDriver::from_str(driver_str)?;

        let url = map
            .get("DATABASE_URL")
            .and_then(Value::as_str)
            .ok_or_else(|| ConfigError::Missing("DATABASE_URL".into()))?
            .to_string();

        Ok(DatabaseConfig { driver, url })
    }
}

pub trait ConfigProvider: Send + Sync {
    fn load(&self) -> Result<Value, ConfigError>;
}

pub struct DotenvProvider {
    pub path: PathBuf,
}

impl Default for DotenvProvider {
    fn default() -> Self {
        Self {
            path: PathBuf::from(".env"),
        }
    }
}

impl ConfigProvider for DotenvProvider {
    fn load(&self) -> Result<Value, ConfigError> {
        match from_path_iter(&self.path) {
            Ok(iter) => {
                let mut map = Map::new();
                for item in iter {
                    match item {
                        Ok((key, value)) => {
                            map.insert(key, Value::String(value));
                        }
                        Err(err) => {
                            return Err(ConfigError::LoadFailed(err.to_string()));
                        }
                    }
                }
                Ok(Value::Object(map))
            }
            Err(dotenvy::Error::Io(err)) if err.kind() == ErrorKind::NotFound => {
                Err(ConfigError::NotFound)
            }
            Err(err) => Err(ConfigError::LoadFailed(err.to_string())),
        }
    }
}
