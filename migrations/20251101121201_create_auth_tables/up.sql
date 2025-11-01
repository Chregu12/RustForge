-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    password_hash TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    email_verified_at TIMESTAMP NULL,
    remember_token VARCHAR(100) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index on email for faster lookups
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_active ON users(is_active);

-- Create sessions table for persistent sessions
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(255) PRIMARY KEY,
    user_id INTEGER NULL,
    ip_address VARCHAR(45) NULL,
    user_agent TEXT NULL,
    payload TEXT NOT NULL,
    last_activity TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create index on user_id and last_activity for session cleanup
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_last_activity ON sessions(last_activity);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

-- Create roles table
CREATE TABLE IF NOT EXISTS roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL UNIQUE,
    slug VARCHAR(255) NOT NULL UNIQUE,
    description TEXT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index on role slug for faster lookups
CREATE INDEX idx_roles_slug ON roles(slug);

-- Create permissions table
CREATE TABLE IF NOT EXISTS permissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL UNIQUE,
    slug VARCHAR(255) NOT NULL UNIQUE,
    description TEXT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index on permission slug
CREATE INDEX idx_permissions_slug ON permissions(slug);

-- Create role_user pivot table (many-to-many)
CREATE TABLE IF NOT EXISTS role_user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    UNIQUE(user_id, role_id)
);

-- Create indexes for role_user
CREATE INDEX idx_role_user_user_id ON role_user(user_id);
CREATE INDEX idx_role_user_role_id ON role_user(role_id);

-- Create permission_role pivot table (many-to-many)
CREATE TABLE IF NOT EXISTS permission_role (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    role_id INTEGER NOT NULL,
    permission_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE,
    UNIQUE(role_id, permission_id)
);

-- Create indexes for permission_role
CREATE INDEX idx_permission_role_role_id ON permission_role(role_id);
CREATE INDEX idx_permission_role_permission_id ON permission_role(permission_id);

-- Create refresh_tokens table for JWT token rotation
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    token VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMP NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create indexes for refresh_tokens
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_token ON refresh_tokens(token);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);

-- Insert default roles
INSERT INTO roles (name, slug, description) VALUES
    ('Administrator', 'admin', 'Full system access'),
    ('User', 'user', 'Standard user access'),
    ('Guest', 'guest', 'Limited access');

-- Insert default permissions
INSERT INTO permissions (name, slug, description) VALUES
    ('View Dashboard', 'dashboard.view', 'Can view the dashboard'),
    ('Manage Users', 'users.manage', 'Can create, update, and delete users'),
    ('View Users', 'users.view', 'Can view user list'),
    ('Manage Roles', 'roles.manage', 'Can create, update, and delete roles'),
    ('View Roles', 'roles.view', 'Can view role list'),
    ('Manage Permissions', 'permissions.manage', 'Can assign permissions to roles'),
    ('View Settings', 'settings.view', 'Can view system settings'),
    ('Manage Settings', 'settings.manage', 'Can modify system settings');

-- Assign all permissions to admin role
INSERT INTO permission_role (role_id, permission_id)
SELECT
    (SELECT id FROM roles WHERE slug = 'admin'),
    id
FROM permissions;

-- Assign limited permissions to user role
INSERT INTO permission_role (role_id, permission_id)
SELECT
    (SELECT id FROM roles WHERE slug = 'user'),
    id
FROM permissions
WHERE slug IN ('dashboard.view', 'users.view', 'settings.view');
