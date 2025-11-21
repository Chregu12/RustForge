# Phase 12 Admin Panel Example

Integration example showing how Phase 12 CMS features work with Phase 11 Admin features.

## Features Integrated

### Phase 11 (Enterprise)
- **rf-admin**: Admin panel scaffolding and CRUD operations
- **rf-auth**: Authentication and session management
- **rf-authorization**: Role-based access control

### Phase 12 (Full-Stack/CMS)
- **rf-blade**: Admin panel templates
- **rf-cms**: Media library integration
- **rf-vite**: Admin asset pipeline
- **rf-livereload**: Development workflow

## Architecture

```
Admin Panel
├── Authentication (rf-auth)
│   ├── Login/Logout
│   ├── Session Management
│   └── Remember Me
├── Authorization (rf-authorization)
│   ├── Role-based Access
│   ├── Permissions
│   └── Policy Guards
├── Admin Interface (rf-admin)
│   ├── Dashboard
│   ├── CRUD Generators
│   └── Data Tables
├── Content Management (rf-cms)
│   ├── Media Library
│   ├── Image Upload
│   └── File Manager
├── Templates (rf-blade)
│   ├── Admin Layouts
│   ├── Forms
│   └── Components
└── Assets (rf-vite + rf-livereload)
    ├── Admin CSS/JS
    └── HMR in Development
```

## Sample Implementation

### 1. Admin Application Setup

```rust
use rf_admin::{AdminPanel, ResourceConfig};
use rf_auth::AuthMiddleware;
use rf_authorization::Policy;
use rf_blade::BladeEngine;
use rf_cms::MediaLibrary;
use rf_vite::ViteConfig;
use rf_livereload::LiveReload;
use axum::Router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize template engine
    let blade = BladeEngine::new("templates/admin")?;

    // Initialize CMS
    let media = MediaLibrary::new("storage/admin/media");

    // Initialize admin panel
    let admin = AdminPanel::new("/admin")
        .title("RustForge Admin")
        .with_blade(blade)
        .with_media(media);

    // Register resources
    admin.resource("posts", ResourceConfig::new()
        .fields(vec![
            Field::text("title"),
            Field::wysiwyg("content"),
            Field::image("featured_image"),
        ])
        .policy(PostPolicy)
    );

    // Setup development tools
    if cfg!(debug_assertions) {
        let live_reload = LiveReload::new()
            .watch("templates/admin")
            .watch("resources/admin");

        tokio::spawn(async move {
            live_reload.start().await.ok();
        });

        let vite = ViteConfig::new(".")
            .entry("resources/admin/js/app.js")
            .entry("resources/admin/css/app.css");

        tokio::spawn(async move {
            vite.dev_server().await.ok();
        });
    }

    // Build router with auth middleware
    let app = Router::new()
        .nest("/admin", admin.routes())
        .layer(AuthMiddleware::require_auth());

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### 2. Admin Template (Blade)

```html
<!-- templates/admin/layouts/app.blade.html -->
<!DOCTYPE html>
<html>
<head>
    <title>@yield('title') - Admin Panel</title>

    @if($vite_dev)
        <script type="module" src="http://localhost:5173/@vite/client"></script>
        <script type="module" src="http://localhost:5173/resources/admin/js/app.js"></script>
        <link rel="stylesheet" href="http://localhost:5173/resources/admin/css/app.css">
    @else
        <link rel="stylesheet" href="/build/admin/app.css">
        <script type="module" src="/build/admin/app.js"></script>
    @endif
</head>
<body>
    <nav class="admin-nav">
        <a href="/admin">Dashboard</a>
        <a href="/admin/posts">Posts</a>
        <a href="/admin/media">Media Library</a>
        <a href="/admin/users">Users</a>

        @auth
            <span>{{ $user.name }}</span>
            <a href="/logout">Logout</a>
        @endauth
    </nav>

    <main>
        @yield('content')
    </main>
</body>
</html>
```

### 3. Media Library Integration

```html
<!-- templates/admin/media/index.blade.html -->
@extends('admin.layouts.app')

@section('title', 'Media Library')

@section('content')
<div class="media-library">
    <div class="upload-area">
        <form action="/admin/media/upload" method="POST" enctype="multipart/form-data">
            @csrf
            <input type="file" name="files[]" multiple accept="image/*">
            <button type="submit">Upload</button>
        </form>
    </div>

    <div class="media-grid">
        @foreach($media_files as $file)
            <div class="media-item" data-id="{{ $file.id }}">
                <img src="{{ $file.thumbnail_url }}" alt="{{ $file.filename }}">
                <div class="media-info">
                    <span>{{ $file.filename }}</span>
                    <span>{{ $file.size_human }}</span>
                </div>
                <div class="media-actions">
                    <button class="copy-url">Copy URL</button>
                    <button class="delete">Delete</button>
                </div>
            </div>
        @endforeach
    </div>
</div>

<script>
// Media library interactions
document.querySelectorAll('.copy-url').forEach(btn => {
    btn.addEventListener('click', async (e) => {
        const item = e.target.closest('.media-item');
        const id = item.dataset.id;
        const url = `/storage/media/${id}`;
        await navigator.clipboard.writeText(url);
        alert('URL copied!');
    });
});
</script>
@endsection
```

### 4. WYSIWYG Editor Integration

```html
<!-- templates/admin/posts/form.blade.html -->
@extends('admin.layouts.app')

@section('title', $post ? 'Edit Post' : 'Create Post')

@section('content')
<form action="{{ $action }}" method="POST">
    @csrf

    <div class="form-group">
        <label>Title</label>
        <input type="text" name="title" value="{{ $post.title ?? '' }}" required>
    </div>

    <div class="form-group">
        <label>Content</label>
        <textarea
            id="editor"
            name="content"
        >{{ $post.content ?? '' }}</textarea>
    </div>

    <div class="form-group">
        <label>Featured Image</label>
        <button type="button" id="select-image">Select from Media Library</button>
        <input type="hidden" name="featured_image" id="featured-image-id">
        <img id="featured-image-preview" src="{{ $post.featured_image ?? '' }}">
    </div>

    <button type="submit">Save</button>
</form>

<!-- TinyMCE Integration via rf-cms -->
<script src="https://cdn.tiny.cloud/1/YOUR-API-KEY/tinymce/6/tinymce.min.js"></script>
<script>
tinymce.init({
    selector: '#editor',
    plugins: 'image link lists',
    toolbar: 'undo redo | formatselect | bold italic | alignleft aligncenter alignright | bullist numlist | link image',
    file_picker_callback: (callback) => {
        // Open media library modal
        openMediaLibrary((file) => {
            callback(file.url, { alt: file.filename });
        });
    }
});

function openMediaLibrary(onSelect) {
    // Custom media library modal
    // Integrates with rf-cms MediaLibrary
    fetch('/admin/media/list')
        .then(r => r.json())
        .then(files => {
            // Show modal with files
            // Call onSelect when user picks a file
        });
}
</script>
@endsection
```

## Key Integration Points

### 1. Authentication Flow

```
User Login → rf-auth validates
           → Session created
           → Redirect to admin dashboard
           → All admin routes protected by AuthMiddleware
```

### 2. Authorization Checks

```rust
use rf_authorization::{Policy, PolicyResult};

struct PostPolicy;

impl Policy for PostPolicy {
    fn view(&self, user: &User, post: &Post) -> PolicyResult {
        // Anyone can view published posts
        if post.published {
            return PolicyResult::Allow;
        }

        // Only author or admin can view drafts
        if user.id == post.author_id || user.is_admin() {
            PolicyResult::Allow
        } else {
            PolicyResult::Deny("You cannot view this draft".into())
        }
    }

    fn update(&self, user: &User, post: &Post) -> PolicyResult {
        if user.id == post.author_id || user.has_permission("edit_any_post") {
            PolicyResult::Allow
        } else {
            PolicyResult::Deny("You cannot edit this post".into())
        }
    }
}
```

### 3. Media Upload Handler

```rust
async fn upload_media(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, StatusCode> {
    let mut uploaded_files = vec![];

    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(filename) = field.file_name() {
            let data = field.bytes().await.unwrap();

            // Upload via rf-cms
            let file = state.media.upload(filename, data.to_vec())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Generate thumbnail
            let thumbnail = state.media.thumbnail(&file.id, 200, 200)
                .await
                .ok();

            uploaded_files.push(MediaResponse {
                id: file.id,
                url: format!("/storage/media/{}", file.filename),
                thumbnail_url: thumbnail.map(|t| format!("/storage/media/{}", t.filename)),
                filename: file.filename,
                size: file.size,
            });
        }
    }

    Ok(Json(UploadResponse {
        success: true,
        files: uploaded_files,
    }))
}
```

## Development Workflow

### Start Development Server

```bash
# Terminal 1: Run the admin server
cargo run

# Terminal 2: Watch for frontend changes (handled automatically)
# Vite dev server starts on port 5173
# LiveReload WebSocket on port 35729
```

### Live Reload Behavior

1. **Template Changes** (`templates/admin/**/*.blade.html`)
   - Full page reload via LiveReload
   - Changes appear instantly

2. **CSS Changes** (`resources/admin/css/**/*.css`)
   - Hot reload without page refresh
   - Styles update immediately

3. **JavaScript Changes** (`resources/admin/js/**/*.js`)
   - Hot Module Replacement via Vite
   - Modules reload with state preservation

## Production Build

```bash
# Build frontend assets
npm run build

# Run in production mode
VITE_DEV=false cargo run --release
```

## Security Considerations

1. **CSRF Protection**: Use `@csrf` directive in all forms
2. **Auth Middleware**: Protect all admin routes
3. **File Upload Validation**: Validate file types and sizes
4. **XSS Prevention**: Sanitize WYSIWYG content via rf-cms
5. **Authorization**: Check permissions for every action

## Extending the Admin Panel

### Add Custom Resource

```rust
admin.resource("products", ResourceConfig::new()
    .fields(vec![
        Field::text("name"),
        Field::number("price"),
        Field::select("category", vec!["Electronics", "Clothing", "Books"]),
        Field::image("image"),
        Field::textarea("description"),
    ])
    .display_columns(vec!["name", "price", "category"])
    .filters(vec!["category", "price_range"])
    .search_fields(vec!["name", "description"])
);
```

### Add Dashboard Widget

```html
@extends('admin.layouts.app')

@section('content')
<div class="dashboard">
    <x-admin-widget title="Total Posts" :value="$stats.total_posts" icon="📝" />
    <x-admin-widget title="Media Files" :value="$stats.media_count" icon="🖼️" />
    <x-admin-widget title="Users" :value="$stats.user_count" icon="👥" />
    <x-admin-widget title="Storage Used" :value="$stats.storage_used" icon="💾" />
</div>
@endsection
```

## Complete Feature Matrix

| Feature | Crate | Status |
|---------|-------|--------|
| User Authentication | rf-auth | ✅ |
| Role-based Access | rf-authorization | ✅ |
| Admin CRUD | rf-admin | ✅ |
| Template Engine | rf-blade | ✅ |
| Media Library | rf-cms | ✅ |
| WYSIWYG Editor | rf-cms | ✅ |
| Asset Pipeline | rf-vite | ✅ |
| Live Reload | rf-livereload | ✅ |

## Next Steps

1. Implement custom authentication UI
2. Add more admin resources
3. Create custom dashboard widgets
4. Integrate analytics
5. Add export/import functionality

## License

Part of the RustForge framework examples.
