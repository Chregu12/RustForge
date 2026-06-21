//! Controller resolution and routing for RESTful resources.
//!
//! This module provides Laravel-like controller routing with:
//! - Controller traits for standard CRUD operations
//! - Automatic route binding
//! - Action routing
//! - Controller groups

use async_trait::async_trait;
use axum::{
    extract::{Json, Path, State},
    response::{IntoResponse, Response},
};
use serde_json::Value;

/// Standard controller actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerAction {
    /// List all resources (GET /resource)
    Index,
    /// Show create form (GET /resource/create)
    Create,
    /// Store new resource (POST /resource)
    Store,
    /// Show a resource (GET /resource/:id)
    Show,
    /// Show edit form (GET /resource/:id/edit)
    Edit,
    /// Update a resource (PUT/PATCH /resource/:id)
    Update,
    /// Delete a resource (DELETE /resource/:id)
    Destroy,
}

impl ControllerAction {
    /// Get the HTTP method for this action.
    pub fn method(&self) -> &'static str {
        match self {
            Self::Index | Self::Create | Self::Show | Self::Edit => "GET",
            Self::Store => "POST",
            Self::Update => "PUT",
            Self::Destroy => "DELETE",
        }
    }

    /// Get the path pattern for this action.
    pub fn path(&self, resource: &str) -> String {
        match self {
            Self::Index => format!("/{}", resource),
            Self::Create => format!("/{}/create", resource),
            Self::Store => format!("/{}", resource),
            Self::Show => format!("/{}/:id", resource),
            Self::Edit => format!("/{}/:id/edit", resource),
            Self::Update => format!("/{}/:id", resource),
            Self::Destroy => format!("/{}/:id", resource),
        }
    }

    /// Get all standard actions.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Index,
            Self::Create,
            Self::Store,
            Self::Show,
            Self::Edit,
            Self::Update,
            Self::Destroy,
        ]
    }

    /// Get resource actions (index, store, show, update, destroy).
    pub fn resource_actions() -> Vec<Self> {
        vec![
            Self::Index,
            Self::Store,
            Self::Show,
            Self::Update,
            Self::Destroy,
        ]
    }

    /// Parse from string.
    #[allow(clippy::should_implement_trait)] // intentional inherent parser returning Option, not FromStr
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "index" => Some(Self::Index),
            "create" => Some(Self::Create),
            "store" => Some(Self::Store),
            "show" => Some(Self::Show),
            "edit" => Some(Self::Edit),
            "update" => Some(Self::Update),
            "destroy" => Some(Self::Destroy),
            _ => None,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Index => "index",
            Self::Create => "create",
            Self::Store => "store",
            Self::Show => "show",
            Self::Edit => "edit",
            Self::Update => "update",
            Self::Destroy => "destroy",
        }
    }
}

/// Trait for RESTful controllers.
///
/// Implement this trait to create a controller with standard CRUD operations.
#[async_trait]
pub trait Controller: Send + Sync + 'static {
    /// The state type for this controller.
    type State: Clone + Send + Sync + 'static;

    /// List all resources.
    async fn index(&self, _state: State<Self::State>) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }

    /// Show create form.
    async fn create(&self, _state: State<Self::State>) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }

    /// Store a new resource.
    async fn store(&self, _state: State<Self::State>, _payload: Json<Value>) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }

    /// Show a specific resource.
    async fn show(&self, _state: State<Self::State>, _id: Path<String>) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }

    /// Show edit form.
    async fn edit(&self, _state: State<Self::State>, _id: Path<String>) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }

    /// Update a resource.
    async fn update(
        &self,
        _state: State<Self::State>,
        _id: Path<String>,
        _payload: Json<Value>,
    ) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }

    /// Delete a resource.
    async fn destroy(&self, _state: State<Self::State>, _id: Path<String>) -> Response {
        axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()
    }
}

/// Controller registry for managing controllers.
#[derive(Clone)]
pub struct ControllerRegistry<S> {
    controllers: std::sync::Arc<
        parking_lot::RwLock<
            std::collections::HashMap<String, std::sync::Arc<dyn std::any::Any + Send + Sync>>,
        >,
    >,
    _state: std::marker::PhantomData<S>,
}

impl<S> ControllerRegistry<S> {
    /// Create a new controller registry.
    pub fn new() -> Self {
        Self {
            controllers: std::sync::Arc::new(parking_lot::RwLock::new(
                std::collections::HashMap::new(),
            )),
            _state: std::marker::PhantomData,
        }
    }

    /// Register a controller.
    pub fn register<C>(&self, name: impl Into<String>, controller: C)
    where
        C: 'static + Send + Sync,
    {
        self.controllers
            .write()
            .insert(name.into(), std::sync::Arc::new(controller));
    }

    /// Get a controller by name.
    pub fn get<C: 'static + Send + Sync>(&self, name: &str) -> Option<std::sync::Arc<C>> {
        self.controllers
            .read()
            .get(name)
            .and_then(|c| std::sync::Arc::clone(c).downcast::<C>().ok())
    }

    /// Check if a controller exists.
    pub fn has(&self, name: &str) -> bool {
        self.controllers.read().contains_key(name)
    }

    /// Get all controller names.
    pub fn names(&self) -> Vec<String> {
        self.controllers.read().keys().cloned().collect()
    }
}

impl<S> Default for ControllerRegistry<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> std::fmt::Debug for ControllerRegistry<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControllerRegistry")
            .field("count", &self.controllers.read().len())
            .finish()
    }
}

/// Builder for controller routes.
pub struct ControllerRouteBuilder {
    name: String,
    only: Option<Vec<ControllerAction>>,
    except: Option<Vec<ControllerAction>>,
}

impl ControllerRouteBuilder {
    /// Create a new controller route builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            only: None,
            except: None,
        }
    }

    /// Only include specific actions.
    pub fn only(mut self, actions: Vec<ControllerAction>) -> Self {
        self.only = Some(actions);
        self
    }

    /// Exclude specific actions.
    pub fn except(mut self, actions: Vec<ControllerAction>) -> Self {
        self.except = Some(actions);
        self
    }

    /// Get the actions to include.
    pub fn actions(&self) -> Vec<ControllerAction> {
        let all_actions = ControllerAction::all();

        if let Some(only) = &self.only {
            return only.clone();
        }

        if let Some(except) = &self.except {
            return all_actions
                .into_iter()
                .filter(|a| !except.contains(a))
                .collect();
        }

        all_actions
    }

    /// Get the controller name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Helper macro for creating controller routes.
///
/// # Example
///
/// ```rust,ignore
/// use rf_routing::controller;
///
/// let routes = controller! {
///     UserController,
///     only: [Index, Show, Store]
/// };
/// ```
#[macro_export]
macro_rules! controller {
    ($controller:ty) => {{
        $crate::ControllerRouteBuilder::new(stringify!($controller))
    }};

    ($controller:ty, only: [$($action:ident),*]) => {{
        $crate::ControllerRouteBuilder::new(stringify!($controller))
            .only(vec![$($crate::ControllerAction::$action),*])
    }};

    ($controller:ty, except: [$($action:ident),*]) => {{
        $crate::ControllerRouteBuilder::new(stringify!($controller))
            .except(vec![$($crate::ControllerAction::$action),*])
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_action_method() {
        assert_eq!(ControllerAction::Index.method(), "GET");
        assert_eq!(ControllerAction::Store.method(), "POST");
        assert_eq!(ControllerAction::Update.method(), "PUT");
        assert_eq!(ControllerAction::Destroy.method(), "DELETE");
    }

    #[test]
    fn test_controller_action_path() {
        assert_eq!(ControllerAction::Index.path("users"), "/users");
        assert_eq!(ControllerAction::Create.path("users"), "/users/create");
        assert_eq!(ControllerAction::Show.path("users"), "/users/:id");
        assert_eq!(ControllerAction::Edit.path("users"), "/users/:id/edit");
    }

    #[test]
    fn test_controller_action_all() {
        let actions = ControllerAction::all();
        assert_eq!(actions.len(), 7);
    }

    #[test]
    fn test_controller_action_resource_actions() {
        let actions = ControllerAction::resource_actions();
        assert_eq!(actions.len(), 5);
        assert!(actions.contains(&ControllerAction::Index));
        assert!(!actions.contains(&ControllerAction::Create));
    }

    #[test]
    fn test_controller_action_from_str() {
        assert_eq!(
            ControllerAction::from_str("index"),
            Some(ControllerAction::Index)
        );
        assert_eq!(
            ControllerAction::from_str("show"),
            Some(ControllerAction::Show)
        );
        assert_eq!(ControllerAction::from_str("invalid"), None);
    }

    #[test]
    fn test_controller_action_as_str() {
        assert_eq!(ControllerAction::Index.as_str(), "index");
        assert_eq!(ControllerAction::Store.as_str(), "store");
    }

    #[test]
    fn test_controller_registry_creation() {
        let registry: ControllerRegistry<()> = ControllerRegistry::new();
        assert_eq!(registry.names().len(), 0);
    }

    #[test]
    fn test_controller_registry_register() {
        struct TestController;

        let registry: ControllerRegistry<()> = ControllerRegistry::new();
        registry.register("test", TestController);

        assert!(registry.has("test"));
        assert_eq!(registry.names().len(), 1);
    }

    #[test]
    fn test_controller_registry_get() {
        struct TestController;

        let registry: ControllerRegistry<()> = ControllerRegistry::new();
        registry.register("test", TestController);

        let controller = registry.get::<TestController>("test");
        assert!(controller.is_some());

        let missing = registry.get::<TestController>("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_controller_route_builder_creation() {
        let builder = ControllerRouteBuilder::new("UserController");
        assert_eq!(builder.name(), "UserController");
        assert_eq!(builder.actions().len(), 7);
    }

    #[test]
    fn test_controller_route_builder_only() {
        let builder = ControllerRouteBuilder::new("UserController")
            .only(vec![ControllerAction::Index, ControllerAction::Show]);

        let actions = builder.actions();
        assert_eq!(actions.len(), 2);
        assert!(actions.contains(&ControllerAction::Index));
        assert!(actions.contains(&ControllerAction::Show));
    }

    #[test]
    fn test_controller_route_builder_except() {
        let builder = ControllerRouteBuilder::new("UserController")
            .except(vec![ControllerAction::Create, ControllerAction::Edit]);

        let actions = builder.actions();
        assert_eq!(actions.len(), 5);
        assert!(!actions.contains(&ControllerAction::Create));
        assert!(!actions.contains(&ControllerAction::Edit));
    }
}
