-- Cache table (for database cache driver)
CREATE TABLE IF NOT EXISTS cache (
    key VARCHAR(255) PRIMARY KEY,
    value TEXT NOT NULL,
    expiration INTEGER NOT NULL
);

CREATE INDEX idx_cache_expiration ON cache(expiration);
