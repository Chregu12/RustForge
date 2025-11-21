# Cloud Storage & Broadcasting - Quick Start Guide

## 🚀 Quick Start: S3 Storage

### 1. Add Dependency
```toml
[dependencies]
rf-storage = { path = "crates/rf-storage" }
tokio = { version = "1.0", features = ["full"] }
```

### 2. Basic Usage
```rust
use rf_storage::{S3Config, S3Storage, Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure S3
    let config = S3Config {
        bucket: "my-bucket".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()), // MinIO
        access_key: "minioadmin".to_string(),
        secret_key: "minioadmin".to_string(),
        path_style: true,
    };

    let storage = S3Storage::new(config).await?;

    // Upload
    storage.put("hello.txt", b"Hello, World!".to_vec()).await?;

    // Download
    let contents = storage.get("hello.txt").await?;
    println!("File contents: {}", String::from_utf8_lossy(&contents));

    // Delete
    storage.delete("hello.txt").await?;

    Ok(())
}
```

### 3. Run MinIO (for local development)
```bash
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"
```

### 4. Test It
```bash
cargo test -p rf-storage
```

---

## 🌐 Quick Start: Broadcasting

### 1. Add Dependency
```toml
[dependencies]
rf-broadcasting = { path = "crates/rf-broadcasting" }
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### 2. Define Event
```rust
use rf_broadcasting::Broadcast;
use serde_json::json;

#[derive(Debug, Clone)]
struct OrderShipped {
    order_id: u64,
    customer_name: String,
}

impl Broadcast for OrderShipped {
    fn broadcast_on(&self) -> Vec<String> {
        vec!["orders".to_string()]
    }

    fn broadcast_as(&self) -> Option<String> {
        Some("OrderShipped".to_string())
    }

    fn broadcast_with(&self) -> serde_json::Value {
        json!({
            "order_id": self.order_id,
            "customer_name": self.customer_name,
        })
    }
}
```

### 3. Broadcast Event
```rust
use rf_broadcasting::{Broadcaster, RedisBroadcastDriver};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup broadcaster
    let driver = RedisBroadcastDriver::from_url("redis://localhost:6379")?;
    let broadcaster = Broadcaster::new(Arc::new(driver));

    // Broadcast event
    let event = OrderShipped {
        order_id: 123,
        customer_name: "John Doe".to_string(),
    };

    broadcaster.broadcast(event).await?;
    println!("Event broadcasted!");

    Ok(())
}
```

### 4. Run WebSocket Server
```rust
use rf_broadcasting::{WebSocketServer, WebSocketConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WebSocketConfig::default(); // Port 6001

    let server = WebSocketServer::new(config);
    println!("WebSocket server starting on port 6001");

    server.start().await?;
    Ok(())
}
```

### 5. Run Redis
```bash
docker run -p 6379:6379 redis
```

### 6. Test Client (JavaScript)
```html
<!DOCTYPE html>
<html>
<body>
    <h1>Broadcasting Test</h1>
    <div id="messages"></div>

    <script>
        const ws = new WebSocket('ws://localhost:6001');

        ws.onopen = () => {
            console.log('Connected!');

            // Subscribe to orders channel
            ws.send(JSON.stringify({
                command: 'subscribe',
                channel: 'orders',
                auth: null
            }));
        };

        ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            console.log('Received:', message);

            if (message.type === 'event') {
                const div = document.getElementById('messages');
                div.innerHTML += `<p>Event: ${message.event} - ${JSON.stringify(message.data)}</p>`;
            }
        };
    </script>
</body>
</html>
```

### 7. Test It
```bash
cargo test -p rf-broadcasting
```

---

## 📦 Complete Example: File Upload with Broadcasting

This example uploads a file to S3 and broadcasts an event when done.

```rust
use rf_storage::{S3Config, S3Storage, Storage};
use rf_broadcasting::{Broadcast, Broadcaster, RedisBroadcastDriver};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct FileUploaded {
    file_name: String,
    file_size: u64,
    url: String,
}

impl Broadcast for FileUploaded {
    fn broadcast_on(&self) -> Vec<String> {
        vec!["files".to_string()]
    }

    fn broadcast_as(&self) -> Option<String> {
        Some("FileUploaded".to_string())
    }

    fn broadcast_with(&self) -> serde_json::Value {
        json!({
            "file_name": self.file_name,
            "file_size": self.file_size,
            "url": self.url,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup S3 storage
    let s3_config = S3Config {
        bucket: "uploads".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key: "minioadmin".to_string(),
        secret_key: "minioadmin".to_string(),
        path_style: true,
    };
    let storage = S3Storage::new(s3_config).await?;

    // Setup broadcaster
    let driver = RedisBroadcastDriver::from_url("redis://localhost:6379")?;
    let broadcaster = Broadcaster::new(Arc::new(driver));

    // Upload file
    let file_name = "document.pdf";
    let file_contents = b"PDF content here...".to_vec();

    storage.put(file_name, file_contents.clone()).await?;

    // Get file info
    let size = storage.size(file_name).await?;
    let url = storage.url(file_name);

    println!("File uploaded: {} ({} bytes)", file_name, size);
    println!("URL: {}", url);

    // Broadcast event
    let event = FileUploaded {
        file_name: file_name.to_string(),
        file_size: size,
        url,
    };

    broadcaster.broadcast(event).await?;
    println!("Upload event broadcasted!");

    Ok(())
}
```

---

## 🔧 Configuration

### Environment Variables
```bash
# S3 Configuration
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key
AWS_REGION=us-east-1
S3_BUCKET=my-bucket
S3_ENDPOINT=http://localhost:9000  # For MinIO

# Redis Configuration
REDIS_URL=redis://localhost:6379

# WebSocket Configuration
WS_PORT=6001
WS_HOST=0.0.0.0
```

### Production Configuration (AWS S3)
```rust
let config = S3Config {
    bucket: env::var("S3_BUCKET")?,
    region: env::var("AWS_REGION")?,
    endpoint: None, // Use AWS S3 directly
    access_key: env::var("AWS_ACCESS_KEY_ID")?,
    secret_key: env::var("AWS_SECRET_ACCESS_KEY")?,
    path_style: false, // Use virtual-hosted-style
};
```

---

## 📝 Common Tasks

### Upload with Signed URL
```rust
use std::time::Duration;

// Upload file
storage.put("private/document.pdf", file_contents).await?;

// Generate signed URL (expires in 1 hour)
let signed_url = storage
    .temporary_url("private/document.pdf", Duration::from_secs(3600))
    .await?
    .unwrap();

println!("Temporary URL: {}", signed_url);
```

### List Files
```rust
// List all files in uploads/ directory
let files = storage.list("uploads/").await?;

for file in files {
    println!("File: {}", file);
}
```

### Copy Files
```rust
// Copy file to backup
storage.copy(
    "uploads/document.pdf",
    "backups/document-2024-01-01.pdf"
).await?;
```

### Move Files
```rust
// Move file to archive
storage.move_file(
    "uploads/old.pdf",
    "archive/2024/old.pdf"
).await?;
```

### Multi-Disk Storage
```rust
use rf_storage::StorageManager;

let mut manager = StorageManager::new();

// Add multiple disks
manager.add_disk("s3", Arc::new(s3_storage));
manager.add_disk("local", Arc::new(local_storage));

// Set default
manager.set_default("s3");

// Use default disk
let disk = manager.disk_default()?;
disk.put("file.txt", b"Hello".to_vec()).await?;

// Use specific disk
let local = manager.disk("local")?;
local.put("file.txt", b"Hello".to_vec()).await?;
```

### Private Channels
```rust
use rf_broadcasting::{Broadcast, ChannelType};

#[derive(Debug, Clone)]
struct UserNotification {
    user_id: u64,
    message: String,
}

impl Broadcast for UserNotification {
    fn broadcast_on(&self) -> Vec<String> {
        vec![format!("private-user.{}", self.user_id)]
    }

    fn broadcast_with(&self) -> serde_json::Value {
        json!({ "message": self.message })
    }
}

// Client must provide auth token when subscribing
// ws.send(JSON.stringify({
//     command: 'subscribe',
//     channel: 'private-user.123',
//     auth: 'user-auth-token'
// }));
```

---

## 🧪 Testing

### Run All Tests
```bash
# S3 Storage tests
cargo test -p rf-storage

# Broadcasting tests
cargo test -p rf-broadcasting

# Both
cargo test -p rf-storage -p rf-broadcasting
```

### Run Integration Tests Only
```bash
# S3 integration tests (requires MinIO)
cargo test -p rf-storage --test s3_integration

# Broadcasting integration tests (requires Redis)
cargo test -p rf-broadcasting --test websocket_integration
```

### Run Examples
```bash
# S3 usage example
cargo run -p rf-storage --example s3_usage

# WebSocket server example
cargo run -p rf-broadcasting --example websocket_server
```

---

## 🐛 Troubleshooting

### S3 Connection Issues
```bash
# Check if MinIO is running
curl http://localhost:9000

# Check MinIO logs
docker logs <container-id>

# Test connection
aws s3 ls --endpoint-url http://localhost:9000
```

### Redis Connection Issues
```bash
# Check if Redis is running
redis-cli ping

# Check Redis logs
docker logs <container-id>

# Test connection
redis-cli -h localhost -p 6379
```

### WebSocket Connection Issues
```bash
# Check if port is in use
lsof -i :6001

# Test WebSocket connection
wscat -c ws://localhost:6001
```

---

## 📚 Additional Resources

- **S3 Storage API**: `crates/rf-storage/src/lib.rs`
- **Broadcasting API**: `crates/rf-broadcasting/src/lib.rs`
- **Full Examples**:
  - `crates/rf-storage/examples/s3_usage.rs`
  - `crates/rf-broadcasting/examples/websocket_server.rs`
- **Tests**:
  - `crates/rf-storage/tests/s3_integration.rs`
  - `crates/rf-broadcasting/tests/websocket_integration.rs`

---

## 🎯 Next Steps

1. ✅ Install dependencies
2. ✅ Run MinIO and Redis
3. ✅ Try the examples
4. ✅ Read the full report: `S3_BROADCASTING_REPORT.md`
5. ✅ Build your application!

Happy coding! 🚀
