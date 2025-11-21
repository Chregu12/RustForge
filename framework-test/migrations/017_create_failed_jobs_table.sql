-- Failed Jobs table (for tracking job failures)
CREATE TABLE IF NOT EXISTS failed_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection VARCHAR(255) NOT NULL,
    queue VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    exception TEXT NOT NULL,
    failed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_failed_jobs_queue ON failed_jobs(queue);
CREATE INDEX idx_failed_jobs_failed_at ON failed_jobs(failed_at);
