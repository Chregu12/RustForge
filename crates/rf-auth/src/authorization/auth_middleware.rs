//! Authorization middleware for Axum

use super::error::AuthorizationError;
use super::gates::Gate;
use super::registry::global_registry;
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use tower::{Layer, Service};

/// Layer for gate-based authorization
///
/// This layer checks if a user passes a gate before allowing the request to proceed.
///
/// # Example
///
/// ```rust
/// use axum::{Router, routing::get};
/// use rf_auth::authorization::auth_middleware::AuthorizeGateLayer;
///
/// # async fn admin_handler() -> &'static str { "Admin only" }
/// # fn example() {
/// let app = Router::new()
///     .route("/admin", get(admin_handler))
///     .layer(AuthorizeGateLayer::new("admin"));
/// # }
/// ```
#[derive(Clone)]
pub struct AuthorizeGateLayer<U = ()> {
    gate_name: String,
    _phantom: PhantomData<U>,
}

impl<U> AuthorizeGateLayer<U> {
    /// Create a new gate authorization layer
    pub fn new(gate_name: impl Into<String>) -> Self {
        Self {
            gate_name: gate_name.into(),
            _phantom: PhantomData,
        }
    }
}

impl<S, U> Layer<S> for AuthorizeGateLayer<U>
where
    U: Send + Sync + 'static,
{
    type Service = AuthorizeGateMiddleware<S, U>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizeGateMiddleware {
            inner,
            gate_name: self.gate_name.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Middleware service for gate-based authorization
#[derive(Clone)]
pub struct AuthorizeGateMiddleware<S, U = ()> {
    inner: S,
    gate_name: String,
    _phantom: PhantomData<U>,
}

impl<S, U> Service<Request> for AuthorizeGateMiddleware<S, U>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    U: Clone + Send + Sync + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let inner = self.inner.clone();
        let gate_name = self.gate_name.clone();

        Box::pin(async move {
            // Try to extract user from request extensions
            let user = req.extensions().get::<U>().cloned();

            if let Some(user) = user {
                // Get gate from request extensions or create a new one
                let gate: Gate<U> = req
                    .extensions()
                    .get::<Gate<U>>()
                    .cloned()
                    .unwrap_or_else(Gate::new);

                // Check authorization
                if gate.allows(&user, &gate_name).await {
                    let mut inner = inner.clone();
                    return inner.call(req).await;
                } else {
                    return Ok(AuthorizationError::Forbidden(format!(
                        "Gate '{}' denied access",
                        gate_name
                    ))
                    .into_response());
                }
            } else {
                return Ok(AuthorizationError::UserNotFound.into_response());
            }
        })
    }
}

/// Layer for policy-based authorization
///
/// This layer checks if a user can perform an action on a resource before
/// allowing the request to proceed.
///
/// # Example
///
/// ```rust
/// use axum::{Router, routing::put};
/// use rf_auth::authorization::auth_middleware::AuthorizePolicyLayer;
///
/// struct Post;
///
/// # async fn update_post_handler() -> &'static str { "Updated" }
/// # fn example() {
/// let app = Router::new()
///     .route("/posts/:id", put(update_post_handler))
///     .layer(AuthorizePolicyLayer::<(), Post>::new("update"));
/// # }
/// ```
#[derive(Clone)]
pub struct AuthorizePolicyLayer<U = (), R = ()> {
    action: String,
    _phantom: PhantomData<(U, R)>,
}

impl<U, R> AuthorizePolicyLayer<U, R> {
    /// Create a new policy authorization layer
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            _phantom: PhantomData,
        }
    }
}

impl<S, U, R> Layer<S> for AuthorizePolicyLayer<U, R>
where
    U: Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    type Service = AuthorizePolicyMiddleware<S, U, R>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizePolicyMiddleware {
            inner,
            action: self.action.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Middleware service for policy-based authorization
#[derive(Clone)]
pub struct AuthorizePolicyMiddleware<S, U = (), R = ()> {
    inner: S,
    action: String,
    _phantom: PhantomData<(U, R)>,
}

impl<S, U, R> Service<Request> for AuthorizePolicyMiddleware<S, U, R>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    U: Clone + Send + Sync + 'static,
    R: Clone + Send + Sync + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let inner = self.inner.clone();
        let action = self.action.clone();

        Box::pin(async move {
            use std::any::TypeId;
            use std::sync::Arc;

            // Try to extract user and resource from request extensions
            let user_opt = req.extensions().get::<U>().cloned();
            let resource_opt = req.extensions().get::<R>().cloned();

            match (user_opt, resource_opt) {
                (Some(user), Some(resource)) => {
                    // Clone the policy Arc before releasing the lock
                    let policy_opt = {
                        let registry = global_registry().lock().unwrap();
                        let type_id = TypeId::of::<R>();
                        registry.policies.get(&type_id).map(|arc| Arc::clone(arc))
                    };

                    let authorized = if let Some(policy_arc_any) = policy_opt {
                        if let Some(policy_arc) = policy_arc_any
                            .downcast_ref::<Arc<dyn crate::authorization::policies::Policy<U, R>>>()
                        {
                            // Check before hook
                            if let Some(result) = policy_arc.before(&user, &resource).await {
                                result
                            } else {
                                // Check specific action
                                match action.as_str() {
                                    "view" => policy_arc.view(&user, &resource).await,
                                    "update" => policy_arc.update(&user, &resource).await,
                                    "delete" => policy_arc.delete(&user, &resource).await,
                                    "restore" => policy_arc.restore(&user, &resource).await,
                                    "force_delete" => {
                                        policy_arc.force_delete(&user, &resource).await
                                    }
                                    _ => false,
                                }
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if authorized {
                        let mut inner = inner.clone();
                        return inner.call(req).await;
                    } else {
                        return Ok(AuthorizationError::Forbidden(format!(
                            "Action '{}' denied on resource",
                            action
                        ))
                        .into_response());
                    }
                }
                (None, _) => Ok(AuthorizationError::UserNotFound.into_response()),
                (_, None) => Ok(AuthorizationError::ResourceNotFound.into_response()),
            }
        })
    }
}

/// Middleware function for gate-based authorization
///
/// This is a convenience function that can be used with Axum's `middleware::from_fn`.
///
/// # Example
///
/// ```rust
/// use axum::{Router, routing::get, middleware};
/// use rf_auth::authorization::auth_middleware::require_gate;
///
/// # async fn admin_handler() -> &'static str { "Admin only" }
/// # fn example() {
/// let app = Router::new()
///     .route("/admin", get(admin_handler))
///     .layer(middleware::from_fn(|req, next| {
///         require_gate(req, next, "admin")
///     }));
/// # }
/// ```
pub async fn require_gate<U>(
    req: Request,
    next: Next,
    gate_name: impl Into<String>,
) -> Result<Response, Response>
where
    U: Clone + Send + Sync + 'static,
{
    let gate_name = gate_name.into();

    // Try to extract user from request extensions
    let user = req.extensions().get::<U>().cloned();

    if let Some(user) = user {
        // Get gate from request extensions or create a new one
        let gate: Gate<U> = req
            .extensions()
            .get::<Gate<U>>()
            .cloned()
            .unwrap_or_else(Gate::new);

        // Check authorization
        if gate.allows(&user, &gate_name).await {
            Ok(next.run(req).await)
        } else {
            Err(
                AuthorizationError::Forbidden(format!("Gate '{}' denied access", gate_name))
                    .into_response(),
            )
        }
    } else {
        Err(AuthorizationError::UserNotFound.into_response())
    }
}

/// Middleware function for policy-based authorization
///
/// This is a convenience function that can be used with Axum's `middleware::from_fn`.
pub async fn require_policy<U, R>(
    req: Request,
    next: Next,
    action: impl Into<String>,
) -> Result<Response, Response>
where
    U: Clone + Send + Sync + 'static,
    R: Clone + Send + Sync + 'static,
{
    use std::any::TypeId;
    use std::sync::Arc;

    let action = action.into();

    // Try to extract user and resource from request extensions
    let user_opt = req.extensions().get::<U>().cloned();
    let resource_opt = req.extensions().get::<R>().cloned();

    match (user_opt, resource_opt) {
        (Some(user), Some(resource)) => {
            // Clone the policy Arc before releasing the lock
            let policy_opt = {
                let registry = global_registry().lock().unwrap();
                let type_id = TypeId::of::<R>();
                registry.policies.get(&type_id).map(|arc| Arc::clone(arc))
            };

            let authorized = if let Some(policy_arc_any) = policy_opt {
                if let Some(policy_arc) = policy_arc_any
                    .downcast_ref::<Arc<dyn crate::authorization::policies::Policy<U, R>>>()
                {
                    // Check before hook
                    if let Some(result) = policy_arc.before(&user, &resource).await {
                        result
                    } else {
                        // Check specific action
                        match action.as_str() {
                            "view" => policy_arc.view(&user, &resource).await,
                            "update" => policy_arc.update(&user, &resource).await,
                            "delete" => policy_arc.delete(&user, &resource).await,
                            "restore" => policy_arc.restore(&user, &resource).await,
                            "force_delete" => policy_arc.force_delete(&user, &resource).await,
                            _ => false,
                        }
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if authorized {
                Ok(next.run(req).await)
            } else {
                Err(AuthorizationError::Forbidden(format!(
                    "Action '{}' denied on resource",
                    action
                ))
                .into_response())
            }
        }
        (None, _) => Err(AuthorizationError::UserNotFound.into_response()),
        (_, None) => Err(AuthorizationError::ResourceNotFound.into_response()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[derive(Clone, Debug)]
    struct TestUser {
        role: String,
    }

    async fn handler() -> &'static str {
        "Success"
    }

    #[tokio::test]
    async fn test_gate_layer_allows_authorized_user() {
        let gate: Gate<TestUser> = Gate::new();
        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });

        let app = Router::new()
            .route("/", get(handler))
            .layer(AuthorizeGateLayer::<TestUser>::new("admin"));

        let admin = TestUser {
            role: "admin".to_string(),
        };

        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut().insert(admin);
        req.extensions_mut().insert(gate);

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_gate_layer_denies_unauthorized_user() {
        let gate: Gate<TestUser> = Gate::new();
        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });

        let app = Router::new()
            .route("/", get(handler))
            .layer(AuthorizeGateLayer::<TestUser>::new("admin"));

        let user = TestUser {
            role: "user".to_string(),
        };

        let mut req = Request::builder().uri("/").body(Body::empty()).unwrap();
        req.extensions_mut().insert(user);
        req.extensions_mut().insert(gate);

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_gate_layer_without_user() {
        let app = Router::new()
            .route("/", get(handler))
            .layer(AuthorizeGateLayer::<TestUser>::new("admin"));

        let req = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
