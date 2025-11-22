//! S3 Storage Usage Example
//!
//! This example demonstrates how to use the S3 storage driver with RustForge.
//!
//! # Setup
//!
//! 1. Start MinIO (S3-compatible storage):
//!    ```bash
//!    docker run -p 9000:9000 -p 9001:9001 \
//!      -e MINIO_ROOT_USER=minioadmin \
//!      -e MINIO_ROOT_PASSWORD=minioadmin \
//!      minio/minio server /data --console-address ":9001"
//!    ```
//!
//! 2. Access MinIO Console at http://localhost:9001
//!    - Login: minioadmin / minioadmin
//!    - Create a bucket named "my-app-files"
//!
//! 3. Run this example:
//!    ```bash
//!    cargo run --example s3_usage
//!    ```

use rf_storage::{S3Config, S3Storage, Storage, StorageManager};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 RustForge S3 Storage Example\n");

    // ===== 1. Basic S3 Configuration =====
    println!("📦 Configuring S3 storage...");

    let config = S3Config {
        bucket: "my-app-files".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key: "minioadmin".to_string(),
        secret_key: "minioadmin".to_string(),
        path_style: true, // Required for MinIO
    };

    let storage = S3Storage::new(config).await?;
    println!("✓ S3 storage configured\n");

    // ===== 2. Upload Files =====
    println!("📤 Uploading files...");

    // Upload a text file
    storage
        .put("documents/readme.txt", b"Welcome to RustForge!".to_vec())
        .await?;
    println!("✓ Uploaded: documents/readme.txt");

    // Upload JSON data
    let user_data = serde_json::json!({
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
    });
    storage
        .put("data/user-1.json", serde_json::to_vec(&user_data)?)
        .await?;
    println!("✓ Uploaded: data/user-1.json");

    // Upload an image (simulated)
    let fake_image = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
    storage.put("images/avatar.jpg", fake_image).await?;
    println!("✓ Uploaded: images/avatar.jpg\n");

    // ===== 3. Download Files =====
    println!("📥 Downloading files...");

    let readme = storage.get("documents/readme.txt").await?;
    println!("✓ Downloaded readme.txt:");
    println!("  Content: {}\n", String::from_utf8_lossy(&readme));

    let user_json = storage.get("data/user-1.json").await?;
    let user: serde_json::Value = serde_json::from_slice(&user_json)?;
    println!("✓ Downloaded user-1.json:");
    println!("  User: {}\n", user);

    // ===== 4. File Operations =====
    println!("📋 File operations...");

    // Check if file exists
    let exists = storage.exists("documents/readme.txt").await?;
    println!("✓ File exists check: {}", exists);

    // Get file size
    let size = storage.size("documents/readme.txt").await?;
    println!("✓ File size: {} bytes", size);

    // Get public URL
    let url = storage.url("images/avatar.jpg");
    println!("✓ Public URL: {}\n", url);

    // ===== 5. List Files =====
    println!("📂 Listing files...");

    let documents = storage.list("documents/").await?;
    println!("✓ Documents folder:");
    for doc in documents {
        println!("  - {}", doc);
    }

    let all_files = storage.list("").await?;
    println!("\n✓ All files ({} total):", all_files.len());
    for file in &all_files {
        println!("  - {}", file);
    }
    println!();

    // ===== 6. Copy and Move Files =====
    println!("🔄 Copy and move operations...");

    // Copy a file
    storage
        .copy("documents/readme.txt", "documents/readme-backup.txt")
        .await?;
    println!("✓ Copied: readme.txt → readme-backup.txt");

    // Move a file
    storage
        .move_file("documents/readme-backup.txt", "backups/readme.txt")
        .await?;
    println!("✓ Moved: readme-backup.txt → backups/readme.txt\n");

    // ===== 7. Temporary URLs (Signed) =====
    println!("🔐 Generating temporary URLs...");

    // Generate a signed URL that expires in 1 hour
    let temp_url = storage
        .temporary_url("data/user-1.json", Duration::from_secs(3600))
        .await?;

    if let Some(url) = temp_url {
        println!("✓ Temporary URL (expires in 1 hour):");
        println!("  {}\n", url);
    }

    // ===== 8. Using Storage Manager (Multi-Disk) =====
    println!("💾 Storage Manager with multiple disks...");

    let mut manager = StorageManager::new();

    // Add S3 disk
    let s3_storage = S3Storage::new(S3Config {
        bucket: "my-app-files".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key: "minioadmin".to_string(),
        secret_key: "minioadmin".to_string(),
        path_style: true,
    })
    .await?;
    manager.add_disk("s3", Arc::new(s3_storage));

    // Use default disk (s3)
    let disk = manager.disk_default()?;
    disk.put("manager-test.txt", b"Hello from manager!".to_vec())
        .await?;
    println!("✓ Uploaded via storage manager");

    let content = disk.get("manager-test.txt").await?;
    println!(
        "✓ Downloaded via storage manager: {}\n",
        String::from_utf8_lossy(&content)
    );

    // ===== 9. Error Handling =====
    println!("⚠️  Error handling...");

    match storage.get("non-existent-file.txt").await {
        Ok(_) => println!("✗ Should have failed"),
        Err(e) => println!("✓ Correctly handled error: {}", e),
    }
    println!();

    // ===== 10. Cleanup =====
    println!("🧹 Cleanup...");

    let files_to_delete = vec![
        "documents/readme.txt",
        "data/user-1.json",
        "images/avatar.jpg",
        "backups/readme.txt",
        "manager-test.txt",
    ];

    for file in files_to_delete {
        if storage.exists(file).await? {
            storage.delete(file).await?;
            println!("✓ Deleted: {}", file);
        }
    }

    println!("\n✅ S3 storage example completed successfully!");
    Ok(())
}
