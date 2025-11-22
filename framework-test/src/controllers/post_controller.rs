/*!
 * Post Controller
 *
 * Handles blog post operations and demonstrates all relationship types.
 */

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use crate::{AppState, models::{Post, Comment, Tag, Image}};

#[derive(Debug, Deserialize)]
pub struct ListPostsQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub featured: Option<bool>,
    pub category_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct PostListResponse {
    pub data: Vec<Post>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub user_id: i32,
    pub category_id: Option<i32>,
    pub title: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub featured: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub excerpt: Option<String>,
    pub featured: Option<bool>,
    pub category_id: Option<i32>,
}

/// List all posts with pagination
pub async fn index(
    State(state): State<AppState>,
    query: Option<Query<ListPostsQuery>>,
) -> Result<Json<PostListResponse>, (StatusCode, String)> {
    // REAL IMPLEMENTATION would use:
    // let posts = Post::query()
    //     .with("user")
    //     .with("category")
    //     .where_published()
    //     .latest()
    //     .paginate(page, per_page)
    //     .await?;

    let query = query.map(|q| q.0).unwrap_or(ListPostsQuery {
        page: Some(1),
        per_page: Some(15),
        featured: None,
        category_id: None,
    });

    let demo_posts = vec![
        Post::factory(1, 1, "Getting Started with RustForge"),
        Post::factory(2, 1, "Advanced Eloquent Relationships"),
        Post::factory(3, 2, "Building RESTful APIs"),
    ];

    Ok(Json(PostListResponse {
        data: demo_posts,
        total: 3,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(15),
    }))
}

/// Get a single post with relationships
pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Post>, (StatusCode, String)> {
    // REAL IMPLEMENTATION with eager loading:
    // let post = Post::query()
    //     .with("user")
    //     .with("category")
    //     .with("comments.user")
    //     .with("tags")
    //     .with("images")
    //     .find(id)
    //     .await?;

    let mut post = Post::factory(id, 1, "Demo Post");

    // Increment view count (demonstrates model methods)
    post.increment_views(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(post))
}

/// Create a new post
pub async fn store(
    State(state): State<AppState>,
    Json(request): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<Post>), (StatusCode, String)> {
    // REAL IMPLEMENTATION would:
    // 1. Validate request (rf-validation)
    // 2. Create post in database
    // 3. Fire PostCreated event
    // 4. Index in search (rf-search)

    let mut post = Post::factory(999, request.user_id, &request.title);
    post.content = request.content;
    post.excerpt = request.excerpt;
    post.featured = request.featured.unwrap_or(false);
    post.category_id = request.category_id;

    // Event dispatching would be:
    // PostCreated::dispatch(&post).await?;

    // Search indexing would be:
    // post.searchable().await?;

    Ok((StatusCode::CREATED, Json(post)))
}

/// Update an existing post
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdatePostRequest>,
) -> Result<Json<Post>, (StatusCode, String)> {
    // REAL IMPLEMENTATION:
    // let mut post = Post::find(id).await?;
    // if let Some(title) = request.title {
    //     post.title = title;
    //     post.slug = slugify(&post.title);
    // }
    // post.save().await?;

    let mut post = Post::factory(id, 1, "Updated Post");

    if let Some(title) = request.title {
        post.title = title;
        post.slug = post.title.to_lowercase().replace(" ", "-");
    }

    if let Some(content) = request.content {
        post.content = content;
    }

    Ok(Json(post))
}

/// Delete a post
pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    // REAL IMPLEMENTATION:
    // let mut post = Post::find(id).await?;
    // post.soft_delete();
    // post.save().await?;
    // post.unsearchable().await?; // Remove from search index

    Ok(StatusCode::NO_CONTENT)
}

/// Get post's comments (demonstrating MorphMany polymorphic relationship)
pub async fn comments(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<Comment>>, (StatusCode, String)> {
    let post = Post::factory(id, 1, "Demo Post");

    // Demonstrates polymorphic MorphMany relationship
    let comments = post.comments(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(comments))
}

/// Get post's tags (demonstrating MorphToMany polymorphic relationship)
pub async fn tags(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<Tag>>, (StatusCode, String)> {
    let post = Post::factory(id, 1, "Demo Post");

    // Demonstrates polymorphic MorphToMany relationship
    let tags = post.tags(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tags))
}

/// Get post's images (demonstrating MorphMany polymorphic relationship)
pub async fn images(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<Image>>, (StatusCode, String)> {
    let post = Post::factory(id, 1, "Demo Post");

    // Demonstrates polymorphic MorphMany relationship
    let images = post.images(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(images))
}

/// Get post's author (demonstrating BelongsTo relationship)
pub async fn author(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<crate::models::User>, (StatusCode, String)> {
    let post = Post::factory(id, 1, "Demo Post");

    // Demonstrates BelongsTo relationship
    let user = post.user(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Author not found".to_string()))?;

    Ok(Json(user))
}
