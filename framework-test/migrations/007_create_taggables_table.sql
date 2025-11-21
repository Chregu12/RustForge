-- Taggables pivot table (for MorphToMany polymorphic relationships)
CREATE TABLE IF NOT EXISTS taggables (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tag_id INTEGER NOT NULL,
    taggable_type VARCHAR(255) NOT NULL,
    taggable_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    UNIQUE(tag_id, taggable_type, taggable_id)
);

CREATE INDEX idx_taggables_tag_id ON taggables(tag_id);
CREATE INDEX idx_taggables_taggable ON taggables(taggable_type, taggable_id);
