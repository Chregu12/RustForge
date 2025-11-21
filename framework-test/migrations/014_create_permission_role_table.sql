-- Permission Role pivot table
CREATE TABLE IF NOT EXISTS permission_role (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    permission_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    UNIQUE(permission_id, role_id)
);

CREATE INDEX idx_permission_role_permission_id ON permission_role(permission_id);
CREATE INDEX idx_permission_role_role_id ON permission_role(role_id);
