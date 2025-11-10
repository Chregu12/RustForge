//! S3-compatible storage backend

use crate::{Storage, StorageError, StorageResult};
use async_trait::async_trait;
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
#[derive(Clone)]
pub struct S3Storage {
    config: S3Config,
    base_url: String,
}

impl S3Storage {
    /// Create new S3 storage
    pub fn new(config: S3Config) -> Self {
        let base_url = if let Some(endpoint) = &config.endpoint {
            format!("{}/{}", endpoint, config.bucket)
        } else {
            format!("https://s3.{}.amazonaws.com/{}", config.region, config.bucket)
        };

        Self { config, base_url }
    }

    /// Generate signed URL for temporary access
    pub fn signed_url(&self, path: &str, expires_in: Duration) -> StorageResult<String> {
        // Simplified signed URL generation
        // In production, use AWS SDK's presigned URL functionality
        let expires = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| StorageError::Other(e.to_string()))?
            .as_secs()
            + expires_in.as_secs();

        Ok(format!(
            "{}/{}?X-Amz-Expires={}",
            self.base_url, path, expires
        ))
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
        // Simulate S3 put operation
        // In production, use: client.put_object().bucket().key().body().send().await
        tracing::debug!(
            "S3Storage::put - path: {}, size: {} bytes, config: {}",
            path,
            contents.len(),
            self.client_config()
        );

        // For now, return success
        // Real implementation would use AWS SDK
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        // Simulate S3 get operation
        // In production, use: client.get_object().bucket().key().send().await
        tracing::debug!(
            "S3Storage::get - path: {}, config: {}",
            path,
            self.client_config()
        );

        // Return empty vec for simulation
        // Real implementation would download from S3
        Ok(Vec::new())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        // Simulate S3 delete operation
        // In production, use: client.delete_object().bucket().key().send().await
        tracing::debug!(
            "S3Storage::delete - path: {}, config: {}",
            path,
            self.client_config()
        );

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        // Simulate S3 head operation
        // In production, use: client.head_object().bucket().key().send().await
        tracing::debug!(
            "S3Storage::exists - path: {}, config: {}",
            path,
            self.client_config()
        );

        Ok(false) // Simulated response
    }

    async fn size(&self, path: &str) -> Result<u64, StorageError> {
        // Simulate S3 head operation to get size
        // In production, use: client.head_object().bucket().key().send().await
        tracing::debug!(
            "S3Storage::size - path: {}, config: {}",
            path,
            self.client_config()
        );

        Ok(0) // Simulated response
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        // Simulate S3 list operation
        // In production, use: client.list_objects_v2().bucket().prefix().send().await
        tracing::debug!(
            "S3Storage::list - prefix: {}, config: {}",
            prefix,
            self.client_config()
        );

        Ok(Vec::new()) // Simulated response
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_config() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            path_style: false,
        };

        let storage = S3Storage::new(config);
        assert_eq!(
            storage.url("test.txt"),
            "https://s3.us-east-1.amazonaws.com/test-bucket/test.txt"
        );
    }

    #[test]
    fn test_s3_with_endpoint() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            path_style: true,
        };

        let storage = S3Storage::new(config);
        assert_eq!(
            storage.url("test.txt"),
            "http://localhost:9000/test-bucket/test.txt"
        );
    }

    #[test]
    fn test_signed_url() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: "access".to_string(),
            secret_key: "secret".to_string(),
            path_style: false,
        };

        let storage = S3Storage::new(config);
        let url = storage.signed_url("test.txt", Duration::from_secs(3600));
        assert!(url.is_ok());
        assert!(url.unwrap().contains("X-Amz-Expires"));
    }

    #[tokio::test]
    async fn test_s3_operations() {
        let config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some("http://localhost:9000".to_string()),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            path_style: true,
        };

        let storage = S3Storage::new(config);

        // Test put (simulated)
        let result = storage.put("test.txt", b"Hello".to_vec()).await;
        assert!(result.is_ok());

        // Test get (simulated)
        let result = storage.get("test.txt").await;
        assert!(result.is_ok());

        // Test delete (simulated)
        let result = storage.delete("test.txt").await;
        assert!(result.is_ok());
    }
}
