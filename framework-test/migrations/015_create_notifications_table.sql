-- Notifications table (for database notification channel)
CREATE TABLE IF NOT EXISTS notifications (
    id VARCHAR(36) PRIMARY KEY,
    type VARCHAR(255) NOT NULL,
    notifiable_type VARCHAR(255) NOT NULL,
    notifiable_id INTEGER NOT NULL,
    data TEXT NOT NULL,
    read_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_notifiable ON notifications(notifiable_type, notifiable_id);
CREATE INDEX idx_notifications_read_at ON notifications(read_at);
