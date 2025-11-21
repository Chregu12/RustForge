//! Middleware for Authorization
//!
//! Provides middleware to protect routes with authorization checks.

use crate::error::{AuthorizationError, AuthorizationResult};
use crate::gates::Gate;
use crate::policies::PolicyRegistry;
use async_trait::async_trait;
use std::sync::Arc;

/// Middleware trait for authorization
///
/// This is a simplified middleware trait. In a real application, this would
/// integrate with your web framework (Axum, Actix, etc.)
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Handle the request
    async fn handle(&self, request: Request) -> AuthorizationResult<Response>;
}

/// Simplified request type for middleware
///
/// In a real application, this would be your web framework's request type
pub struct Request {
    pub extensions: Extensions,
}

impl Request {
    pub fn new() -> Self {
        Self {
            extensions: Extensions::new(),
        }
    }

    pub fn with_user<U: 'static + Send + Sync>(mut self, user: U) -> Self {
        self.extensions.insert(user);
        self
    }

    pub fn user<U: 'static>(&self) -> Option<&U> {
        self.extensions.get::<U>()
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::new()
    }
}

/// Simplified response type for middleware
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

impl Response {
    pub fn ok() -> Self {
        Self {
            status: 200,
            body: "OK".to_string(),
        }
    }

    pub fn forbidden(message: String) -> Self {
        Self {
            status: 403,
            body: message,
        }
    }
}

/// Extensions map for storing request data
pub struct Extensions {
    data: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) {
        self.data.insert(std::any::TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.data
            .get(&std::any::TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware to authorize using a Gate
///
/// # Example
///
/// ```rust
/// use rf_authorization::middleware::{AuthorizeGateMiddleware, Request, Response};
/// use rf_authorization::gates::Gate;
/// use std::sync::Arc;
///
/// #[derive(Clone)]
/// struct User {
///     is_admin: bool,
/// }
///
/// let mut gate = Gate::new();
/// gate.define("admin", Arc::new(|user: &User, _| user.is_admin));
///
/// let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin");
///
/// let request = Request::new().with_user(User { is_admin: true });
///
/// // This would be called by your web framework
/// // let response = middleware.handle(request).await?;
/// ```
pub struct AuthorizeGateMiddleware<U> {
    gate: Arc<Gate<U>>,
    ability: String,
}

impl<U> AuthorizeGateMiddleware<U> {
    pub fn new(gate: Arc<Gate<U>>, ability: impl Into<String>) -> Self {
        Self {
            gate,
            ability: ability.into(),
        }
    }
}

#[async_trait]
impl<U: 'static + Send + Sync> Middleware for AuthorizeGateMiddleware<U> {
    async fn handle(&self, request: Request) -> AuthorizationResult<Response> {
        let user = request
            .user::<U>()
            .ok_or_else(|| AuthorizationError::Unauthorized("No user in request".to_string()))?;

        self.gate.authorize(user, &self.ability)?;

        Ok(Response::ok())
    }
}

/// Middleware to authorize using a Policy
///
/// # Example
///
/// ```rust
/// use rf_authorization::middleware::{AuthorizePolicyMiddleware, Request, Response};
/// use rf_authorization::policies::{Policy, PolicyRegistry};
/// use std::sync::Arc;
///
/// #[derive(Clone)]
/// struct User {
///     id: i64,
/// }
///
/// struct Post {
///     id: i64,
///     author_id: i64,
/// }
///
/// struct PostPolicy;
///
/// impl Policy<Post> for PostPolicy {
///     type User = User;
///
///     fn update(&self, user: &User, post: &Post) -> bool {
///         user.id == post.author_id
///     }
/// }
///
/// let mut registry = PolicyRegistry::new();
/// registry.register::<Post, PostPolicy>(PostPolicy);
///
/// let middleware = AuthorizePolicyMiddleware::<User, Post>::new(
///     Arc::new(registry),
///     "update"
/// );
/// ```
pub struct AuthorizePolicyMiddleware<U, T> {
    registry: Arc<PolicyRegistry>,
    ability: String,
    _phantom: std::marker::PhantomData<(U, T)>,
}

impl<U, T> AuthorizePolicyMiddleware<U, T> {
    pub fn new(registry: Arc<PolicyRegistry>, ability: impl Into<String>) -> Self {
        Self {
            registry,
            ability: ability.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<U: 'static + Send + Sync, T: 'static + Send + Sync> Middleware for AuthorizePolicyMiddleware<U, T> {
    async fn handle(&self, request: Request) -> AuthorizationResult<Response> {
        let user = request
            .user::<U>()
            .ok_or_else(|| AuthorizationError::Unauthorized("No user in request".to_string()))?;

        // In a real application, you would extract the model from the request
        // For now, we just check if the user has the ability (without a specific model)
        // This would typically be used with viewAny, create, etc.
        self.registry.authorize::<T, U>(user, &self.ability, None)?;

        Ok(Response::ok())
    }
}

/// Middleware to check multiple abilities (user must have ALL)
pub struct RequireAllMiddleware<U> {
    gate: Arc<Gate<U>>,
    abilities: Vec<String>,
}

impl<U> RequireAllMiddleware<U> {
    pub fn new(gate: Arc<Gate<U>>, abilities: Vec<String>) -> Self {
        Self { gate, abilities }
    }
}

#[async_trait]
impl<U: 'static + Send + Sync> Middleware for RequireAllMiddleware<U> {
    async fn handle(&self, request: Request) -> AuthorizationResult<Response> {
        let user = request
            .user::<U>()
            .ok_or_else(|| AuthorizationError::Unauthorized("No user in request".to_string()))?;

        let ability_refs: Vec<&str> = self.abilities.iter().map(|s| s.as_str()).collect();
        if !self.gate.allows_all(user, &ability_refs) {
            return Err(AuthorizationError::Forbidden(format!(
                "User does not have all required abilities: {:?}",
                self.abilities
            )));
        }

        Ok(Response::ok())
    }
}

/// Middleware to check multiple abilities (user must have ANY)
pub struct RequireAnyMiddleware<U> {
    gate: Arc<Gate<U>>,
    abilities: Vec<String>,
}

impl<U> RequireAnyMiddleware<U> {
    pub fn new(gate: Arc<Gate<U>>, abilities: Vec<String>) -> Self {
        Self { gate, abilities }
    }
}

#[async_trait]
impl<U: 'static + Send + Sync> Middleware for RequireAnyMiddleware<U> {
    async fn handle(&self, request: Request) -> AuthorizationResult<Response> {
        let user = request
            .user::<U>()
            .ok_or_else(|| AuthorizationError::Unauthorized("No user in request".to_string()))?;

        let ability_refs: Vec<&str> = self.abilities.iter().map(|s| s.as_str()).collect();
        if !self.gate.allows_any(user, &ability_refs) {
            return Err(AuthorizationError::Forbidden(format!(
                "User does not have any of the required abilities: {:?}",
                self.abilities
            )));
        }

        Ok(Response::ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates::Gate;
    use crate::policies::{Policy, PolicyRegistry};

    #[derive(Clone)]
    struct TestUser {
        id: i64,
        is_admin: bool,
        permissions: Vec<String>,
    }

    impl TestUser {
        fn has_permission(&self, permission: &str) -> bool {
            self.permissions.contains(&permission.to_string())
        }
    }

    #[tokio::test]
    async fn test_gate_middleware_allows() {
        let mut gate = Gate::new();
        gate.define("admin", Arc::new(|user: &TestUser, _| user.is_admin));

        let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin");

        let admin_user = TestUser {
            id: 1,
            is_admin: true,
            permissions: vec![],
        };

        let request = Request::new().with_user(admin_user);
        let result = middleware.handle(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_gate_middleware_denies() {
        let mut gate = Gate::new();
        gate.define("admin", Arc::new(|user: &TestUser, _| user.is_admin));

        let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin");

        let regular_user = TestUser {
            id: 2,
            is_admin: false,
            permissions: vec![],
        };

        let request = Request::new().with_user(regular_user);
        let result = middleware.handle(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gate_middleware_no_user() {
        let gate: Gate<TestUser> = Gate::new();
        let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin");

        let request = Request::new();
        let result = middleware.handle(request).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthorizationError::Unauthorized(_)));
    }

    struct TestPost {
        id: i64,
        author_id: i64,
    }

    struct TestPostPolicy;

    impl Policy<TestPost> for TestPostPolicy {
        type User = TestUser;

        fn create(&self, _user: &TestUser) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_policy_middleware() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let middleware = AuthorizePolicyMiddleware::<TestUser, TestPost>::new(
            Arc::new(registry),
            "create",
        );

        let user = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec![],
        };

        let request = Request::new().with_user(user);
        let result = middleware.handle(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require_all_middleware_success() {
        let mut gate = Gate::new();
        gate.define("read", Arc::new(|user: &TestUser, _| {
            user.has_permission("read")
        }));
        gate.define("write", Arc::new(|user: &TestUser, _| {
            user.has_permission("write")
        }));

        let middleware = RequireAllMiddleware::new(
            Arc::new(gate),
            vec!["read".to_string(), "write".to_string()],
        );

        let user = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["read".to_string(), "write".to_string()],
        };

        let request = Request::new().with_user(user);
        let result = middleware.handle(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require_all_middleware_failure() {
        let mut gate = Gate::new();
        gate.define("read", Arc::new(|user: &TestUser, _| {
            user.has_permission("read")
        }));
        gate.define("write", Arc::new(|user: &TestUser, _| {
            user.has_permission("write")
        }));

        let middleware = RequireAllMiddleware::new(
            Arc::new(gate),
            vec!["read".to_string(), "write".to_string()],
        );

        let user = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["read".to_string()],
        };

        let request = Request::new().with_user(user);
        let result = middleware.handle(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_require_any_middleware_success() {
        let mut gate = Gate::new();
        gate.define("read", Arc::new(|user: &TestUser, _| {
            user.has_permission("read")
        }));
        gate.define("write", Arc::new(|user: &TestUser, _| {
            user.has_permission("write")
        }));

        let middleware = RequireAnyMiddleware::new(
            Arc::new(gate),
            vec!["read".to_string(), "write".to_string()],
        );

        let user = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["read".to_string()],
        };

        let request = Request::new().with_user(user);
        let result = middleware.handle(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_require_any_middleware_failure() {
        let mut gate = Gate::new();
        gate.define("read", Arc::new(|user: &TestUser, _| {
            user.has_permission("read")
        }));
        gate.define("write", Arc::new(|user: &TestUser, _| {
            user.has_permission("write")
        }));

        let middleware = RequireAnyMiddleware::new(
            Arc::new(gate),
            vec!["read".to_string(), "write".to_string()],
        );

        let user = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec![],
        };

        let request = Request::new().with_user(user);
        let result = middleware.handle(request).await;

        assert!(result.is_err());
    }
}
