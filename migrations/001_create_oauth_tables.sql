-- OAuth2 Tables Migration
-- Created: 2025-11-05
-- Description: Creates tables for OAuth2 clients, tokens, and authorization codes

-- ============================================================
-- OAuth Clients Table
-- ============================================================
CREATE TABLE oauth_clients (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    secret_hash VARCHAR(255), -- NULL for public clients (PKCE)
    redirect_uris TEXT NOT NULL, -- JSON array
    grants TEXT NOT NULL, -- JSON array of grant types
    scopes TEXT NOT NULL, -- JSON array of scopes
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for OAuth clients
CREATE INDEX idx_oauth_clients_name ON oauth_clients(name);
CREATE INDEX idx_oauth_clients_revoked ON oauth_clients(revoked);
CREATE INDEX idx_oauth_clients_created_at ON oauth_clients(created_at);

-- ============================================================
-- OAuth Access Tokens Table
-- ============================================================
CREATE TABLE oauth_access_tokens (
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    user_id UUID, -- NULL for client_credentials grant
    token TEXT NOT NULL UNIQUE,
    scopes TEXT NOT NULL, -- JSON array
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
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
    id UUID PRIMARY KEY,
    access_token_id UUID NOT NULL REFERENCES oauth_access_tokens(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
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
    id UUID PRIMARY KEY,
    client_id UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    code VARCHAR(255) NOT NULL UNIQUE,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL, -- JSON array
    code_challenge VARCHAR(255), -- For PKCE
    code_challenge_method VARCHAR(10), -- plain or S256
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
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
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    token TEXT NOT NULL UNIQUE,
    scopes TEXT NOT NULL, -- JSON array
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE, -- NULL means never expires
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
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
SELECT id, 'access_token' AS token_type FROM oauth_access_tokens WHERE expires_at < NOW()
UNION ALL
SELECT id, 'refresh_token' AS token_type FROM oauth_refresh_tokens WHERE expires_at < NOW()
UNION ALL
SELECT id, 'authorization_code' AS token_type FROM oauth_authorization_codes WHERE expires_at < NOW()
UNION ALL
SELECT id, 'personal_access_token' AS token_type FROM oauth_personal_access_tokens
WHERE expires_at IS NOT NULL AND expires_at < NOW();
