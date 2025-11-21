# Phase 14 - Complete Integration Example

This example shows how all 4 Phase 14 crates work together to create a modern API endpoint.

## Complete API Example

```rust
// File: src/handlers/posts.rs

use axum::{
    extract::{Path, State},
    response::Json,
};
use rf_api_resources::{PaginatedCollection, PaginationMeta, Resource, ResourceCollection};
use rf_collections::collect;
use rf_requests::{FormRequest, ValidationRulesBuilder};
use rf_routing::{RouteRegistry, route_params};
use serde::{Deserialize, Serialize};

// ============================================================================
// MODELS
// ============================================================================

#[derive(Debug, Clone)]
struct Post {
    id: i64,
    title: String,
    content: String,
    author_id: i64,
    published: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
    email: String,
    is_admin: bool,
}

// ============================================================================
// API RESOURCES
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct PostResource {
    id: i64,
    title: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<UserResource>,
    published: bool,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    admin_only_field: Option<String>,
}

impl Resource for PostResource {}

impl PostResource {
    fn from_post(post: Post, user: Option<User>, is_admin: bool) -> Self {
        Self {
            id: post.id,
            title: post.title,
            content: post.content,
            author: user.map(UserResource::from),
            published: post.published,
            created_at: post.created_at.to_rfc3339(),
            admin_only_field: if is_admin {
                Some("Internal metadata".to_string())
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct UserResource {
    id: i64,
    name: String,
    email: String,
}

impl Resource for UserResource {}

impl From<User> for UserResource {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
        }
    }
}

// ============================================================================
// FORM REQUESTS
// ============================================================================

#[derive(Debug, Deserialize)]
struct CreatePostRequest {
    title: String,
    content: String,
}

#[async_trait::async_trait]
impl FormRequest for CreatePostRequest {
    async fn authorize(&self) -> rf_requests::FormRequestResult<()> {
        // Check if user can create posts
        // In real app, you'd check user permissions here
        Ok(())
    }

    fn rules(&self) -> std::collections::HashMap<String, Vec<rf_requests::ValidationRule>> {
        ValidationRulesBuilder::new()
            .required("title")
            .min_length("title", 3)
            .max_length("title", 255)
            .required("content")
            .min_length("content", 10)
            .build()
    }
}

#[derive(Debug, Deserialize)]
struct UpdatePostRequest {
    title: Option<String>,
    content: Option<String>,
    published: Option<bool>,
}

#[async_trait::async_trait]
impl FormRequest for UpdatePostRequest {
    async fn authorize(&self) -> rf_requests::FormRequestResult<()> {
        // Check if user can update this specific post
        Ok(())
    }

    fn rules(&self) -> std::collections::HashMap<String, Vec<rf_requests::ValidationRule>> {
        ValidationRulesBuilder::new()
            .min_length("title", 3)
            .max_length("title", 255)
            .min_length("content", 10)
            .build()
    }
}

// ============================================================================
// HANDLERS
// ============================================================================

/// List all posts with filtering and pagination
async fn list_posts(
    State(app): State<AppState>,
) -> Result<Json<impl Serialize>, AppError> {
    // Fetch posts from database
    let posts = fetch_posts_from_db().await?;
    let current_user = get_current_user().await?;

    // Use rf-collections for powerful data manipulation
    let filtered_posts = collect(posts)
        .filter(|p| p.published || current_user.is_admin) // Admins see unpublished
        .sort_by(|p| std::cmp::Reverse(p.created_at))     // Newest first
        .take(20)                                          // Limit to 20
        .to_vec();

    // Transform to resources
    let resources: Vec<PostResource> = filtered_posts
        .into_iter()
        .map(|post| {
            let author = fetch_user(post.author_id);
            PostResource::from_post(post, author, current_user.is_admin)
        })
        .collect();

    // Create paginated response
    let meta = PaginationMeta::new(1, 20, 100); // page 1, 20 per page, 100 total
    let collection = PaginatedCollection::new(resources, meta);

    Ok(Json(collection))
}

/// Create a new post
async fn create_post(
    State(app): State<AppState>,
    Form(request): Form<CreatePostRequest>,
) -> Result<Json<PostResource>, AppError> {
    // Request is automatically validated and authorized!
    let validated = request.process().await?;

    // Create post
    let post = Post {
        id: 123,
        title: validated.title,
        content: validated.content,
        author_id: 1,
        published: false,
        created_at: chrono::Utc::now(),
    };

    // Save to database
    save_post(&post).await?;

    // Generate resource URLs using rf-routing
    let mut routes = RouteRegistry::new();
    routes.register(NamedRoute::new("posts.show", "/api/posts/{id}"));

    let post_url = routes.url("posts.show", &route_params! {
        "id" => post.id
    }).unwrap();

    println!("Created post at: {}", post_url);

    // Return resource
    let author = fetch_user(post.author_id);
    let resource = PostResource::from_post(post, author, false);

    Ok(Json(resource))
}

/// Get a single post
async fn show_post(
    Path(id): Path<i64>,
    State(app): State<AppState>,
) -> Result<Json<PostResource>, AppError> {
    let post = fetch_post(id).await?;
    let author = fetch_user(post.author_id);
    let current_user = get_current_user().await?;

    let resource = PostResource::from_post(post, author, current_user.is_admin);

    Ok(Json(resource))
}

/// Update a post
async fn update_post(
    Path(id): Path<i64>,
    State(app): State<AppState>,
    Form(request): Form<UpdatePostRequest>,
) -> Result<Json<PostResource>, AppError> {
    // Validate and authorize
    let validated = request.process().await?;

    // Fetch and update post
    let mut post = fetch_post(id).await?;

    if let Some(title) = validated.title {
        post.title = title;
    }
    if let Some(content) = validated.content {
        post.content = content;
    }
    if let Some(published) = validated.published {
        post.published = published;
    }

    save_post(&post).await?;

    // Return updated resource
    let author = fetch_user(post.author_id);
    let current_user = get_current_user().await?;
    let resource = PostResource::from_post(post, author, current_user.is_admin);

    Ok(Json(resource))
}

/// Generate a signed download URL
async fn generate_download_url(
    Path(id): Path<i64>,
    State(app): State<AppState>,
) -> Result<Json<SignedUrlResponse>, AppError> {
    let post = fetch_post(id).await?;

    // Generate signed URL that expires in 1 hour
    let signed = SignedUrlBuilder::new(
        format!("/api/posts/{}/download", id),
        &app.secret_key
    )
    .expires_in_hours(1)
    .build();

    Ok(Json(SignedUrlResponse {
        url: signed.to_string(),
        expires_at: signed.expires_at().unwrap().to_rfc3339(),
    }))
}

// ============================================================================
// ADVANCED COLLECTION OPERATIONS
// ============================================================================

/// Example: Complex data aggregation using rf-collections
async fn post_statistics(
    State(app): State<AppState>,
) -> Result<Json<PostStats>, AppError> {
    let posts = fetch_posts_from_db().await?;

    // Group posts by author and calculate statistics
    let author_stats = collect(posts.clone())
        .group_by(|p| p.author_id);

    let stats_by_author: Vec<AuthorStats> = author_stats
        .into_iter()
        .map(|(author_id, posts)| {
            let collection = collect(posts);

            AuthorStats {
                author_id,
                total_posts: collection.count(),
                published_posts: collection
                    .clone()
                    .filter(|p| p.published)
                    .count(),
                avg_content_length: collection
                    .map(|p| p.content.len() as f64)
                    .avg(),
            }
        })
        .collect();

    // Find most active authors
    let top_authors = collect(stats_by_author.clone())
        .sort_by(|s| std::cmp::Reverse(s.total_posts))
        .take(10)
        .to_vec();

    Ok(Json(PostStats {
        total_posts: posts.len(),
        published_posts: collect(posts)
            .filter(|p| p.published)
            .count(),
        top_authors,
    }))
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Serialize)]
struct SignedUrlResponse {
    url: String,
    expires_at: String,
}

#[derive(Serialize, Clone)]
struct AuthorStats {
    author_id: i64,
    total_posts: usize,
    published_posts: usize,
    avg_content_length: f64,
}

#[derive(Serialize)]
struct PostStats {
    total_posts: usize,
    published_posts: usize,
    top_authors: Vec<AuthorStats>,
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

use axum::{
    routing::{get, post, put},
    Router,
};
use rf_routing::NamedRoute;

fn create_router() -> Router<AppState> {
    // Register all named routes
    let mut routes = RouteRegistry::new();
    routes.register(NamedRoute::new("posts.index", "/api/posts"));
    routes.register(NamedRoute::new("posts.create", "/api/posts"));
    routes.register(NamedRoute::new("posts.show", "/api/posts/{id}"));
    routes.register(NamedRoute::new("posts.update", "/api/posts/{id}"));
    routes.register(NamedRoute::new("posts.stats", "/api/posts/stats"));
    routes.register(NamedRoute::new("posts.download", "/api/posts/{id}/download"));

    Router::new()
        .route("/api/posts", get(list_posts).post(create_post))
        .route("/api/posts/stats", get(post_statistics))
        .route("/api/posts/:id", get(show_post).put(update_post))
        .route("/api/posts/:id/download", get(generate_download_url))
}

// ============================================================================
// HELPER FUNCTIONS (mock implementations)
// ============================================================================

#[derive(Clone)]
struct AppState {
    secret_key: String,
}

struct AppError;

impl From<rf_requests::FormRequestError> for AppError {
    fn from(_: rf_requests::FormRequestError) -> Self {
        AppError
    }
}

async fn fetch_posts_from_db() -> Result<Vec<Post>, AppError> {
    Ok(vec![])
}

async fn fetch_post(_id: i64) -> Result<Post, AppError> {
    Ok(Post {
        id: 1,
        title: "Test".to_string(),
        content: "Content".to_string(),
        author_id: 1,
        published: true,
        created_at: chrono::Utc::now(),
    })
}

async fn fetch_user(_id: i64) -> Option<User> {
    Some(User {
        id: 1,
        name: "John".to_string(),
        email: "john@example.com".to_string(),
        is_admin: false,
    })
}

async fn get_current_user() -> Result<User, AppError> {
    Ok(User {
        id: 1,
        name: "Current".to_string(),
        email: "current@example.com".to_string(),
        is_admin: false,
    })
}

async fn save_post(_post: &Post) -> Result<(), AppError> {
    Ok(())
}
```

## Example API Responses

### GET /api/posts

```json
{
  "data": [
    {
      "id": 1,
      "title": "My First Post",
      "content": "This is the content...",
      "author": {
        "id": 1,
        "name": "John Doe",
        "email": "john@example.com"
      },
      "published": true,
      "created_at": "2024-11-14T10:00:00Z"
    },
    {
      "id": 2,
      "title": "Another Post",
      "content": "More content...",
      "author": {
        "id": 2,
        "name": "Jane Smith",
        "email": "jane@example.com"
      },
      "published": true,
      "created_at": "2024-11-14T09:00:00Z",
      "admin_only_field": "Internal metadata"
    }
  ],
  "meta": {
    "current_page": 1,
    "last_page": 5,
    "per_page": 20,
    "total": 100,
    "from": 1,
    "to": 20
  }
}
```

### POST /api/posts

Request:
```json
{
  "title": "My New Post",
  "content": "This is my new post content..."
}
```

Response:
```json
{
  "id": 123,
  "title": "My New Post",
  "content": "This is my new post content...",
  "author": {
    "id": 1,
    "name": "John Doe",
    "email": "john@example.com"
  },
  "published": false,
  "created_at": "2024-11-14T10:30:00Z"
}
```

### GET /api/posts/stats

```json
{
  "total_posts": 150,
  "published_posts": 120,
  "top_authors": [
    {
      "author_id": 1,
      "total_posts": 45,
      "published_posts": 42,
      "avg_content_length": 1250.5
    },
    {
      "author_id": 2,
      "total_posts": 38,
      "published_posts": 35,
      "avg_content_length": 980.3
    }
  ]
}
```

### GET /api/posts/123/download

```json
{
  "url": "/api/posts/123/download?signature=abc123...&expires=1699963200",
  "expires_at": "2024-11-14T11:30:00Z"
}
```

## Key Benefits Demonstrated

### 1. Type Safety
- All requests validated at compile time and runtime
- No manual JSON serialization
- Type-safe route parameters

### 2. Developer Experience
- Fluent, readable code
- Declarative resource transformations
- Powerful data manipulation with collections

### 3. Maintainability
- Centralized validation rules
- Reusable resources
- Named routes prevent broken links

### 4. Security
- Signed URLs with expiration
- Authorization in form requests
- Conditional field exposure

### 5. Performance
- Lazy collections for large datasets
- Efficient serialization
- Zero-copy operations where possible

## Conclusion

This example demonstrates how all 4 Phase 14 crates work together to create a professional, maintainable API with Laravel-quality developer experience in Rust.
