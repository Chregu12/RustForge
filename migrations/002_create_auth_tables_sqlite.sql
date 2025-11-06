-- Authentication Tables Migration (SQLite)
-- Created: 2025-11-05
-- Description: Creates tables for users, sessions, password resets, and email verifications

-- ============================================================
-- Users Table
-- ============================================================
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    email_verified_at TEXT,
    two_factor_secret TEXT, -- Encrypted TOTP secret
    two_factor_recovery_codes TEXT, -- JSON array of encrypted recovery codes
    remember_token TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for users
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_email_verified_at ON users(email_verified_at);
CREATE INDEX idx_users_created_at ON users(created_at);

-- ============================================================
-- Sessions Table
-- ============================================================
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    ip_address TEXT, -- IPv6 compatible
    user_agent TEXT,
    last_activity TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for sessions
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(token);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity);

-- ============================================================
-- Password Resets Table
-- ============================================================
CREATE TABLE password_resets (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    token TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for password resets
CREATE INDEX idx_password_resets_email ON password_resets(email);
CREATE INDEX idx_password_resets_token ON password_resets(token);
CREATE INDEX idx_password_resets_expires_at ON password_resets(expires_at);

-- ============================================================
-- Email Verifications Table
-- ============================================================
CREATE TABLE email_verifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for email verifications
CREATE INDEX idx_email_verifications_user_id ON email_verifications(user_id);
CREATE INDEX idx_email_verifications_token ON email_verifications(token);
CREATE INDEX idx_email_verifications_expires_at ON email_verifications(expires_at);

-- ============================================================
-- User Activity Log (Optional - for security audit trail)
-- ============================================================
CREATE TABLE user_activity_log (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    activity_type TEXT NOT NULL, -- login, logout, password_change, etc.
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT, -- JSON for additional context
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for activity log
CREATE INDEX idx_user_activity_user_id ON user_activity_log(user_id);
CREATE INDEX idx_user_activity_type ON user_activity_log(activity_type);
CREATE INDEX idx_user_activity_created_at ON user_activity_log(created_at);

-- ============================================================
-- Cleanup Job Helpers
-- ============================================================
-- View for expired sessions (useful for cleanup jobs)
CREATE VIEW expired_auth_records AS
SELECT id, 'session' AS record_type FROM sessions WHERE expires_at < datetime('now')
UNION ALL
SELECT id, 'password_reset' AS record_type FROM password_resets WHERE expires_at < datetime('now')
UNION ALL
SELECT id, 'email_verification' AS record_type FROM email_verifications WHERE expires_at < datetime('now');

-- ============================================================
-- Triggers: Update users.updated_at automatically
-- ============================================================
CREATE TRIGGER update_users_updated_at
    AFTER UPDATE ON users
    FOR EACH ROW
BEGIN
    UPDATE users SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER update_oauth_clients_updated_at
    AFTER UPDATE ON oauth_clients
    FOR EACH ROW
BEGIN
    UPDATE oauth_clients SET updated_at = datetime('now') WHERE id = NEW.id;
END;
