use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub default: String,
    pub disks: HashMap<String, DiskConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub driver: String,
    pub root: Option<String>,
    pub url: Option<String>,
    pub visibility: Option<String>,
    // S3 specific
    pub key: Option<String>,
    pub secret: Option<String>,
    pub region: Option<String>,
    pub bucket: Option<String>,
    pub endpoint: Option<String>,
    // Azure specific
    pub account: Option<String>,
    pub container: Option<String>,
}

impl StorageConfig {
    pub fn from_env() -> Self {
        let env_config = std::env::var("STORAGE_CONFIG").unwrap_or_else(|_| "{}".to_string());

        serde_json::from_str(&env_config).unwrap_or_else(|_| StorageConfig {
            default: "local".to_string(),
            disks: HashMap::from_iter(vec![
                (
                    "local".to_string(),
                    DiskConfig {
                        driver: "local".to_string(),
                        root: Some("storage/app".to_string()),
                        url: Some("http://localhost:8000/storage".to_string()),
                        visibility: Some("private".to_string()),
                        ..Default::default()
                    },
                ),
                (
                    "public".to_string(),
                    DiskConfig {
                        driver: "local".to_string(),
                        root: Some("public/storage".to_string()),
                        url: Some("http://localhost:8000/storage".to_string()),
                        visibility: Some("public".to_string()),
                        ..Default::default()
                    },
                ),
            ]),
        })
    }
}

impl Default for DiskConfig {
    fn default() -> Self {
        Self {
            driver: "local".to_string(),
            root: None,
            url: None,
            visibility: None,
            key: None,
            secret: None,
            region: None,
            bucket: None,
            endpoint: None,
            account: None,
            container: None,
        }
    }
}
