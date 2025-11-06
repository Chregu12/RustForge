# Quick Start: Database Persistence

## 30-Second Setup

### PostgreSQL

```bash
# 1. Create database
createdb rustforge_dev

# 2. Run migrations
psql -d rustforge_dev -f migrations/001_create_oauth_tables.sql
psql -d rustforge_dev -f migrations/002_create_auth_tables.sql

# 3. Configure
export DATABASE_URL="postgresql://user:pass@localhost:5432/rustforge_dev"
export OAUTH_STORAGE="database"
export AUTH_SESSION_STORAGE="database"
export JWT_SECRET=$(openssl rand -base64 32)
```

### SQLite (Development)

```bash
# 1. Run migrations
sqlite3 database.sqlite < migrations/001_create_oauth_tables_sqlite.sql
sqlite3 database.sqlite < migrations/002_create_auth_tables_sqlite.sql

# 2. Configure
export DATABASE_URL="sqlite:///./database.sqlite"
export OAUTH_STORAGE="database"
export AUTH_SESSION_STORAGE="database"
export JWT_SECRET=$(openssl rand -base64 32)
```

## Basic Usage

### OAuth2 Setup

```rust
use foundry_oauth_server::{OAuth2Server, OAuth2Config};
use foundry_oauth_server::repositories::PostgresClientRepository;
use sqlx::PgPool;

let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
let client_repo = PostgresClientRepository::new(pool);
let oauth_server = OAuth2Server::new(OAuth2Config::default(), client_repo);
```

### Auth Setup

```rust
use foundry_auth_scaffolding::repositories::{
    PostgresUserRepository,
    PostgresSessionRepository,
};
use sqlx::PgPool;

let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
let user_repo = PostgresUserRepository::new(pool.clone());
let session_repo = PostgresSessionRepository::new(pool);
```

## Common Operations

### Register OAuth2 Client

```rust
use foundry_oauth_server::models::Client;

let client = Client::new(
    "My App".to_string(),
    vec!["http://localhost:3000/callback".to_string()],
);

let stored = client_repo.store(client).await?;
// Save client.id and client.secret somewhere safe!
```

### Register User

```rust
use foundry_auth_scaffolding::{AuthService, RegisterData};

let auth_service = AuthService::new(AuthConfig::default());
let data = RegisterData {
    name: "John Doe".to_string(),
    email: "john@example.com".to_string(),
    password: "secure_password".to_string(),
    password_confirmation: "secure_password".to_string(),
};

let user = auth_service.register(data)?;
user_repo.create(user).await?;
```

### Authenticate User

```rust
use foundry_auth_scaffolding::Credentials;

let user = user_repo.find_by_email("john@example.com").await?.unwrap();
let credentials = Credentials {
    email: "john@example.com".to_string(),
    password: "secure_password".to_string(),
    remember: false,
};

auth_service.attempt(&user, &credentials)?;
let session = auth_service.create_session(&user, false);
session_repo.create_session(session).await?;
```

## Environment Variables

```env
# Required
DATABASE_URL=postgresql://user:pass@localhost:5432/rustforge
OAUTH_STORAGE=database
AUTH_SESSION_STORAGE=database
JWT_SECRET=<32-byte-base64-string>

# Optional
OAUTH_ACCESS_TOKEN_LIFETIME=3600
OAUTH_REFRESH_TOKEN_LIFETIME=2592000
AUTH_SESSION_LIFETIME=7200
AUTH_REQUIRE_EMAIL_VERIFICATION=false
```

## Maintenance

### Cleanup Expired Tokens (PostgreSQL)

```sql
DELETE FROM oauth_access_tokens WHERE expires_at < NOW();
DELETE FROM oauth_refresh_tokens WHERE expires_at < NOW();
DELETE FROM sessions WHERE expires_at < NOW();
```

### Backup Database

```bash
# PostgreSQL
pg_dump rustforge_dev > backup.sql

# SQLite
sqlite3 database.sqlite ".backup backup.sqlite"
```

## Troubleshooting

**Connection failed?**
- Check DATABASE_URL format
- Verify database is running
- Check credentials

**Migration failed?**
- Tables already exist? Drop them or use fresh database
- Permission denied? Check user permissions

**Tests failing?**
- Set DATABASE_URL for test database
- Run migrations on test database first

## Full Documentation

- Comprehensive guide: `DATABASE_PERSISTENCE_GUIDE.md`
- Migration details: `migrations/README.md`
- Implementation report: `DATABASE_MIGRATION_COMPLETE.md`

## Production Checklist

- [ ] Generate strong JWT_SECRET
- [ ] Enable database SSL/TLS
- [ ] Configure connection pooling
- [ ] Set up backups
- [ ] Schedule cleanup jobs
- [ ] Enable monitoring
- [ ] Review token lifetimes
