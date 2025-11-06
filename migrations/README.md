# Database Migrations

This directory contains SQL migrations for the RustForge framework's OAuth2 and Authentication systems.

## Migration Files

### PostgreSQL Migrations

- `001_create_oauth_tables.sql` - OAuth2 tables (clients, tokens, authorization codes, personal access tokens)
- `002_create_auth_tables.sql` - Authentication tables (users, sessions, password resets, email verifications)

### SQLite Migrations

- `001_create_oauth_tables_sqlite.sql` - OAuth2 tables (SQLite compatible)
- `002_create_auth_tables_sqlite.sql` - Authentication tables (SQLite compatible)

## Running Migrations

### PostgreSQL

```bash
# Connect to your PostgreSQL database
psql -U username -d database_name

# Run migrations
\i migrations/001_create_oauth_tables.sql
\i migrations/002_create_auth_tables.sql
```

Or using a migration tool:

```bash
# Using sqlx-cli
sqlx migrate run --database-url "postgresql://username:password@localhost:5432/database_name"
```

### SQLite

```bash
# Using sqlite3 CLI
sqlite3 database.sqlite < migrations/001_create_oauth_tables_sqlite.sql
sqlite3 database.sqlite < migrations/002_create_auth_tables_sqlite.sql
```

Or using sqlx:

```bash
# Using sqlx-cli
sqlx migrate run --database-url "sqlite:///./database.sqlite"
```

## Table Structure

### OAuth2 Tables

1. **oauth_clients** - OAuth2 client applications
   - Stores confidential and public clients
   - Includes redirect URIs, grants, and scopes
   - Secret hashed with Argon2

2. **oauth_access_tokens** - Access tokens
   - JWT tokens for API access
   - Linked to clients and optionally users
   - Includes expiration and revocation

3. **oauth_refresh_tokens** - Refresh tokens
   - Long-lived tokens for obtaining new access tokens
   - Linked to access tokens (CASCADE delete)

4. **oauth_authorization_codes** - Authorization codes
   - Short-lived codes for authorization code flow
   - Supports PKCE (code_challenge fields)

5. **oauth_personal_access_tokens** - Personal access tokens
   - Long-lived tokens for user API access
   - No expiration by default

### Authentication Tables

1. **users** - User accounts
   - Email/password authentication
   - Email verification tracking
   - Two-factor authentication support

2. **sessions** - User sessions
   - Token-based session tracking
   - IP address and user agent logging
   - Automatic expiration

3. **password_resets** - Password reset tokens
   - Time-limited reset tokens
   - Email-based password recovery

4. **email_verifications** - Email verification tokens
   - Time-limited verification tokens
   - User email confirmation

5. **user_activity_log** - Security audit log
   - Tracks user authentication events
   - Useful for security monitoring

## Indexes

All tables include appropriate indexes for:
- Primary keys (UUID)
- Foreign keys
- Frequently queried fields (tokens, emails, expiration dates)
- Composite indexes for common queries

## Views

### oauth_expired_tokens
Convenience view for cleanup jobs that identifies expired:
- Access tokens
- Refresh tokens
- Authorization codes
- Personal access tokens

### expired_auth_records
Convenience view for cleanup jobs that identifies expired:
- Sessions
- Password reset tokens
- Email verification tokens

## Cleanup Jobs

Consider setting up periodic cleanup jobs to remove expired records:

```sql
-- PostgreSQL cleanup example
DELETE FROM oauth_access_tokens WHERE expires_at < NOW();
DELETE FROM oauth_authorization_codes WHERE expires_at < NOW();
DELETE FROM sessions WHERE expires_at < NOW();
DELETE FROM password_resets WHERE expires_at < NOW();
DELETE FROM email_verifications WHERE expires_at < NOW();
```

For production, use a cron job or scheduled task:

```bash
# Example cron job (daily at 2 AM)
0 2 * * * psql -U username -d database_name -c "DELETE FROM oauth_access_tokens WHERE expires_at < NOW();"
```

## Rollback

To rollback migrations (drop all tables):

### PostgreSQL

```sql
DROP VIEW IF EXISTS oauth_expired_tokens;
DROP VIEW IF EXISTS expired_auth_records;
DROP TABLE IF EXISTS oauth_personal_access_tokens CASCADE;
DROP TABLE IF EXISTS oauth_authorization_codes CASCADE;
DROP TABLE IF EXISTS oauth_refresh_tokens CASCADE;
DROP TABLE IF EXISTS oauth_access_tokens CASCADE;
DROP TABLE IF EXISTS oauth_clients CASCADE;
DROP TABLE IF EXISTS user_activity_log CASCADE;
DROP TABLE IF EXISTS email_verifications CASCADE;
DROP TABLE IF EXISTS password_resets CASCADE;
DROP TABLE IF EXISTS sessions CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP FUNCTION IF EXISTS update_updated_at_column();
```

### SQLite

```sql
DROP VIEW IF EXISTS oauth_expired_tokens;
DROP VIEW IF EXISTS expired_auth_records;
DROP TABLE IF EXISTS oauth_personal_access_tokens;
DROP TABLE IF EXISTS oauth_authorization_codes;
DROP TABLE IF EXISTS oauth_refresh_tokens;
DROP TABLE IF EXISTS oauth_access_tokens;
DROP TABLE IF EXISTS oauth_clients;
DROP TABLE IF EXISTS user_activity_log;
DROP TABLE IF EXISTS email_verifications;
DROP TABLE IF EXISTS password_resets;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;
```

## Environment Variables

Make sure your `.env` file is configured:

```env
DATABASE_URL=postgresql://username:password@localhost:5432/database_name
# Or for SQLite:
DATABASE_URL=sqlite:///./database.sqlite

OAUTH_STORAGE=database
AUTH_SESSION_STORAGE=database
```

## Testing

For running tests with a test database:

```bash
# Create test database
createdb rustforge_test

# Run migrations
psql -U username -d rustforge_test < migrations/001_create_oauth_tables.sql
psql -U username -d rustforge_test < migrations/002_create_auth_tables.sql

# Run tests
DATABASE_URL=postgresql://username:password@localhost:5432/rustforge_test cargo test
```

## Security Notes

1. **Never commit production database credentials** to version control
2. **Use strong JWT secrets** in production (generate with `openssl rand -base64 32`)
3. **Enable SSL/TLS** for database connections in production
4. **Regularly rotate secrets** for client credentials and JWT signing
5. **Monitor the activity log** for suspicious authentication attempts
6. **Set up automated cleanup** to remove expired tokens and sessions
