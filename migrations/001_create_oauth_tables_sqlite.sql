-- OAuth2 Tables Migration (SQLite)
-- Created: 2025-11-05
-- Description: Creates tables for OAuth2 clients, tokens, and authorization codes

-- ============================================================
-- OAuth Clients Table
-- ============================================================
CREATE TABLE oauth_clients (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    secret_hash TEXT, -- NULL for public clients (PKCE)
    redirect_uris TEXT NOT NULL, -- JSON array
    grants TEXT NOT NULL, -- JSON array of grant types
    scopes TEXT NOT NULL, -- JSON array of scopes
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for OAuth clients
CREATE INDEX idx_oauth_clients_name ON oauth_clients(name);
CREATE INDEX idx_oauth_clients_revoked ON oauth_clients(revoked);
CREATE INDEX idx_oauth_clients_created_at ON oauth_clients(created_at);

-- ============================================================
-- OAuth Access Tokens Table
-- ============================================================
CREATE TABLE oauth_access_tokens (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    user_id TEXT, -- NULL for client_credentials grant
    token TEXT NOT NULL UNIQUE,
    scopes TEXT NOT NULL, -- JSON array
    revoked INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for access tokens
CREATE INDEX idx_oauth_access_tokens_client_id ON oauth_access_tokens(client_id);
CREATE INDEX idx_oauth_access_tokens_user_id ON oauth_access_tokens(user_id);
CREATE INDEX idx_oauth_access_tokens_token ON oauth_access_tokens(token);
CREATE INDEX idx_oauth_access_tokens_expires_at ON oauth_access_tokens(expires_at);
CREATE INDEX idx_oauth_access_tokens_revoked ON oauth_access_tokens(revoked);

-- ============================================================
-- OAuth Refresh Tokens Table
-- ============================================================
CREATE TABLE oauth_refresh_tokens (
    id TEXT PRIMARY KEY,
    access_token_id TEXT NOT NULL REFERENCES oauth_access_tokens(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    revoked INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for refresh tokens
CREATE INDEX idx_oauth_refresh_tokens_access_token_id ON oauth_refresh_tokens(access_token_id);
CREATE INDEX idx_oauth_refresh_tokens_token ON oauth_refresh_tokens(token);
CREATE INDEX idx_oauth_refresh_tokens_expires_at ON oauth_refresh_tokens(expires_at);
CREATE INDEX idx_oauth_refresh_tokens_revoked ON oauth_refresh_tokens(revoked);

-- ============================================================
-- OAuth Authorization Codes Table
-- ============================================================
CREATE TABLE oauth_authorization_codes (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    code TEXT NOT NULL UNIQUE,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL, -- JSON array
    code_challenge TEXT, -- For PKCE
    code_challenge_method TEXT, -- plain or S256
    revoked INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for authorization codes
CREATE INDEX idx_oauth_auth_codes_client_id ON oauth_authorization_codes(client_id);
CREATE INDEX idx_oauth_auth_codes_user_id ON oauth_authorization_codes(user_id);
CREATE INDEX idx_oauth_auth_codes_code ON oauth_authorization_codes(code);
CREATE INDEX idx_oauth_auth_codes_expires_at ON oauth_authorization_codes(expires_at);
CREATE INDEX idx_oauth_auth_codes_revoked ON oauth_authorization_codes(revoked);

-- ============================================================
-- OAuth Personal Access Tokens Table
-- ============================================================
CREATE TABLE oauth_personal_access_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    scopes TEXT NOT NULL, -- JSON array
    revoked INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    expires_at TEXT, -- NULL means never expires
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for personal access tokens
CREATE INDEX idx_oauth_pat_user_id ON oauth_personal_access_tokens(user_id);
CREATE INDEX idx_oauth_pat_token ON oauth_personal_access_tokens(token);
CREATE INDEX idx_oauth_pat_revoked ON oauth_personal_access_tokens(revoked);
CREATE INDEX idx_oauth_pat_expires_at ON oauth_personal_access_tokens(expires_at);

-- ============================================================
-- Cleanup Job Helpers
-- ============================================================
-- View for expired tokens (useful for cleanup jobs)
CREATE VIEW oauth_expired_tokens AS
SELECT id, 'access_token' AS token_type FROM oauth_access_tokens WHERE expires_at < datetime('now')
UNION ALL
SELECT id, 'refresh_token' AS token_type FROM oauth_refresh_tokens WHERE expires_at < datetime('now')
UNION ALL
SELECT id, 'authorization_code' AS token_type FROM oauth_authorization_codes WHERE expires_at < datetime('now')
UNION ALL
SELECT id, 'personal_access_token' AS token_type FROM oauth_personal_access_tokens
WHERE expires_at IS NOT NULL AND expires_at < datetime('now');
