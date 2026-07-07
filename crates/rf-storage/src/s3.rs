#![allow(dead_code)] // fields/methods retained for planned functionality, not read internally yet
//! S3-compatible storage backend

use crate::{Storage, StorageError, StorageResult};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client,
};
use chrono::Utc;
use std::time::Duration;

/// S3 storage configuration
#[derive(Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>, // For MinIO or other S3-compatible services
    pub access_key: String,
    pub secret_key: String,
    pub path_style: bool, // Force path-style URLs (for MinIO)
}

/// S3-compatible storage backend
pub struct S3Storage {
    config: S3Config,
    client: Client,
    base_url: String,
}

impl S3Storage {
    /// Create new S3 storage
    pub async fn new(config: S3Config) -> StorageResult<Self> {
        let base_url = if let Some(endpoint) = &config.endpoint {
            format!("{}/{}", endpoint, config.bucket)
        } else {
            format!(
                "https://s3.{}.amazonaws.com/{}",
                config.region, config.bucket
            )
        };

        let credentials = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "rf-storage",
        );

        let region = Region::new(config.region.clone());

        let mut sdk_config_builder = aws_sdk_s3::Config::builder()
            .credentials_provider(credentials)
            .region(region)
            .behavior_version(BehaviorVersion::latest());

        if let Some(endpoint) = &config.endpoint {
            sdk_config_builder = sdk_config_builder
                .endpoint_url(endpoint)
                .force_path_style(config.path_style);
        }

        let client = Client::from_conf(sdk_config_builder.build());

        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    /// Generate signed URL for temporary access
    pub async fn signed_url(&self, path: &str, expires_in: Duration) -> StorageResult<String> {
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| StorageError::Other(e.to_string()))?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(path)
            .presigned(presigning_config)
            .await
            .map_err(|e| StorageError::Other(e.to_string()))?;

        Ok(presigned_request.uri().to_string())
    }

    /// Get S3 client configuration
    fn client_config(&self) -> String {
        format!(
            "Bucket: {}, Region: {}, Endpoint: {:?}",
            self.config.bucket, self.config.region, self.config.endpoint
        )
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn put(&self, path: &str, contents: Vec<u8>) -> Result<(), StorageError> {
        tracing::debug!(
            path = %path,
            size = contents.len(),
            bucket = %self.config.bucket,
            "Uploading to S3"
        );

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(path)
            .body(ByteStream::from(contents))
            .send()
            .await
            .map_err(|e| StorageError::Other(e.to_string()))?;

        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        tracing::debug!(
            path = %path,
            bucket = %self.config.bucket,
            "Downloading from S3"
        );

        let resp = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    StorageError::FileNotFound(path.to_string())
                } else {
                    StorageError::Other(e.to_string())
                }
            })?;

        let bytes = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::Other(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        tracing::debug!(
            path = %path,
            bucket = %self.config.bucket,
            "Deleting from S3"
        );

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| StorageError::Other(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        match self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(path)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(StorageError::Other(e.to_string()))
                }
            }
        }
    }

    async fn size(&self, path: &str) -> Result<u64, StorageError> {
        let resp = self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NotFound") {
                    StorageError::FileNotFound(path.to_string())
                } else {
                    StorageError::Other(e.to_string())
                }
            })?;

        Ok(resp.content_length().unwrap_or(0) as u64)
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        tracing::debug!(
            prefix = %prefix,
            bucket = %self.config.bucket,
            "Listing S3 objects"
        );

        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| StorageError::Other(e.to_string()))?;

        let files = resp
            .contents()
            .iter()
            .filter_map(|obj| obj.key().map(|k| k.to_string()))
            .collect();

        Ok(files)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path)
    }

    async fn last_modified(
        &self,
        path: &str,
    ) -> Result<Option<chrono::DateTime<Utc>>, StorageError> {
        let resp = self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(path)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NotFound") {
                    StorageError::FileNotFound(path.to_string())
                } else {
                    StorageError::Other(e.to_string())
                }
            })?;

        if let Some(last_modified) = resp.last_modified() {
            let timestamp = last_modified.secs();
            let dt = chrono::DateTime::from_timestamp(timestamp, 0);
            Ok(dt)
        } else {
            Ok(None)
        }
    }

    async fn temporary_url(
        &self,
        path: &str,
        expires_in: Duration,
    ) -> Result<Option<String>, StorageError> {
        let url = self.signed_url(path, expires_in).await?;
        Ok(Some(url))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to check if S3/MinIO is available for testing
    async fn s3_available() -> bool {
        tokio::net::TcpStream::connect("127.0.0.1:9000")
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn test_s3_config() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            path_style: false,
        };

        let storage = S3Storage::new(config).await.unwrap();
        assert_eq!(
            storage.url("test.txt"),
            "https://s3.us-east-1.amazonaws.com/test-bucket/test.txt"
        );
    }

    #[tokio::test]
    async fn test_s3_with_endpoint() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            path_style: true,
        };

        let storage = S3Storage::new(config).await.unwrap();
        assert_eq!(
            storage.url("test.txt"),
            "http://localhost:9000/test-bucket/test.txt"
        );
    }

    #[tokio::test]
    async fn test_signed_url() {
        if !s3_available().await {
            eprintln!("⏭️  Skipping test_signed_url: MinIO/S3 not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            path_style: false,
        };

        let storage = S3Storage::new(config).await.unwrap();
        let url = storage
            .signed_url("test.txt", Duration::from_secs(3600))
            .await;
        assert!(url.is_ok());
    }

    #[tokio::test]
    async fn test_s3_operations() {
        if !s3_available().await {
            eprintln!("⏭️  Skipping test_s3_operations: MinIO/S3 not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            path_style: true,
        };

        let storage = S3Storage::new(config).await.unwrap();

        // Test put
        let result = storage.put("test.txt", b"Hello".to_vec()).await;
        assert!(result.is_ok());

        // Test exists
        let exists = storage.exists("test.txt").await.unwrap();
        assert!(exists);

        // Test get
        let contents = storage.get("test.txt").await.unwrap();
        assert_eq!(contents, b"Hello");

        // Test size
        let size = storage.size("test.txt").await.unwrap();
        assert_eq!(size, 5);

        // Test delete
        let result = storage.delete("test.txt").await;
        assert!(result.is_ok());

        // Verify deleted
        let exists = storage.exists("test.txt").await.unwrap();
        assert!(!exists);
    }
}
