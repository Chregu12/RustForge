/*!
 * User Controller
 *
 * Handles user management operations including CRUD, authentication, and authorization.
 */

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use crate::{AppState, models::User};
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub data: Vec<User>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
}

/// List all users with pagination
pub async fn index(
    State(state): State<AppState>,
    query: Option<axum::extract::Query<ListUsersQuery>>,
) -> Result<Json<UserListResponse>, (StatusCode, String)> {
    // REAL IMPLEMENTATION would query database:
    // let users = User::query()
    //     .where_not_null("email_verified_at")
    //     .paginate(page, per_page)
    //     .await?;

    let query = query.map(|q| q.0).unwrap_or(ListUsersQuery {
        page: Some(1),
        per_page: Some(15),
        search: None,
    });

    // Demo data showing the relationship system works
    let demo_users = vec![
        User::factory(1, "John Doe", "john@example.com"),
        User::factory(2, "Jane Smith", "jane@example.com"),
        User::factory(3, "Bob Johnson", "bob@example.com"),
    ];

    Ok(Json(UserListResponse {
        data: demo_users,
        total: 3,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(15),
    }))
}

/// Get a single user by ID
pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<User>, (StatusCode, String)> {
    // REAL IMPLEMENTATION:
    // let user = User::find(id).await
    //     .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(User::factory(id, "Demo User", &format!("user{}@example.com", id))))
}

/// Create a new user
pub async fn store(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), (StatusCode, String)> {
    // REAL IMPLEMENTATION would:
    // 1. Validate request
    // 2. Hash password
    // 3. Create user in database
    // 4. Dispatch welcome email job
    // 5. Fire UserCreated event

    // Validation would be done with rf-validation:
    // request.validate()?;

    // Password hashing with rf-auth:
    // let hashed_password = hash_password(&request.password)?;

    let user = User::factory(999, &request.name, &request.email);

    // Job dispatching would be:
    // SendWelcomeEmailJob::new(user.id).dispatch().await?;

    Ok((StatusCode::CREATED, Json(user)))
}

/// Update an existing user
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<User>, (StatusCode, String)> {
    // REAL IMPLEMENTATION:
    // let mut user = User::find(id).await?;
    // if let Some(name) = request.name {
    //     user.name = name;
    // }
    // user.save().await?;

    let mut user = User::factory(id, "Updated User", &format!("user{}@example.com", id));

    if let Some(name) = request.name {
        user.name = name;
    }

    Ok(Json(user))
}

/// Soft delete a user
pub async fn destroy(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    // REAL IMPLEMENTATION:
    // let mut user = User::find(id).await?;
    // user.soft_delete();
    // user.save().await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Restore a soft-deleted user
pub async fn restore(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<User>, (StatusCode, String)> {
    // REAL IMPLEMENTATION:
    // let mut user = User::with_trashed().find(id).await?;
    // user.restore();
    // user.save().await?;

    let user = User::factory(id, "Restored User", &format!("user{}@example.com", id));
    Ok(Json(user))
}

/// Get user's posts (demonstrating HasMany relationship)
pub async fn posts(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<crate::models::Post>>, (StatusCode, String)> {
    let user = User::factory(id, "Demo User", &format!("user{}@example.com", id));

    // REAL IMPLEMENTATION demonstrates HasMany relationship:
    let posts = user.posts(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(posts))
}

/// Get user's roles (demonstrating BelongsToMany relationship)
pub async fn roles(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<crate::models::Role>>, (StatusCode, String)> {
    let user = User::factory(id, "Demo User", &format!("user{}@example.com", id));

    // REAL IMPLEMENTATION demonstrates BelongsToMany relationship:
    let roles = user.roles(&state).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(roles))
}
