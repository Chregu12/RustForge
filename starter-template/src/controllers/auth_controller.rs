//! Authentication Controller
//!
//! Handles user registration, login, and JWT token generation

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth::Claims;
use crate::models::{User, UserActiveModel};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 2, max = 100, message = "Name must be between 2 and 100 characters"))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub name: String,
}

pub struct AuthController;

impl AuthController {
    /// Register a new user
    pub async fn register(
        State(db): State<DatabaseConnection>,
        Json(req): Json<RegisterRequest>,
    ) -> Result<Json<AuthResponse>, (StatusCode, String)> {
        // Validate request
        req.validate()
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        // Check if user already exists
        let existing = User::find()
            .filter(crate::models::UserColumn::Email.eq(&req.email))
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if existing.is_some() {
            return Err((StatusCode::CONFLICT, "Email already registered".to_string()));
        }

        // Hash password
        let password_hash = Self::hash_password(&req.password)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Create user
        let now = Utc::now().naive_utc();
        let user = UserActiveModel {
            email: Set(req.email.clone()),
            name: Set(req.name),
            password_hash: Set(password_hash),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            ..Default::default()
        };

        let user = user
            .insert(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // Generate JWT token
        let token = Self::generate_token(user.id, &user.email)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tracing::info!("User registered: {}", user.email);

        Ok(Json(AuthResponse {
            token,
            user: UserResponse {
                id: user.id,
                email: user.email,
                name: user.name,
            },
        }))
    }

    /// Login user
    pub async fn login(
        State(db): State<DatabaseConnection>,
        Json(req): Json<LoginRequest>,
    ) -> Result<Json<AuthResponse>, (StatusCode, String)> {
        // Validate request
        req.validate()
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

        // Find user
        let user = User::find()
            .filter(crate::models::UserColumn::Email.eq(&req.email))
            .filter(crate::models::UserColumn::DeletedAt.is_null())
            .one(&db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

        // Verify password
        Self::verify_password(&req.password, &user.password_hash)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

        // Generate JWT token
        let token = Self::generate_token(user.id, &user.email)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        tracing::info!("User logged in: {}", user.email);

        Ok(Json(AuthResponse {
            token,
            user: UserResponse {
                id: user.id,
                email: user.email,
                name: user.name,
            },
        }))
    }

    // Helper methods

    fn hash_password(password: &str) -> Result<String, anyhow::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
        Ok(hash.to_string())
    }

    fn verify_password(password: &str, hash: &str) -> Result<(), anyhow::Error> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| anyhow::anyhow!("Failed to parse hash: {}", e))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|e| anyhow::anyhow!("Password verification failed: {}", e))
    }

    fn generate_token(user_id: i32, email: &str) -> Result<String, anyhow::Error> {
        let secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-this-secret-in-production-min-32-chars".to_string());

        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: email.to_string(),
            user_id,
            exp: expiration,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| anyhow::anyhow!("Failed to generate token: {}", e))
    }
}
