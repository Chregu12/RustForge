//! Post Controller
//!
//! Handles CRUD operations for posts

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth::Claims;
use crate::models::{Post, PostActiveModel, PostModel};

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,

    #[validate(length(min = 1, message = "Content is required"))]
    pub content: String,

    pub published: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: Option<String>,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: Option<String>,

    pub published: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub published: Option<bool>,
    pub user_id: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PostModel> for PostResponse {
    fn from(post: PostModel) -> Self {
        Self {
            id: post.id,
            title: post.title,
            content: post.content,
            published: post.published,
            user_id: post.user_id,
            created_at: post.created_at.and_utc().to_rfc3339(),
            updated_at: post.updated_at.and_utc().to_rfc3339(),
        }
    }
}

pub struct PostController;

impl PostController {
    /// List all posts
    pub async fn list(
        State(db): State<DatabaseConnection>,
    ) -> Result<Json<Vec<PostResponse>>, (StatusCode, String)> {
        let posts = Post::find()
            .all(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(posts.into_iter().map(PostResponse::from).collect()))
    }

    /// Get a single post by ID
    pub async fn get(
        State(db): State<DatabaseConnection>,
        Path(id): Path<i32>,
    ) -> Result<Json<PostResponse>, (StatusCode, String)> {
        let post = Post::find_by_id(id)
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

        Ok(Json(PostResponse::from(post)))
    }

    /// Create a new post (requires authentication)
    pub async fn create(
        State(db): State<DatabaseConnection>,
        Extension(claims): Extension<Claims>,
        Json(req): Json<CreatePostRequest>,
    ) -> Result<Json<PostResponse>, (StatusCode, String)> {
        // Validate request
        req.validate()
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        let now = Utc::now().naive_utc();
        let post = PostActiveModel {
            title: Set(req.title),
            content: Set(req.content),
            published: Set(req.published),
            user_id: Set(claims.user_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let post = post
            .insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tracing::info!("Post created: {} by user {}", post.id, claims.user_id);

        Ok(Json(PostResponse::from(post)))
    }

    /// Update a post (requires authentication and ownership)
    pub async fn update(
        State(db): State<DatabaseConnection>,
        Extension(claims): Extension<Claims>,
        Path(id): Path<i32>,
        Json(req): Json<UpdatePostRequest>,
    ) -> Result<Json<PostResponse>, (StatusCode, String)> {
        // Validate request
        req.validate()
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        // Find post
        let post = Post::find_by_id(id)
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

        // Check ownership
        if post.user_id != claims.user_id {
            return Err((StatusCode::FORBIDDEN, "Not authorized to update this post".to_string()));
        }

        // Update post
        let mut active_post: PostActiveModel = post.into();
        if let Some(title) = req.title {
            active_post.title = Set(title);
        }
        if let Some(content) = req.content {
            active_post.content = Set(content);
        }
        if let Some(published) = req.published {
            active_post.published = Set(Some(published));
        }
        active_post.updated_at = Set(Utc::now().naive_utc());

        let post = active_post
            .update(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tracing::info!("Post updated: {} by user {}", post.id, claims.user_id);

        Ok(Json(PostResponse::from(post)))
    }

    /// Delete a post (requires authentication and ownership)
    pub async fn delete(
        State(db): State<DatabaseConnection>,
        Extension(claims): Extension<Claims>,
        Path(id): Path<i32>,
    ) -> Result<StatusCode, (StatusCode, String)> {
        // Find post
        let post = Post::find_by_id(id)
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

        // Check ownership
        if post.user_id != claims.user_id {
            return Err((StatusCode::FORBIDDEN, "Not authorized to delete this post".to_string()));
        }

        // Delete post
        Post::delete_by_id(id)
            .exec(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tracing::info!("Post deleted: {} by user {}", id, claims.user_id);

        Ok(StatusCode::NO_CONTENT)
    }
}
