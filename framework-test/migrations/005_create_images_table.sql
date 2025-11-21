-- Images table (for MorphOne, MorphMany polymorphic relationships)
CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    imageable_type VARCHAR(255) NOT NULL,
    imageable_id INTEGER NOT NULL,
    url VARCHAR(500) NOT NULL,
    filename VARCHAR(255) NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    size INTEGER NOT NULL,
    width INTEGER NULL,
    height INTEGER NULL,
    is_featured BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_images_imageable ON images(imageable_type, imageable_id);
CREATE INDEX idx_images_is_featured ON images(is_featured);
