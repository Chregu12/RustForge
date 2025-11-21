-- Role User pivot table (for many-to-many relationships)
CREATE TABLE IF NOT EXISTS role_user (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    assigned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    UNIQUE(user_id, role_id)
);

CREATE INDEX idx_role_user_user_id ON role_user(user_id);
CREATE INDEX idx_role_user_role_id ON role_user(role_id);
