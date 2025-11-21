# RustForge Error Codes Reference

Complete reference for all RustForge error codes, common causes, and solutions.

## Quick Reference

| Code Range | Category | Description |
|------------|----------|-------------|
| RF001-RF099 | Database | Database connection, queries, migrations |
| RF100-RF199 | Validation | Input validation and constraints |
| RF200-RF299 | Authentication | Login, tokens, user authentication |
| RF300-RF399 | Authorization | Permissions, roles, policies |
| RF400-RF499 | Cache | Cache operations and connections |
| RF500-RF599 | Queue | Job queues and background tasks |
| RF600-RF699 | HTTP | HTTP requests and routing |
| RF700-RF799 | Template | Template rendering and compilation |
| RF800-RF849 | Storage | File storage and S3 operations |
| RF850-RF899 | Mail | Email sending and SMTP |
| RF900-RF999 | General | Configuration and system errors |

---

## Database Errors (RF001-RF099)

### RF001: Database Connection Failed

**Description:** Unable to establish connection to the database server.

**Common Causes:**
- Database server is not running
- Incorrect credentials in `.env` file
- Network/firewall blocking connection
- Database doesn't exist
- Connection pool exhausted

**Solutions:**
```bash
# 1. Check if PostgreSQL is running
systemctl status postgresql
# or for macOS:
brew services list

# 2. Verify DATABASE_URL in .env
DATABASE_URL=postgres://user:password@localhost:5432/dbname

# 3. Test connection manually
psql -h localhost -U postgres -d dbname

# 4. Check firewall rules
sudo ufw status  # Linux
```

**Configuration:**
```toml
[database]
url = "postgres://user:password@localhost:5432/dbname"
max_connections = 10
timeout = 30
```

---

### RF002: Database Query Failed

**Description:** SQL query execution failed.

**Common Causes:**
- SQL syntax error
- Table or column doesn't exist
- Type mismatch
- Constraint violation

**Solutions:**
```rust
// 1. Enable query logging
tracing::debug!("Executing query: {}", query);

// 2. Check table/column names
// 3. Verify data types match schema
// 4. Check for constraint violations
```

---

### RF003: Database Migration Failed

**Description:** Database migration could not be applied.

**Common Causes:**
- Migration file is corrupt
- Schema conflicts
- Insufficient permissions

**Solutions:**
```bash
# 1. Check migration status
cargo run -- migrate status

# 2. Rollback last migration
cargo run -- migrate rollback

# 3. Run migrations fresh (dev only!)
cargo run -- migrate fresh
```

---

## Validation Errors (RF100-RF199)

### RF100: Validation Failed

**Description:** Input validation failed for one or more fields.

**Example:**
```rust
use rf_validation::Validator;

let validator = Validator::new()
    .field("email")
    .required()
    .email()
    .validate(&data)?;
```

---

### RF101: Field Required

**Description:** Required field is missing.

**Solutions:**
- Ensure all required fields are provided
- Check form submission
- Verify API request payload

---

### RF102: Invalid Email

**Description:** Email format is invalid.

**Solutions:**
```rust
// Valid formats:
"user@example.com"
"user+tag@example.co.uk"

// Invalid formats:
"user@"
"@example.com"
"user"
```

---

### RF104: Value Already Exists (Unique Constraint)

**Description:** Unique constraint violation (e.g., email already registered).

**Solutions:**
```rust
// Check if value exists before creating
if User::where("email", email).exists().await? {
    return Err(ValidationError::unique("email"));
}
```

---

### RF105: Referenced Entity Not Found (Foreign Key)

**Description:** Foreign key reference doesn't exist.

**Solutions:**
```rust
// Verify referenced entity exists
if !Role::find(role_id).await?.exists() {
    return Err(ValidationError::exists("role_id"));
}
```

---

## Authentication Errors (RF200-RF299)

### RF200: Authentication Failed

**Description:** Authentication process failed.

---

### RF201: Invalid Credentials

**Description:** Username or password is incorrect.

**Common Causes:**
- Wrong username/email
- Wrong password
- Account doesn't exist

**Solutions:**
- Verify credentials
- Use password reset if needed
- Check account status

---

### RF202: Token Expired

**Description:** Authentication token has expired.

**Solutions:**
```rust
// Refresh token or re-authenticate
auth.refresh_token().await?;
// or
auth.login(credentials).await?;
```

**Configuration:**
```toml
[auth]
token_lifetime = 3600  # 1 hour
refresh_token_lifetime = 2592000  # 30 days
```

---

### RF206: Email Not Verified

**Description:** User must verify email before accessing resources.

**Solutions:**
```rust
// Resend verification email
mail.send_verification(user).await?;

// Check verification status
if !user.email_verified_at.is_some() {
    return Err(AuthError::email_not_verified());
}
```

---

## Authorization Errors (RF300-RF399)

### RF300: Access Forbidden

**Description:** User doesn't have permission to access resource.

**Solutions:**
```rust
// Check permissions
if !user.can("edit-posts") {
    return Err(AuthorizationError::forbidden("edit-posts"));
}

// Check policy
if !gate.authorize(user, "update", post).await? {
    return Err(AuthorizationError::forbidden("update post"));
}
```

---

### RF301: Insufficient Permissions

**Description:** User lacks required permission.

**Solutions:**
- Verify user has correct role
- Check permission assignments
- Review policy rules

---

## Cache Errors (RF400-RF499)

### RF400: Cache Connection Failed

**Description:** Unable to connect to cache server (Redis).

**Common Causes:**
- Redis not running
- Wrong connection URL
- Network issues

**Solutions:**
```bash
# 1. Check Redis status
redis-cli ping
# Should return: PONG

# 2. Verify REDIS_URL in .env
REDIS_URL=redis://localhost:6379

# 3. Test connection
redis-cli -h localhost -p 6379 ping
```

---

## Queue Errors (RF500-RF599)

### RF500: Queue Connection Failed

**Description:** Unable to connect to queue backend.

**Solutions:**
Same as RF400 (Redis-based queue)

---

### RF501: Job Dispatch Failed

**Description:** Failed to dispatch job to queue.

**Solutions:**
```rust
// Check queue connection
queue.health_check().await?;

// Retry dispatch
queue.dispatch(job).retry(3).await?;
```

---

### RF502: Job Failed

**Description:** Background job execution failed.

**Solutions:**
```rust
// Implement retry logic
impl Job for MyJob {
    fn max_retries(&self) -> u32 {
        3
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(60)
    }
}

// Check job logs
tail -f storage/logs/queue.log
```

---

## HTTP Errors (RF600-RF699)

### RF602: Route Not Found

**Description:** Requested route doesn't exist (404).

**Solutions:**
- Verify route is registered
- Check URL spelling
- Review route definitions

```rust
// Register route
app.get("/api/users", handlers::users::index);

// Check registered routes
cargo run -- route:list
```

---

### RF604: Rate Limit Exceeded

**Description:** Too many requests from client (429).

**Solutions:**
```rust
// Configure rate limiting
use rf_ratelimit::RateLimiter;

app.middleware(
    RateLimiter::new()
        .max_requests(60)
        .per_minutes(1)
);
```

**Client-side:**
- Implement exponential backoff
- Cache responses
- Reduce request frequency

---

## Template Errors (RF700-RF799)

### RF700: Template Not Found

**Description:** Blade template file not found.

**Solutions:**
```bash
# 1. Check template path
ls resources/views/errors/404.blade.php

# 2. Verify template name
view("errors.404")  # looks for errors/404.blade.php
```

---

### RF701: Template Rendering Failed

**Description:** Template rendering encountered an error.

**Common Causes:**
- Undefined variable
- Syntax error
- Missing partial

**Solutions:**
```blade
{{-- Check variable exists --}}
@if(isset($user))
    {{ $user->name }}
@endif

{{-- Provide default --}}
{{ $user->name ?? 'Guest' }}
```

---

## Storage Errors (RF800-RF899)

### RF800: Storage Connection Failed

**Description:** Unable to connect to storage backend (S3, local disk).

**Solutions:**
```toml
[storage]
driver = "s3"
bucket = "my-bucket"
region = "us-east-1"
key = "YOUR_ACCESS_KEY"
secret = "YOUR_SECRET_KEY"
```

---

### RF801: File Not Found

**Description:** Requested file doesn't exist in storage.

**Solutions:**
```rust
// Check if file exists
if !storage.exists("path/to/file").await? {
    return Err(StorageError::not_found("path/to/file"));
}

// Handle missing files
match storage.get("path/to/file").await {
    Ok(file) => Ok(file),
    Err(StorageError::FileNotFound(_)) => {
        // Use default or placeholder
        storage.get("defaults/placeholder.jpg").await
    }
}
```

---

## Mail Errors (RF850-RF899)

### RF850: Mail Server Connection Failed

**Description:** Unable to connect to SMTP server.

**Solutions:**
```toml
[mail]
driver = "smtp"
host = "smtp.gmail.com"
port = 587
username = "your-email@gmail.com"
password = "your-app-password"
encryption = "tls"
```

**Gmail Users:**
- Enable "Less secure app access" or
- Use App-specific password

---

### RF851: Mail Send Failed

**Description:** Failed to send email.

**Common Causes:**
- SMTP authentication failed
- Recipient doesn't exist
- Attachment too large
- Rate limit exceeded

**Solutions:**
```rust
// Use queue for reliability
Mail::to(user)
    .queue(WelcomeEmail::new(user))
    .await?;

// Add retry logic
Mail::to(user)
    .send(email)
    .retry(3)
    .await?;
```

---

## General Errors (RF900-RF999)

### RF900: Configuration Error

**Description:** Application configuration is invalid or missing.

**Solutions:**
```bash
# 1. Copy .env.example
cp .env.example .env

# 2. Generate app key
cargo run -- key:generate

# 3. Verify all required vars
cargo run -- config:validate
```

---

### RF901: Environment Variable Missing

**Description:** Required environment variable is not set.

**Solutions:**
```bash
# Check .env file
cat .env | grep VARIABLE_NAME

# Set temporarily
export VARIABLE_NAME=value

# Add to .env
echo "VARIABLE_NAME=value" >> .env
```

---

### RF902: Internal Server Error

**Description:** Unexpected internal error occurred.

**What to do:**
1. Check logs: `tail -f storage/logs/app.log`
2. Note the Request ID from error page
3. Search logs for Request ID
4. Review stack trace
5. Report to development team if needed

---

## Troubleshooting Guide

### General Debugging Steps

1. **Check Logs**
```bash
# Application logs
tail -f storage/logs/app.log

# Database logs
tail -f storage/logs/database.log

# Queue logs
tail -f storage/logs/queue.log
```

2. **Enable Debug Mode**
```bash
# .env
APP_ENV=development
APP_DEBUG=true
LOG_LEVEL=debug
```

3. **Test Components**
```bash
# Test database connection
cargo run -- db:test

# Test cache connection
cargo run -- cache:test

# Test mail configuration
cargo run -- mail:test
```

4. **Check Dependencies**
```bash
# PostgreSQL
systemctl status postgresql

# Redis
redis-cli ping

# Workers
ps aux | grep worker
```

### Getting Help

- **Documentation**: https://docs.rustforge.dev
- **GitHub Issues**: https://github.com/rustforge/rustforge/issues
- **Discord**: https://discord.gg/rustforge
- **Stack Overflow**: Tag with `rustforge`

### Reporting Errors

When reporting errors, include:
1. Error code (e.g., RF001)
2. Request ID from error page
3. Steps to reproduce
4. Environment (OS, Rust version, RustForge version)
5. Relevant logs (sanitize sensitive data!)
6. Configuration (without secrets)

---

**Last Updated:** November 2025
**Version:** 1.0.0
