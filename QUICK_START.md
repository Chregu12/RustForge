# 🚀 RustForge Framework - Quick Start Guide

Dieses Dokument zeigt dir, wie du **sofort** ein neues Projekt mit RustForge erstellen kannst.

---

## 📦 Option 1: Via Git Dependencies (SOFORT VERFÜGBAR)

### 1. Neues Projekt erstellen

```bash
cargo new my-rustforge-app
cd my-rustforge-app
```

### 2. Cargo.toml anpassen

```toml
[package]
name = "my-rustforge-app"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge Framework
foundry-application = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-queue = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-cache = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-forms = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }

# Required dependencies
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
```

### 3. .env erstellen

```bash
APP_NAME=MyApp
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:8000

# Cache (memory für dev, redis für production)
CACHE_DRIVER=memory

# Queue (memory für dev, redis für production)
QUEUE_DRIVER=memory

# Redis (für production)
REDIS_URL=redis://127.0.0.1:6379
```

### 4. src/main.rs

```rust
use anyhow::Result;
use foundry_queue::{QueueManager, Job};
use foundry_cache::CacheManager;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting RustForge Application...");

    // Queue System
    let queue = QueueManager::from_env()?;

    // Dispatch a job
    let job = Job::new("send_welcome_email")
        .with_payload(json!({
            "email": "user@example.com",
            "name": "John Doe"
        }));

    queue.dispatch(job).await?;
    println!("✅ Job dispatched!");

    // Cache System
    let cache = CacheManager::from_env()?;

    cache.set("user:1", &"John Doe".to_string(), None).await?;

    if let Some(name) = cache.get::<String>("user:1").await? {
        println!("✅ Cached value: {}", name);
    }

    println!("✅ Application running successfully!");

    Ok(())
}
```

### 5. Build & Run

```bash
cargo build
cargo run
```

**Output:**
```
🚀 Starting RustForge Application...
✅ Job dispatched!
✅ Cached value: John Doe
✅ Application running successfully!
```

---

## 🎯 Option 2: Vollständiges Beispiel mit allen Features

### Projektstruktur

```
my-app/
├── Cargo.toml
├── .env
├── src/
│   ├── main.rs
│   ├── models/
│   │   └── user.rs
│   ├── jobs/
│   │   └── send_email.rs
│   └── middleware/
│       └── auth.rs
└── tests/
    └── integration_test.rs
```

### src/main.rs (Vollversion)

```rust
use anyhow::Result;
use foundry_queue::{QueueManager, Job};
use foundry_cache::CacheManager;
use foundry_forms::validation::*;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 RustForge Application Starting...\n");

    // =====================================================================
    // 1. QUEUE SYSTEM
    // =====================================================================
    println!("📬 Testing Queue System...");
    let queue = QueueManager::from_env()?;

    // Dispatch welcome email job
    let job = Job::new("send_welcome_email")
        .with_payload(json!({
            "to": "user@example.com",
            "subject": "Welcome!",
            "body": "Thank you for joining!"
        }));

    queue.dispatch(job).await?;
    println!("   ✅ Welcome email job dispatched\n");

    // =====================================================================
    // 2. CACHE SYSTEM
    // =====================================================================
    println!("💾 Testing Cache System...");
    let cache = CacheManager::from_env()?;

    // Cache user object
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    cache.set("user:1", &user, Some(Duration::from_secs(3600))).await?;
    println!("   ✅ User cached with 1 hour TTL");

    // Retrieve from cache
    if let Some(cached_user) = cache.get::<User>("user:1").await? {
        println!("   ✅ Retrieved: {} ({})", cached_user.name, cached_user.email);
    }
    println!();

    // =====================================================================
    // 3. VALIDATION SYSTEM
    // =====================================================================
    println!("🔍 Testing Validation System...");

    let mut form_data = HashMap::new();
    form_data.insert("email".to_string(), "alice@example.com".to_string());
    form_data.insert("username".to_string(), "alice123".to_string());
    form_data.insert("age".to_string(), "25".to_string());

    let data = ValidationData::from(form_data);

    let result = Validator::new(data)
        .rule("email", vec![required(), email()])
        .rule("username", vec![required(), alpha_numeric(), min_length(3)])
        .rule("age", vec![required(), numeric(), min(18.0)])
        .validate();

    match result {
        Ok(_) => println!("   ✅ Form validation passed!"),
        Err(e) => println!("   ❌ Validation errors: {}", e),
    }
    println!();

    // =====================================================================
    // SUMMARY
    // =====================================================================
    println!("═══════════════════════════════════════════════════");
    println!("✅ All systems operational!");
    println!("═══════════════════════════════════════════════════");
    println!("   • Queue: Jobs dispatched successfully");
    println!("   • Cache: Data cached and retrieved");
    println!("   • Validation: Form validation working");
    println!("═══════════════════════════════════════════════════\n");

    Ok(())
}
```

### Cargo.toml (Vollversion)

```toml
[package]
name = "my-rustforge-app"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge Framework
foundry-application = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-queue = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-cache = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-forms = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-oauth = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }
foundry-mail = { git = "https://github.com/Chregu12/RustForge.git", tag = "v0.2.0" }

# Async Runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"

# HTTP (optional)
axum = "0.7"
```

---

## 🔧 Erweiterte Features

### Jobs erstellen (Background Processing)

```rust
// src/jobs/send_email.rs
use foundry_queue::Job;
use serde_json::json;

pub fn create_welcome_email_job(email: &str, name: &str) -> Job {
    Job::new("send_welcome_email")
        .with_payload(json!({
            "email": email,
            "name": name,
            "template": "welcome"
        }))
        .with_priority(5)  // Higher priority
}

pub fn create_report_job(user_id: u64) -> Job {
    Job::new("generate_report")
        .with_payload(json!({
            "user_id": user_id
        }))
        .with_delay(std::time::Duration::from_secs(3600))  // Delay 1 hour
}
```

### Validation Forms erstellen

```rust
// src/forms/user_registration.rs
use foundry_forms::validation::*;
use std::collections::HashMap;

pub struct UserRegistrationForm {
    pub email: String,
    pub password: String,
    pub age: i64,
}

impl UserRegistrationForm {
    pub fn validate(data: HashMap<String, String>) -> Result<Self, ValidationErrors> {
        let validation_data = ValidationData::from(data);

        Validator::new(validation_data.clone())
            .rule("email", vec![required(), email()])
            .rule("password", vec![required(), min_length(8)])
            .rule("age", vec![required(), numeric(), min(18.0)])
            .validate()?;

        Ok(Self {
            email: validation_data.get("email").unwrap().to_string(),
            password: validation_data.get("password").unwrap().to_string(),
            age: validation_data.get("age").unwrap().parse().unwrap(),
        })
    }
}
```

### Cache mit Tagged Keys

```rust
use foundry_cache::CacheManager;
use std::time::Duration;

async fn cache_user_data(cache: &CacheManager, user_id: u64) -> anyhow::Result<()> {
    // Cache user profile
    cache.set(
        &format!("user:{}:profile", user_id),
        &user_profile,
        Some(Duration::from_secs(3600))
    ).await?;

    // Cache user permissions
    cache.set(
        &format!("user:{}:permissions", user_id),
        &permissions,
        Some(Duration::from_secs(1800))
    ).await?;

    Ok(())
}
```

---

## 📊 Production Setup

### .env (Production)

```bash
APP_ENV=production
APP_DEBUG=false
APP_URL=https://myapp.com

# Redis für Production
CACHE_DRIVER=redis
QUEUE_DRIVER=redis
REDIS_URL=redis://prod-redis:6379

# Database
DATABASE_URL=postgresql://user:pass@localhost/myapp

# Email
MAIL_DRIVER=smtp
MAIL_HOST=smtp.mailtrap.io
MAIL_PORT=587
```

### Redis Setup

```bash
# Install Redis
brew install redis  # macOS
apt-get install redis-server  # Ubuntu

# Start Redis
redis-server

# Test Redis
redis-cli ping  # Should return "PONG"
```

### Queue Worker starten

```rust
// src/bin/worker.rs
use foundry_queue::{QueueManager, Worker};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting queue worker...");

    let queue = QueueManager::from_env()?;
    let worker = Worker::new(queue);

    worker.run().await?;

    Ok(())
}
```

```bash
# Worker starten
cargo run --bin worker
```

---

## 🧪 Tests schreiben

```rust
// tests/integration_test.rs
use foundry_cache::CacheManager;
use foundry_queue::QueueManager;

#[tokio::test]
async fn test_cache_system() {
    let cache = CacheManager::from_env().unwrap();

    cache.set("test_key", &"test_value".to_string(), None).await.unwrap();

    let value = cache.get::<String>("test_key").await.unwrap();

    assert_eq!(value, Some("test_value".to_string()));
}

#[tokio::test]
async fn test_queue_dispatch() {
    let queue = QueueManager::from_env().unwrap();

    let job = foundry_queue::Job::new("test_job");

    queue.dispatch(job).await.unwrap();

    // Job sollte dispatched sein
}
```

```bash
# Tests ausführen
cargo test
```

---

## 📚 Weitere Ressourcen

- **Framework Repository:** https://github.com/Chregu12/RustForge
- **CHANGELOG:** [CHANGELOG.md](CHANGELOG.md)
- **Architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Features:** [docs/FEATURES.md](docs/FEATURES.md)

---

## 🆘 Troubleshooting

### Build Fehler

```bash
# Dependencies aktualisieren
cargo update

# Clean build
cargo clean
cargo build
```

### Redis Connection Error

```bash
# Redis Status prüfen
redis-cli ping

# Redis starten
redis-server

# Connection in .env prüfen
REDIS_URL=redis://127.0.0.1:6379
```

### Tokio Runtime Error

```rust
// Stelle sicher dass main() async ist
#[tokio::main]
async fn main() -> Result<()> {
    // ...
}
```

---

## ✅ Nächste Schritte

1. ✅ Basic App läuft
2. 📝 Eigene Jobs erstellen
3. 📝 Validation Forms definieren
4. 📝 Redis für Production einrichten
5. 🚀 Deployen!

---

**Viel Erfolg mit RustForge! 🚀**

Fragen? → GitHub Issues: https://github.com/Chregu12/RustForge/issues
