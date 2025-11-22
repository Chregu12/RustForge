//! User Controller
//!
//! Handles user profile operations

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    Extension,
};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::Serialize;

use crate::middleware::auth::Claims;
use crate::models::{User, UserModel};

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub created_at: String,
}

impl From<UserModel> for ProfileResponse {
    fn from(user: UserModel) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            created_at: user.created_at.and_utc().to_rfc3339(),
        }
    }
}

pub struct UserController;

impl UserController {
    /// Get current user profile (requires authentication)
    pub async fn profile(
        State(db): State<DatabaseConnection>,
        Extension(claims): Extension<Claims>,
    ) -> Result<Json<ProfileResponse>, (StatusCode, String)> {
        let user = User::find_by_id(claims.user_id)
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

        Ok(Json(ProfileResponse::from(user)))
    }
}
