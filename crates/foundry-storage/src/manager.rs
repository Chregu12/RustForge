use crate::config::{DiskConfig, StorageConfig};
use crate::local::LocalStorage;
use crate::{Disk, Storage, StorageDriver};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct StorageManager {
    default: String,
    disks: HashMap<String, Arc<Disk>>,
}

impl StorageManager {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let mut disks = HashMap::new();

        for (name, disk_config) in config.disks {
            let disk = Self::create_disk(&name, disk_config)?;
            disks.insert(name, Arc::new(disk));
        }

        Ok(Self {
            default: config.default,
            disks,
        })
    }

    fn create_disk(name: &str, config: DiskConfig) -> Result<Disk> {
        let storage: Arc<dyn Storage> = match config.driver.as_str() {
            "local" => {
                let root = config.root.unwrap_or_else(|| "storage/app".to_string());
                let url = config
                    .url
                    .unwrap_or_else(|| "http://localhost:8000/storage".to_string());
                Arc::new(LocalStorage::new(root, url))
            }
            "s3" => {
                // TODO: Implement S3 storage
                unimplemented!("S3 storage is not yet implemented.");
            }
            _ => return Err(anyhow::anyhow!("Unknown storage driver: {}", config.driver)),
        };

        let driver = match config.driver.as_str() {
            "local" => StorageDriver::Local,
            "s3" => StorageDriver::S3,
            _ => return Err(anyhow::anyhow!("Unknown storage driver: {}", config.driver)),
        };

        Ok(Disk::new(name.to_string(), driver, storage))
    }

    pub fn disk(&self, name: Option<&str>) -> Result<Arc<Disk>> {
        let name = name.unwrap_or(&self.default);
        self.disks
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Disk not found: {}", name))
    }

    pub fn default_disk(&self) -> Result<Arc<Disk>> {
        self.disk(None)
    }
}
