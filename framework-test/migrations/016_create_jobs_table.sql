-- Jobs table (for queue system)
CREATE TABLE IF NOT EXISTS jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    queue VARCHAR(255) NOT NULL DEFAULT 'default',
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    reserved_at TIMESTAMP NULL,
    available_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_jobs_queue ON jobs(queue);
CREATE INDEX idx_jobs_available_at ON jobs(available_at);
CREATE INDEX idx_jobs_reserved_at ON jobs(reserved_at);
