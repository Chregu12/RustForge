INSERT INTO posts (title, body)
SELECT 'Welcome to RustForge', 'This post came from the database seeder.'
WHERE NOT EXISTS (SELECT 1 FROM posts WHERE title = 'Welcome to RustForge');
