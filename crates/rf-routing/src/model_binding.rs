//! Route Model Binding - Automatic model resolution from route parameters
//!
//! This module provides Laravel-style route model binding, allowing automatic
//! resolution of models from route parameters.
//!
//! ## Usage
//!
//! ### Implicit Binding
//!
//! ```rust,ignore
//! use axum::routing::get;
//! use rf_routing::ModelBinding;
//!
//! async fn show_user(ModelBinding(user): ModelBinding<User>) -> Json<User> {
//!     Json(user)
//! }
//!
//! let app = Router::new()
//!     .route("/users/:id", get(show_user));
//! ```
//!
//! ### Explicit Binding with Custom Resolution
//!
//! ```rust,ignore
//! use rf_routing::ModelBindingRegistry;
//!
//! let mut registry = ModelBindingRegistry::new();
//! registry.bind::<User>("user", |value, db| async move {
//!     User::find_by_slug(value, db).await
//! });
//! ```

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use sea_orm::{DatabaseConnection, EntityTrait, PrimaryKeyTrait};
use serde::de::DeserializeOwned;
use std::{
    any::Any,
    collections::HashMap,
    fmt,
    future::Future,
    pin::Pin,
    str::FromStr,
    sync::Arc,
};
use thiserror::Error;

/// Error types for model binding
#[derive(Error, Debug)]
pub enum ModelBindingError {
    #[error("Model not found: {model} with key {key}")]
    NotFound { model: String, key: String },

    #[error("Missing route parameter: {parameter}")]
    MissingParameter { parameter: String },

    #[error("Invalid parameter format: {message}")]
    InvalidFormat { message: String },

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
}

impl IntoResponse for ModelBindingError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ModelBindingError::NotFound { model, key } => {
                (StatusCode::NOT_FOUND, format!("{} not found: {}", model, key))
            }
            ModelBindingError::MissingParameter { parameter } => (
                StatusCode::BAD_REQUEST,
                format!("Missing route parameter: {}", parameter),
            ),
            ModelBindingError::InvalidFormat { message } => {
                (StatusCode::BAD_REQUEST, format!("Invalid format: {}", message))
            }
            ModelBindingError::DatabaseError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", err))
            }
        };

        (status, message).into_response()
    }
}

/// Trait for models that can be bound from route parameters
#[async_trait]
pub trait Bindable: Sized + Send + Sync {
    /// The primary key type
    type Key: FromStr + Send + Sync + fmt::Display;

    /// Find a model by its route key
    async fn find_by_route_key(
        key: Self::Key,
        db: &DatabaseConnection,
    ) -> Result<Option<Self>, sea_orm::DbErr>;

    /// Get the name of the route key parameter (default: "id")
    fn route_key_name() -> &'static str {
        "id"
    }

    /// Get the model name for error messages
    fn model_name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// Extractor for automatic model binding
///
/// # Example
///
/// ```rust,ignore
/// async fn show(ModelBinding(user): ModelBinding<User>) -> Json<User> {
///     Json(user)
/// }
/// ```
pub struct ModelBinding<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for ModelBinding<T>
where
    T: Bindable + 'static,
    S: Send + Sync,
    DatabaseConnection: FromRequestParts<S>,
{
    type Rejection = ModelBindingError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract database connection
        let db = DatabaseConnection::from_request_parts(parts, state)
            .await
            .map_err(|_| ModelBindingError::DatabaseError(sea_orm::DbErr::Custom(
                "Could not extract database connection".to_string(),
            )))?;

        // Extract path parameters
        let path_params = Path::<HashMap<String, String>>::from_request_parts(parts, state)
            .await
            .map_err(|_| ModelBindingError::MissingParameter {
                parameter: T::route_key_name().to_string(),
            })?;

        // Get the key from path parameters
        let key_str = path_params
            .get(T::route_key_name())
            .ok_or_else(|| ModelBindingError::MissingParameter {
                parameter: T::route_key_name().to_string(),
            })?;

        // Parse the key
        let key = T::Key::from_str(key_str).map_err(|_| ModelBindingError::InvalidFormat {
            message: format!("Cannot parse {} as key", key_str),
        })?;

        // Find the model
        let model = T::find_by_route_key(key, &db)
            .await?
            .ok_or_else(|| ModelBindingError::NotFound {
                model: T::model_name().to_string(),
                key: key_str.to_string(),
            })?;

        Ok(ModelBinding(model))
    }
}

/// Type alias for custom resolver functions
type ResolverFn<T> = Arc<
    dyn Fn(String, Arc<DatabaseConnection>) -> Pin<Box<dyn Future<Output = Option<T>> + Send>>
        + Send
        + Sync,
>;

/// Registry for explicit model bindings
///
/// Allows registering custom model resolution logic.
///
/// # Example
///
/// ```rust,ignore
/// let mut registry = ModelBindingRegistry::new();
///
/// // Bind User by slug instead of ID
/// registry.bind::<User>("user", |value, db| async move {
///     User::find_by_slug(&value, &db).await.ok()
/// });
/// ```
pub struct ModelBindingRegistry {
    bindings: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ModelBindingRegistry {
    /// Create a new binding registry
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Register a custom binding resolver
    pub fn bind<T, F, Fut>(&mut self, name: &str, resolver: F)
    where
        T: Send + Sync + 'static,
        F: Fn(String, Arc<DatabaseConnection>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<T>> + Send + 'static,
    {
        let resolver: ResolverFn<T> = Arc::new(move |value, db| {
            Box::pin(resolver(value, db))
        });
        self.bindings.insert(name.to_string(), Box::new(resolver));
    }

    /// Resolve a model using a registered binding
    pub async fn resolve<T: 'static>(
        &self,
        name: &str,
        value: String,
        db: Arc<DatabaseConnection>,
    ) -> Option<T> {
        let resolver = self.bindings.get(name)?;
        let resolver = resolver.downcast_ref::<ResolverFn<T>>()?;
        resolver(value, db).await
    }

    /// Check if a binding exists
    pub fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Remove a binding
    pub fn forget(&mut self, name: &str) {
        self.bindings.remove(name);
    }

    /// Clear all bindings
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    /// Get the number of registered bindings
    pub fn count(&self) -> usize {
        self.bindings.len()
    }
}

impl Default for ModelBindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ModelBindingRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelBindingRegistry")
            .field("binding_count", &self.bindings.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_binding_registry() {
        let mut registry = ModelBindingRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(!registry.has_binding("user"));

        registry.bind::<String, _, _>("user", |value, _db| async move {
            Some(format!("User: {}", value))
        });

        assert_eq!(registry.count(), 1);
        assert!(registry.has_binding("user"));

        registry.forget("user");
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = ModelBindingRegistry::new();
        registry.bind::<String, _, _>("user", |value, _db| async move { Some(value) });
        registry.bind::<i32, _, _>("post", |_value, _db| async move { Some(42) });

        assert_eq!(registry.count(), 2);

        registry.clear();
        assert_eq!(registry.count(), 0);
    }

    #[tokio::test]
    async fn test_model_binding_error_display() {
        let err = ModelBindingError::NotFound {
            model: "User".to_string(),
            key: "123".to_string(),
        };
        assert!(err.to_string().contains("User"));
        assert!(err.to_string().contains("123"));

        let err = ModelBindingError::MissingParameter {
            parameter: "id".to_string(),
        };
        assert!(err.to_string().contains("id"));
    }
}
