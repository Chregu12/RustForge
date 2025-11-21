# Phase 12 Blog Example

A comprehensive full-stack blog application demonstrating the integration of all RustForge Phase 12 features.

## Features Demonstrated

### rf-blade (Template Engine)
- Template inheritance with `@extends` and `@section`
- Variable interpolation with `{{ }}` syntax
- Raw HTML output with `{!! !!}`
- Component-based architecture
- Conditional rendering with `@if` directives

### rf-vite (Asset Pipeline)
- Development server integration
- Hot Module Replacement (HMR)
- Asset fingerprinting for production
- Modern JavaScript/CSS bundling

### rf-livereload (Development Tools)
- Automatic browser reload on file changes
- CSS-only reload (no full page refresh)
- WebSocket-based communication
- Configurable debouncing

### rf-cms (Content Management)
- Media library for image uploads
- File metadata extraction
- Image thumbnail generation
- Content storage and retrieval

## Project Structure

```
phase12-blog/
├── src/
│   └── main.rs           # Main application server
├── templates/
│   ├── layouts/
│   │   └── app.blade.html    # Base layout template
│   └── posts/
│       ├── index.blade.html  # Post listing
│       ├── show.blade.html   # Single post view
│       └── create.blade.html # Create post form
├── resources/
│   ├── js/
│   │   └── app.js        # Frontend JavaScript
│   └── css/
│       └── app.css       # Frontend styles
├── storage/
│   └── media/            # Uploaded media files
└── Cargo.toml
```

## Running the Example

### Development Mode

1. Install dependencies:
```bash
npm install -D vite
```

2. Run the server:
```bash
cargo run
```

3. Open your browser to: http://localhost:3000

In development mode, you'll have:
- Live reload on template changes
- Hot Module Replacement for CSS/JS
- Automatic Vite dev server on port 5173
- LiveReload WebSocket on port 35729

### Production Mode

1. Build assets:
```bash
npx vite build
```

2. Run with production mode:
```bash
VITE_DEV=false cargo run --release
```

## Key Integration Points

### Template + Assets
The layout template (`templates/layouts/app.blade.html`) conditionally loads assets:
- **Dev**: Uses Vite dev server URLs with HMR
- **Prod**: Uses fingerprinted build assets

### Live Reload
Templates, CSS, and JavaScript files are watched for changes:
- Template changes trigger full reload
- CSS changes trigger stylesheet reload only
- JS changes reload affected modules

### Media Upload
The CMS integration provides:
- File upload endpoint at `/media/upload`
- Automatic file hashing and deduplication
- Image thumbnail generation
- Secure file storage

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Homepage - list all posts |
| GET | `/posts/:id` | View single post |
| GET | `/posts/create` | Show create post form |
| POST | `/posts/create` | Create new post |
| POST | `/media/upload` | Upload media file |

## Sample Data

The application seeds with two example posts on startup to demonstrate:
- Post listing with cards
- Individual post views
- HTML content rendering
- Template inheritance

## Extending the Example

### Add Authentication
Integrate with Phase 11's `rf-auth` for user authentication:
```rust
use rf_auth::AuthMiddleware;

let app = Router::new()
    .route("/posts/create", get(create_post_form))
    .layer(AuthMiddleware::new());
```

### Add Database
Replace in-memory storage with `rf-database`:
```rust
use rf_database::Database;

let db = Database::connect("postgres://...").await?;
let posts = db.query("SELECT * FROM posts").await?;
```

### Add API
Create a JSON API alongside the HTML views:
```rust
#[derive(Serialize)]
struct PostResponse {
    data: Post,
}

async fn api_posts() -> Json<Vec<Post>> {
    // Return JSON instead of HTML
}
```

## Performance Notes

- Templates are cached after first compilation
- Static assets served from `/storage` directory
- In-memory storage for demo (use database in production)
- HMR reduces development cycle time significantly

## License

Part of the RustForge framework examples.
