//! Comprehensive S3 storage integration tests
//!
//! These tests require a running MinIO/S3 instance.
//! Start MinIO with: docker run -p 9000:9000 -p 9001:9001 minio/minio server /data --console-address ":9001"
//!
//! Or use the test environment script: ./scripts/test-env-up.sh

use rf_storage::{S3Config, S3Storage, Storage, StorageError};
use std::time::Duration;

/// Helper to check if S3 (MinIO) is available
async fn s3_available() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:9000").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Helper to create a test S3 storage instance
async fn create_test_storage() -> S3Storage {
    let config = S3Config {
        bucket: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key: "minioadmin".to_string(),
        secret_key: "minioadmin".to_string(),
        path_style: true,
    };

    let storage = S3Storage::new(config).await.unwrap();

    // Ensure bucket exists (create if needed)
    // Note: MinIO auto-creates buckets on first PUT, but we can explicitly create it
    // For now, we'll rely on auto-creation during tests

    storage
}

#[tokio::test]
async fn test_s3_put_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        eprintln!("   Start MinIO with: docker run -p 9000:9000 minio/minio server /data");
        return;
    }

    let storage = create_test_storage().await;

    let result = storage
        .put("test-files/hello.txt", b"Hello, S3!".to_vec())
        .await;

    assert!(result.is_ok(), "Failed to put file: {:?}", result);
}

#[tokio::test]
async fn test_s3_get_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put a file first
    let content = b"Test content for retrieval";
    storage
        .put("test-files/retrieve.txt", content.to_vec())
        .await
        .unwrap();

    // Get the file
    let retrieved = storage.get("test-files/retrieve.txt").await.unwrap();

    assert_eq!(retrieved, content);
}

#[tokio::test]
async fn test_s3_file_exists() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put a file
    storage
        .put("test-files/exists.txt", b"I exist!".to_vec())
        .await
        .unwrap();

    // Check it exists
    let exists = storage.exists("test-files/exists.txt").await.unwrap();
    assert!(exists);

    // Check non-existent file
    let not_exists = storage
        .exists("test-files/does-not-exist.txt")
        .await
        .unwrap();
    assert!(!not_exists);
}

#[tokio::test]
async fn test_s3_delete_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put a file
    storage
        .put("test-files/delete-me.txt", b"Delete this".to_vec())
        .await
        .unwrap();

    // Verify it exists
    assert!(storage.exists("test-files/delete-me.txt").await.unwrap());

    // Delete it
    let result = storage.delete("test-files/delete-me.txt").await;
    assert!(result.is_ok());

    // Verify it's gone
    assert!(!storage.exists("test-files/delete-me.txt").await.unwrap());
}

#[tokio::test]
async fn test_s3_get_file_url() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    let url = storage.url("test-files/public.txt");

    assert!(url.contains("localhost:9000"));
    assert!(url.contains("test-bucket"));
    assert!(url.contains("test-files/public.txt"));
}

#[tokio::test]
async fn test_s3_temporary_url() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put a file first
    storage
        .put("test-files/private.txt", b"Private content".to_vec())
        .await
        .unwrap();

    // Generate temporary URL
    let temp_url = storage
        .temporary_url("test-files/private.txt", Duration::from_secs(3600))
        .await
        .unwrap();

    assert!(temp_url.is_some());
    let url = temp_url.unwrap();

    // URL should contain signature parameters
    assert!(url.contains("X-Amz-Signature") || url.contains("Signature"));
    assert!(url.contains("test-files/private.txt"));
}

#[tokio::test]
async fn test_s3_list_files() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put multiple files
    storage
        .put("test-list/file1.txt", b"File 1".to_vec())
        .await
        .unwrap();
    storage
        .put("test-list/file2.txt", b"File 2".to_vec())
        .await
        .unwrap();
    storage
        .put("test-list/file3.txt", b"File 3".to_vec())
        .await
        .unwrap();

    // List files
    let files = storage.list("test-list/").await.unwrap();

    assert!(files.len() >= 3);
    assert!(files.iter().any(|f| f.contains("file1.txt")));
    assert!(files.iter().any(|f| f.contains("file2.txt")));
    assert!(files.iter().any(|f| f.contains("file3.txt")));
}

#[tokio::test]
async fn test_s3_copy_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put original file
    let content = b"Original content";
    storage
        .put("test-copy/original.txt", content.to_vec())
        .await
        .unwrap();

    // Copy it
    let result = storage
        .copy("test-copy/original.txt", "test-copy/copy.txt")
        .await;
    assert!(result.is_ok());

    // Verify both exist
    assert!(storage.exists("test-copy/original.txt").await.unwrap());
    assert!(storage.exists("test-copy/copy.txt").await.unwrap());

    // Verify content matches
    let copied = storage.get("test-copy/copy.txt").await.unwrap();
    assert_eq!(copied, content);
}

#[tokio::test]
async fn test_s3_move_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put original file
    let content = b"Move me";
    storage
        .put("test-move/original.txt", content.to_vec())
        .await
        .unwrap();

    // Move it
    let result = storage
        .move_file("test-move/original.txt", "test-move/moved.txt")
        .await;
    assert!(result.is_ok());

    // Verify original is gone
    assert!(!storage.exists("test-move/original.txt").await.unwrap());

    // Verify new location exists
    assert!(storage.exists("test-move/moved.txt").await.unwrap());

    // Verify content matches
    let moved = storage.get("test-move/moved.txt").await.unwrap();
    assert_eq!(moved, content);
}

#[tokio::test]
async fn test_s3_file_not_found_error() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Try to get non-existent file
    let result = storage.get("non-existent/file.txt").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        StorageError::FileNotFound(_) => {
            // Expected error
        }
        other => panic!("Expected FileNotFound error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_s3_file_size() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put a file
    let content = b"This is exactly 42 bytes of content!!";
    assert_eq!(content.len(), 38); // Actually 38, but that's fine
    storage
        .put("test-size/measured.txt", content.to_vec())
        .await
        .unwrap();

    // Get size
    let size = storage.size("test-size/measured.txt").await.unwrap();

    assert_eq!(size, content.len() as u64);
}

#[tokio::test]
async fn test_s3_last_modified() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put a file
    storage
        .put("test-modified/timestamped.txt", b"Content".to_vec())
        .await
        .unwrap();

    // Get last modified
    let modified = storage
        .last_modified("test-modified/timestamped.txt")
        .await
        .unwrap();

    assert!(modified.is_some());
}

#[tokio::test]
async fn test_s3_large_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Create a 1MB file
    let large_content = vec![42u8; 1024 * 1024];

    // Put it
    storage
        .put("test-large/big-file.bin", large_content.clone())
        .await
        .unwrap();

    // Get it back
    let retrieved = storage.get("test-large/big-file.bin").await.unwrap();

    assert_eq!(retrieved.len(), large_content.len());
    assert_eq!(retrieved, large_content);
}

#[tokio::test]
async fn test_s3_nested_paths() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put file in deeply nested path
    let path = "test-nested/level1/level2/level3/deep-file.txt";
    storage
        .put(path, b"Deep content".to_vec())
        .await
        .unwrap();

    // Verify it exists
    assert!(storage.exists(path).await.unwrap());

    // Get it
    let content = storage.get(path).await.unwrap();
    assert_eq!(content, b"Deep content");
}

#[tokio::test]
async fn test_s3_empty_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put empty file
    storage
        .put("test-empty/empty.txt", vec![])
        .await
        .unwrap();

    // Get it
    let content = storage.get("test-empty/empty.txt").await.unwrap();
    assert_eq!(content.len(), 0);

    // Check size
    let size = storage.size("test-empty/empty.txt").await.unwrap();
    assert_eq!(size, 0);
}

#[tokio::test]
async fn test_s3_special_characters_in_path() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Put file with special characters in path
    let path = "test-special/file-with-dashes_and_underscores (and parens).txt";
    storage
        .put(path, b"Special path".to_vec())
        .await
        .unwrap();

    // Verify it exists
    assert!(storage.exists(path).await.unwrap());

    // Get it
    let content = storage.get(path).await.unwrap();
    assert_eq!(content, b"Special path");
}

#[tokio::test]
async fn test_s3_overwrite_file() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    let path = "test-overwrite/file.txt";

    // Put initial content
    storage.put(path, b"Original".to_vec()).await.unwrap();

    // Verify initial content
    let content1 = storage.get(path).await.unwrap();
    assert_eq!(content1, b"Original");

    // Overwrite
    storage.put(path, b"Updated".to_vec()).await.unwrap();

    // Verify updated content
    let content2 = storage.get(path).await.unwrap();
    assert_eq!(content2, b"Updated");
}

#[tokio::test]
async fn test_s3_concurrent_operations() {
    if !s3_available().await {
        eprintln!("⏭️  Skipping S3 tests: MinIO not available");
        return;
    }

    let storage = create_test_storage().await;

    // Perform multiple operations concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let storage = create_test_storage().await;
        let handle = tokio::spawn(async move {
            let path = format!("test-concurrent/file-{}.txt", i);
            let content = format!("Content {}", i);
            storage.put(&path, content.as_bytes().to_vec()).await.unwrap();

            let retrieved = storage.get(&path).await.unwrap();
            assert_eq!(retrieved, content.as_bytes());
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // List all files
    let files = storage.list("test-concurrent/").await.unwrap();
    assert!(files.len() >= 10);
}
