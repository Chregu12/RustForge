# ADR-005: Configuration Management

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Applikationen benötigen:
- Environment-Variable-Support (.env)
- Type-Safe Configuration
- Validation beim Startup (fail-fast)
- Multiple Environments (dev, staging, prod)

## Decision

**config + dotenvy** für hierarchische, type-safe Konfiguration

### API-Design:

```rust
use serde::Deserialize;
use config::{Config, Environment, File};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

impl AppConfig {
    pub fn load(env: &str) -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok(); // Load .env

        Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?
            .try_deserialize()
    }
}
```

### Hierarchie (Precedence):

1. **Environment Variables** (höchste Priorität)
   - `APP__SERVER__PORT=8080`
2. **Environment-spezifische Dateien**
   - `config/production.toml`
3. **Default Config**
   - `config/default.toml`

### Config-Dateien (TOML):

```toml
# config/default.toml
[server]
host = "127.0.0.1"
port = 3000
workers = 4

[database]
url = "postgres://localhost/myapp"
max_connections = 10

[redis]
url = "redis://localhost:6379"
pool_size = 10

[auth]
jwt_secret = "dev-secret-change-in-production"
token_expiry_hours = 24
```

```toml
# config/production.toml
[server]
host = "0.0.0.0"
workers = 16

[auth]
jwt_secret = "${JWT_SECRET}"  # Must be set via env var
```

### Alternativen (abgelehnt):

**figment:**
- ❌ Weniger aktive Entwicklung
- ❌ Komplexere API

**Pure dotenvy:**
- ❌ Keine Typsicherheit
- ❌ Keine Hierarchie
- ❌ String-Parsing manuell

## Consequences

**Positiv:**
- ✅ Type-Safe (Compiler prüft Config-Zugriffe)
- ✅ Fail-Fast (Invalid Config = Startup-Error)
- ✅ 12-Factor-App-konform
- ✅ Environment-Overrides einfach

**Negativ:**
- ❌ TOML-Syntax (nicht jeder kennt es)
- ❌ Secrets noch in Dateien (braucht Vault-Integration)

## Implementation

```rust
// rf-core/src/config.rs
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub cors: CorsConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // Load .env file
        dotenvy::dotenv().ok();

        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        let config = Config::builder()
            // Start with default config
            .add_source(File::with_name("config/default"))
            // Override with environment-specific config
            .add_source(File::with_name(&format!("config/{}", env)).required(false))
            // Override with environment variables (APP__DATABASE__URL=...)
            .add_source(
                Environment::with_prefix("APP")
                    .separator("__")
                    .try_parsing(true)
            )
            .build()?;

        config.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.auth.jwt_secret == "dev-secret-change-in-production" {
            if std::env::var("APP_ENV").unwrap_or_default() == "production" {
                return Err("JWT_SECRET must be set in production".to_string());
            }
        }

        if self.database.max_connections < 1 {
            return Err("database.max_connections must be >= 1".to_string());
        }

        Ok(())
    }
}

// Usage in main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;
    config.validate()?;

    info!("Starting server on {}:{}", config.server.host, config.server.port);

    // ...
}
```

### Secrets Management (Future):

```rust
// Integration mit HashiCorp Vault (später)
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

async fn load_secrets(config: &mut AppConfig) -> Result<()> {
    let client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address("https://vault.example.com")
            .build()?
    )?;

    config.auth.jwt_secret = client.read_secret("secret/jwt").await?;
    config.database.url = client.read_secret("secret/database").await?;

    Ok(())
}
```
