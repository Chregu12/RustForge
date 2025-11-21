# RustForge Blog Example Application

A complete, production-ready blog application demonstrating RustForge's core features.

---

## Features

This example showcases:

### Authentication & Authorization
- ✅ User registration with email validation
- ✅ Login/logout with session management
- ✅ Password reset via email
- ✅ Authorization policies (only authors can edit their posts)

### CRUD Operations
- ✅ Create, read, update, delete posts
- ✅ Create, read, delete comments
- ✅ Manage user profiles
- ✅ Tag system for posts

### Advanced Features
- ✅ Markdown support for post content
- ✅ Image uploads for post covers
- ✅ Full-text search functionality
- ✅ Pagination for posts and comments
- ✅ Author profiles with bio
- ✅ Related posts suggestion
- ✅ Comment threading

### Technical Features
- ✅ Eloquent relationships (User hasMany Post, Post hasMany Comment)
- ✅ Eager loading to prevent N+1 queries
- ✅ Blade templates with layouts and components
- ✅ Form validation with custom rules
- ✅ File storage and serving
- ✅ Database migrations and seeders
- ✅ Factory pattern for testing
- ✅ Comprehensive test suite

---

## Screenshots

### Homepage
```
┌──────────────────────────────────────────────────┐
│ 📝 RustForge Blog                    [Login]     │
├──────────────────────────────────────────────────┤
│                                                   │
│  [Search: ____________]  [🔍 Search]             │
│                                                   │
│  ┌─────────────────────────────────────────┐    │
│  │ 🖼️ [Cover Image]                         │    │
│  │                                           │    │
│  │ Getting Started with RustForge           │    │
│  │ By Alice Johnson • 2 days ago            │    │
│  │ Tags: #rust #tutorial                    │    │
│  │                                           │    │
│  │ Learn how to build web applications      │    │
│  │ with RustForge in this comprehensive...  │    │
│  │                                           │    │
│  │ [Read More →] [✏️ Edit] [🗑️ Delete]       │    │
│  │ 💬 5 comments                             │    │
│  └─────────────────────────────────────────┘    │
│                                                   │
│  ┌─────────────────────────────────────────┐    │
│  │ Advanced Rust Patterns                   │    │
│  │ By Bob Smith • 5 days ago                │    │
│  │ ...                                       │    │
│  └─────────────────────────────────────────┘    │
│                                                   │
│  « Previous | 1 2 3 4 5 | Next »                │
└──────────────────────────────────────────────────┘
```

### Post Detail
```
┌──────────────────────────────────────────────────┐
│ 📝 RustForge Blog           👤 Alice | Logout    │
├──────────────────────────────────────────────────┤
│                                                   │
│  Getting Started with RustForge                  │
│  By Alice Johnson • December 15, 2024            │
│  Tags: #rust #tutorial #beginners                │
│  [✏️ Edit] [🗑️ Delete]                           │
│                                                   │
│  ┌────────────────────────────────────────────┐ │
│  │ [Cover Image]                               │ │
│  └────────────────────────────────────────────┘ │
│                                                   │
│  RustForge is a web framework that brings        │
│  Laravel's elegance to Rust...                   │
│  [Markdown content rendered as HTML]             │
│                                                   │
│  ────────────────────────────────────────────   │
│                                                   │
│  💬 Comments (5)                                 │
│                                                   │
│  ┌─────────────────────────────────────────┐    │
│  │ John Doe • 1 day ago                     │    │
│  │ Great article! Really helped me get...   │    │
│  │ [Reply] [Delete]                         │    │
│  │                                           │    │
│  │   ↳ Alice Johnson • 1 day ago            │    │
│  │     Thanks John! Glad it helped...       │    │
│  └─────────────────────────────────────────┘    │
│                                                   │
│  [Add Comment]                                   │
│  ┌─────────────────────────────────────────┐    │
│  │ [Your comment here...]                   │    │
│  │                                           │    │
│  │                                           │    │
│  │ [Submit Comment]                         │    │
│  └─────────────────────────────────────────┘    │
└──────────────────────────────────────────────────┘
```

---

## Installation

### Prerequisites

- Rust 1.75 or higher
- PostgreSQL 14+ (or Docker)
- Redis (optional, for caching)

### Setup Steps

1. **Clone the repository**

```bash
cd examples/blog-complete
```

2. **Install dependencies**

```bash
cargo build
```

3. **Set up environment**

```bash
cp .env.example .env
```

Edit `.env` and configure your database:

```env
DATABASE_URL=postgres://postgres:secret@localhost/blog
APP_KEY=your-secret-key
```

4. **Generate application key**

```bash
forge key:generate
```

5. **Run migrations**

```bash
forge migrate
```

6. **Seed the database** (optional)

```bash
forge db:seed
```

This creates:
- 5 sample users
- 20 sample posts
- 50 sample comments
- 10 sample tags

7. **Run the application**

```bash
cargo run
# or
forge serve
```

8. **Open in browser**

Navigate to `http://localhost:8000`

**Default credentials:**
- Email: `admin@blog.local`
- Password: `password`

---

## Project Structure

```
blog-complete/
├── src/
│   ├── main.rs                     # Application entry point
│   ├── routes.rs                   # Route definitions
│   │
│   ├── controllers/
│   │   ├── mod.rs
│   │   ├── auth_controller.rs     # Registration, login, logout
│   │   ├── post_controller.rs     # CRUD for posts
│   │   ├── comment_controller.rs  # Create, delete comments
│   │   └── profile_controller.rs  # User profiles
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs                # User model with relationships
│   │   ├── post.rs                # Post model with tags, comments
│   │   ├── comment.rs             # Comment model
│   │   └── tag.rs                 # Tag model
│   │
│   ├── views/
│   │   ├── layouts/
│   │   │   └── app.blade.html     # Main layout template
│   │   ├── posts/
│   │   │   ├── index.blade.html   # List of posts
│   │   │   ├── show.blade.html    # Single post view
│   │   │   ├── create.blade.html  # Create post form
│   │   │   └── edit.blade.html    # Edit post form
│   │   ├── auth/
│   │   │   ├── login.blade.html
│   │   │   ├── register.blade.html
│   │   │   └── forgot.blade.html
│   │   └── comments/
│   │       └── _comment.blade.html # Comment partial
│   │
│   ├── policies/
│   │   └── post_policy.rs         # Authorization for posts
│   │
│   └── services/
│       ├── search_service.rs      # Full-text search
│       └── markdown_service.rs    # Markdown rendering
│
├── migrations/
│   ├── 001_create_users_table.rs
│   ├── 002_create_posts_table.rs
│   ├── 003_create_comments_table.rs
│   └── 004_create_tags_table.rs
│
├── seeders/
│   ├── user_seeder.rs
│   ├── post_seeder.rs
│   └── tag_seeder.rs
│
├── tests/
│   ├── integration/
│   │   ├── auth_test.rs
│   │   ├── post_test.rs
│   │   └── comment_test.rs
│   └── unit/
│       └── markdown_test.rs
│
├── public/
│   ├── css/
│   │   └── app.css
│   └── js/
│       └── app.js
│
├── storage/
│   ├── uploads/                   # User-uploaded images
│   └── logs/
│
├── .env.example                   # Example environment config
├── Cargo.toml                     # Dependencies
├── docker-compose.yml             # Docker setup for PostgreSQL
└── README.md                      # This file
```

---

## Key Code Examples

### 1. Eloquent Relationships

**User Model (src/models/user.rs):**

```rust
#[derive(Model)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
}

impl User {
    // User has many posts
    pub fn posts(&self) -> HasMany<Post> {
        self.has_many()
    }

    // User has many comments
    pub fn comments(&self) -> HasMany<Comment> {
        self.has_many()
    }
}
```

**Post Model (src/models/post.rs):**

```rust
#[derive(Model)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: String,
    pub cover_image: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
}

impl Post {
    // Post belongs to user (author)
    pub fn author(&self) -> BelongsTo<User> {
        self.belongs_to("user_id")
    }

    // Post has many comments
    pub fn comments(&self) -> HasMany<Comment> {
        self.has_many()
    }

    // Post has many tags (many-to-many)
    pub fn tags(&self) -> BelongsToMany<Tag> {
        self.belongs_to_many("post_tag")
    }
}
```

### 2. Eager Loading (N+1 Prevention)

**List Posts with Authors (src/controllers/post_controller.rs):**

```rust
pub async fn index(req: Request) -> Response {
    // ✅ Loads posts and authors in 2 queries (not N+1)
    let posts = Post::with("author", req.db())
        .with("tags", req.db())
        .order_by_desc(post::Column::PublishedAt)
        .paginate(15)
        .await?;

    View::make("posts.index")
        .with("posts", posts)
        .render()
}
```

### 3. Authorization Policy

**Post Policy (src/policies/post_policy.rs):**

```rust
pub struct PostPolicy;

impl PostPolicy {
    // Only the author can update their post
    pub fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.user_id
    }

    // Only the author can delete their post
    pub fn delete(&self, user: &User, post: &Post) -> bool {
        user.id == post.user_id
    }
}
```

**Using in Controller:**

```rust
pub async fn update(req: Request) -> Response {
    let post = Post::find(req.param("id")?, req.db()).await?;

    // Check authorization
    if !req.authorize("update", &post).await? {
        return Response::forbidden("You cannot edit this post");
    }

    // ... update logic
}
```

### 4. Form Validation

**Create Post (src/controllers/post_controller.rs):**

```rust
pub async fn store(req: Request) -> Response {
    let validated = req.validate(|v| {
        v.rule("title", vec![Required, MinLength(5), MaxLength(255)])
         .rule("content", vec![Required, MinLength(50)])
         .rule("cover_image", vec![Nullable, Image, MaxFileSize(5_000_000)])
         .rule("tags", vec![Array, MaxItems(5)])
    }).await?;

    // ... create post
}
```

### 5. File Upload

**Handle Cover Image:**

```rust
pub async fn store(req: Request) -> Response {
    let validated = req.validate(...).await?;

    // Handle file upload
    let cover_image = if let Some(file) = req.file("cover_image")? {
        let path = file.store("uploads/covers").await?;
        Some(path)
    } else {
        None
    };

    let post = Post::create(req.db(), PostData {
        title: validated.get("title"),
        content: validated.get("content"),
        cover_image,
        user_id: req.user()?.id,
    }).await?;

    Response::redirect(format!("/posts/{}", post.slug))
}
```

### 6. Search Functionality

**Full-Text Search (src/services/search_service.rs):**

```rust
pub struct SearchService;

impl SearchService {
    pub async fn search_posts(query: &str, db: &DatabaseConnection)
        -> Result<Vec<Post>, DbErr>
    {
        Post::query()
            .filter(
                Condition::any()
                    .add(post::Column::Title.contains(query))
                    .add(post::Column::Content.contains(query))
            )
            .order_by_desc(post::Column::PublishedAt)
            .limit(20)
            .all(db)
            .await
    }
}
```

---

## Testing

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suite

```bash
# Integration tests
cargo test --test integration

# Unit tests
cargo test --lib
```

### Test Coverage

```bash
cargo tarpaulin --out Html
```

### Example Tests

**Post Creation Test (tests/integration/post_test.rs):**

```rust
#[tokio::test]
async fn test_authenticated_user_can_create_post() {
    let app = create_test_app().await;
    let user = UserFactory::create(app.db()).await.unwrap();

    let response = app
        .post("/posts")
        .auth(&user)
        .form(json!({
            "title": "Test Post",
            "content": "This is test content...",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let post = Post::where_eq("title", "Test Post", app.db())
        .await
        .unwrap()
        .first()
        .unwrap();

    assert_eq!(post.user_id, user.id);
}
```

---

## Docker Setup

### Using Docker Compose

Start PostgreSQL and Redis:

```bash
docker-compose up -d
```

This starts:
- PostgreSQL on port 5432
- Redis on port 6379
- Adminer (DB GUI) on port 8080

---

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | / | Homepage (list posts) |
| GET | /posts/:slug | View post |
| GET | /posts/create | Create post form (auth) |
| POST | /posts | Store new post (auth) |
| GET | /posts/:id/edit | Edit post form (auth) |
| PUT | /posts/:id | Update post (auth) |
| DELETE | /posts/:id | Delete post (auth) |
| POST | /posts/:id/comments | Add comment (auth) |
| DELETE | /comments/:id | Delete comment (auth) |
| GET | /search | Search posts |
| GET | /login | Login form |
| POST | /login | Authenticate |
| POST | /logout | Logout |
| GET | /register | Registration form |
| POST | /register | Create account |
| GET | /profile/:username | User profile |

---

## Technologies Used

### Backend
- **RustForge** - Web framework
- **SeaORM** - Database ORM
- **Axum** - HTTP server
- **Tokio** - Async runtime

### Database
- **PostgreSQL** - Primary database
- **Redis** - Caching (optional)

### Frontend
- **Blade Templates** - Server-side rendering
- **TailwindCSS** - Styling (via CDN)
- **Alpine.js** - Minimal JavaScript

### Testing
- **rf-testing** - Test framework
- **Faker** - Test data generation

---

## Learning Objectives

This example teaches you:

1. **Eloquent Relationships** - How to define and use HasMany, BelongsTo, BelongsToMany
2. **Eager Loading** - Prevent N+1 queries with `.with()`
3. **Authorization** - Use policies to control access
4. **Form Validation** - Validate user input with rules
5. **File Uploads** - Handle and store user-uploaded files
6. **Search** - Implement full-text search
7. **Pagination** - Paginate large datasets
8. **Markdown** - Render Markdown content
9. **Testing** - Write integration and unit tests
10. **Best Practices** - Production-ready code structure

---

## Common Tasks

### Add a New Post

1. Log in as a user
2. Click "New Post"
3. Fill in title, content, and optionally upload cover image
4. Add tags (comma-separated)
5. Click "Publish"

### Search for Posts

1. Use the search bar on homepage
2. Enter keywords
3. View results

### Comment on a Post

1. Open a post
2. Scroll to comments section
3. Write your comment
4. Click "Submit"

### Manage Your Posts

1. Go to "My Posts"
2. View all your published posts
3. Edit or delete as needed

---

## Troubleshooting

### Database Connection Errors

**Problem:** Can't connect to database

**Solution:**
1. Check PostgreSQL is running: `pg_isready`
2. Verify DATABASE_URL in `.env`
3. Check credentials are correct

### File Upload Errors

**Problem:** Cover image upload fails

**Solution:**
1. Ensure `storage/uploads` directory exists and is writable
2. Check file size is under 5MB
3. Verify file is an image (jpg, png, gif, webp)

### Compilation Errors

**Problem:** Cargo build fails

**Solution:**
1. Update Rust: `rustup update`
2. Clean build: `cargo clean && cargo build`
3. Check Rust version: `rustc --version` (need 1.75+)

---

## Next Steps

After exploring this example:

1. **Customize It** - Change the styling, add features
2. **Deploy It** - Deploy to production (see deployment guide)
3. **Build Your Own** - Use this as a template for your project
4. **Read the Tutorials** - Dive deeper with [tutorials](/docs/tutorials/)

---

## Contributing

Found a bug or have a suggestion? Please open an issue!

---

## License

This example is MIT licensed. Use it freely for learning or as a starter for your projects.

---

**Happy coding!** 🚀
