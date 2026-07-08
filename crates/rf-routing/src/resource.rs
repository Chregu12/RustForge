//! Resource routing for RESTful APIs.
//!
//! This module provides Laravel-like resource routing with:
//! - Standard RESTful routes
//! - Resource filtering (only/except)
//! - Shallow nesting
//! - Nested resources
//! - API resources

use crate::controller::ControllerAction;
use std::collections::HashSet;

/// Resource route configuration.
#[derive(Debug, Clone)]
pub struct ResourceRouter {
    name: String,
    only: Option<HashSet<ControllerAction>>,
    except: Option<HashSet<ControllerAction>>,
    shallow: bool,
    nested: Vec<ResourceRouter>,
    api_only: bool,
}

impl ResourceRouter {
    /// Create a new resource router.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::ResourceRouter;
    ///
    /// let resource = ResourceRouter::new("posts");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            only: None,
            except: None,
            shallow: false,
            nested: Vec::new(),
            api_only: false,
        }
    }

    /// Only include specific actions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::{ResourceRouter, ControllerAction};
    ///
    /// let resource = ResourceRouter::new("posts")
    ///     .only(vec![ControllerAction::Index, ControllerAction::Show]);
    /// ```
    pub fn only(mut self, actions: Vec<ControllerAction>) -> Self {
        self.only = Some(actions.into_iter().collect());
        self
    }

    /// Exclude specific actions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::{ResourceRouter, ControllerAction};
    ///
    /// let resource = ResourceRouter::new("posts")
    ///     .except(vec![ControllerAction::Create, ControllerAction::Edit]);
    /// ```
    pub fn except(mut self, actions: Vec<ControllerAction>) -> Self {
        self.except = Some(actions.into_iter().collect());
        self
    }

    /// Enable shallow nesting for this resource.
    ///
    /// Shallow nesting generates URLs without parent IDs for non-nested actions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::ResourceRouter;
    ///
    /// let resource = ResourceRouter::new("comments").shallow();
    /// // Generates: /comments/{id} instead of /posts/{post_id}/comments/{id}
    /// ```
    pub fn shallow(mut self) -> Self {
        self.shallow = true;
        self
    }

    /// Make this an API resource (no create/edit forms).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::ResourceRouter;
    ///
    /// let resource = ResourceRouter::new("posts").api_resource();
    /// ```
    pub fn api_resource(mut self) -> Self {
        self.api_only = true;
        self
    }

    /// Add a nested resource.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::ResourceRouter;
    ///
    /// let posts = ResourceRouter::new("posts")
    ///     .nest(ResourceRouter::new("comments"));
    /// ```
    pub fn nest(mut self, resource: ResourceRouter) -> Self {
        self.nested.push(resource);
        self
    }

    /// Get the resource name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the actions to include.
    pub fn actions(&self) -> Vec<ControllerAction> {
        let base_actions = if self.api_only {
            ControllerAction::resource_actions()
        } else {
            ControllerAction::all()
        };

        if let Some(only) = &self.only {
            return base_actions
                .into_iter()
                .filter(|a| only.contains(a))
                .collect();
        }

        if let Some(except) = &self.except {
            return base_actions
                .into_iter()
                .filter(|a| !except.contains(a))
                .collect();
        }

        base_actions
    }

    /// Check if an action should be included.
    pub fn should_include(&self, action: &ControllerAction) -> bool {
        if self.api_only
            && (*action == ControllerAction::Create || *action == ControllerAction::Edit)
        {
            return false;
        }

        if let Some(only) = &self.only {
            return only.contains(action);
        }

        if let Some(except) = &self.except {
            return !except.contains(action);
        }

        true
    }

    /// Check if this resource uses shallow nesting.
    pub fn is_shallow(&self) -> bool {
        self.shallow
    }

    /// Check if this is an API-only resource.
    pub fn is_api_only(&self) -> bool {
        self.api_only
    }

    /// Get nested resources.
    pub fn nested_resources(&self) -> &[ResourceRouter] {
        &self.nested
    }

    /// Generate route paths for this resource.
    pub fn paths(&self, parent_path: Option<&str>) -> Vec<(ControllerAction, String)> {
        let mut paths = Vec::new();
        let base_path = if let Some(parent) = parent_path {
            format!("{}/{}", parent, self.name)
        } else {
            format!("/{}", self.name)
        };

        for action in self.actions() {
            let path = match action {
                ControllerAction::Index => base_path.clone(),
                ControllerAction::Create => format!("{}/create", base_path),
                ControllerAction::Store => base_path.clone(),
                ControllerAction::Show => {
                    if self.shallow && parent_path.is_some() {
                        format!("/{}/{{id}}", self.name)
                    } else {
                        format!("{}/{{id}}", base_path)
                    }
                }
                ControllerAction::Edit => {
                    if self.shallow && parent_path.is_some() {
                        format!("/{}/{{id}}/edit", self.name)
                    } else {
                        format!("{}/{{id}}/edit", base_path)
                    }
                }
                ControllerAction::Update => {
                    if self.shallow && parent_path.is_some() {
                        format!("/{}/{{id}}", self.name)
                    } else {
                        format!("{}/{{id}}", base_path)
                    }
                }
                ControllerAction::Destroy => {
                    if self.shallow && parent_path.is_some() {
                        format!("/{}/{{id}}", self.name)
                    } else {
                        format!("{}/{{id}}", base_path)
                    }
                }
            };

            paths.push((action, path));
        }

        paths
    }

    /// Generate route names for this resource.
    pub fn route_names(&self, parent_name: Option<&str>) -> Vec<(ControllerAction, String)> {
        let base_name = if let Some(parent) = parent_name {
            format!("{}.{}", parent, self.name)
        } else {
            self.name.clone()
        };

        self.actions()
            .into_iter()
            .map(|action| {
                let name = format!("{}.{}", base_name, action.as_str());
                (action, name)
            })
            .collect()
    }
}

/// Builder for creating multiple resources.
#[derive(Debug, Clone, Default)]
pub struct ResourceCollection {
    resources: Vec<ResourceRouter>,
}

impl ResourceCollection {
    /// Create a new resource collection.
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    /// Add a resource to the collection.
    pub fn add(mut self, resource: ResourceRouter) -> Self {
        self.resources.push(resource);
        self
    }

    /// Get all resources.
    pub fn resources(&self) -> &[ResourceRouter] {
        &self.resources
    }

    /// Get resources by name.
    pub fn find(&self, name: &str) -> Option<&ResourceRouter> {
        self.resources.iter().find(|r| r.name() == name)
    }
}

/// Helper function to create an API resource.
///
/// # Example
///
/// ```rust
/// use rf_routing::api_resource;
///
/// let posts = api_resource("posts");
/// ```
pub fn api_resource(name: impl Into<String>) -> ResourceRouter {
    ResourceRouter::new(name).api_resource()
}

/// Helper function to create a resource with only specific actions.
///
/// # Example
///
/// ```rust
/// use rf_routing::{resource_only, ControllerAction};
///
/// let posts = resource_only("posts", vec![ControllerAction::Index, ControllerAction::Show]);
/// ```
pub fn resource_only(name: impl Into<String>, actions: Vec<ControllerAction>) -> ResourceRouter {
    ResourceRouter::new(name).only(actions)
}

/// Helper function to create a resource except specific actions.
///
/// # Example
///
/// ```rust
/// use rf_routing::{resource_except, ControllerAction};
///
/// let posts = resource_except("posts", vec![ControllerAction::Destroy]);
/// ```
pub fn resource_except(name: impl Into<String>, actions: Vec<ControllerAction>) -> ResourceRouter {
    ResourceRouter::new(name).except(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_router_creation() {
        let resource = ResourceRouter::new("posts");
        assert_eq!(resource.name(), "posts");
        assert!(!resource.is_shallow());
        assert!(!resource.is_api_only());
    }

    #[test]
    fn test_resource_router_only() {
        let resource = ResourceRouter::new("posts")
            .only(vec![ControllerAction::Index, ControllerAction::Show]);

        let actions = resource.actions();
        assert_eq!(actions.len(), 2);
        assert!(resource.should_include(&ControllerAction::Index));
        assert!(resource.should_include(&ControllerAction::Show));
        assert!(!resource.should_include(&ControllerAction::Store));
    }

    #[test]
    fn test_resource_router_except() {
        let resource = ResourceRouter::new("posts")
            .except(vec![ControllerAction::Create, ControllerAction::Edit]);

        let actions = resource.actions();
        assert_eq!(actions.len(), 5);
        assert!(!resource.should_include(&ControllerAction::Create));
        assert!(!resource.should_include(&ControllerAction::Edit));
        assert!(resource.should_include(&ControllerAction::Index));
    }

    #[test]
    fn test_resource_router_shallow() {
        let resource = ResourceRouter::new("comments").shallow();
        assert!(resource.is_shallow());
    }

    #[test]
    fn test_resource_router_api_resource() {
        let resource = ResourceRouter::new("posts").api_resource();
        assert!(resource.is_api_only());

        let actions = resource.actions();
        assert!(!actions.contains(&ControllerAction::Create));
        assert!(!actions.contains(&ControllerAction::Edit));
        assert_eq!(actions.len(), 5);
    }

    #[test]
    fn test_resource_router_nest() {
        let posts = ResourceRouter::new("posts").nest(ResourceRouter::new("comments"));

        assert_eq!(posts.nested_resources().len(), 1);
        assert_eq!(posts.nested_resources()[0].name(), "comments");
    }

    #[test]
    fn test_resource_router_paths() {
        let resource = ResourceRouter::new("posts");
        let paths = resource.paths(None);

        assert_eq!(paths.len(), 7);

        let index_path = paths
            .iter()
            .find(|(a, _)| *a == ControllerAction::Index)
            .map(|(_, p)| p);
        assert_eq!(index_path, Some(&"/posts".to_string()));

        let show_path = paths
            .iter()
            .find(|(a, _)| *a == ControllerAction::Show)
            .map(|(_, p)| p);
        assert_eq!(show_path, Some(&"/posts/{id}".to_string()));
    }

    #[test]
    fn test_resource_router_paths_nested() {
        let resource = ResourceRouter::new("comments");
        let paths = resource.paths(Some("/posts/{post_id}"));

        let index_path = paths
            .iter()
            .find(|(a, _)| *a == ControllerAction::Index)
            .map(|(_, p)| p);
        assert_eq!(index_path, Some(&"/posts/{post_id}/comments".to_string()));
    }

    #[test]
    fn test_resource_router_paths_shallow() {
        let resource = ResourceRouter::new("comments").shallow();
        let paths = resource.paths(Some("/posts/{post_id}"));

        let show_path = paths
            .iter()
            .find(|(a, _)| *a == ControllerAction::Show)
            .map(|(_, p)| p);
        assert_eq!(show_path, Some(&"/comments/{id}".to_string()));
    }

    #[test]
    fn test_resource_router_route_names() {
        let resource = ResourceRouter::new("posts");
        let names = resource.route_names(None);

        assert_eq!(names.len(), 7);

        let index_name = names
            .iter()
            .find(|(a, _)| *a == ControllerAction::Index)
            .map(|(_, n)| n);
        assert_eq!(index_name, Some(&"posts.index".to_string()));
    }

    #[test]
    fn test_resource_router_route_names_nested() {
        let resource = ResourceRouter::new("comments");
        let names = resource.route_names(Some("posts"));

        let index_name = names
            .iter()
            .find(|(a, _)| *a == ControllerAction::Index)
            .map(|(_, n)| n);
        assert_eq!(index_name, Some(&"posts.comments.index".to_string()));
    }

    #[test]
    fn test_resource_collection() {
        let collection = ResourceCollection::new()
            .add(ResourceRouter::new("posts"))
            .add(ResourceRouter::new("users"));

        assert_eq!(collection.resources().len(), 2);
        assert!(collection.find("posts").is_some());
        assert!(collection.find("users").is_some());
        assert!(collection.find("comments").is_none());
    }

    #[test]
    fn test_api_resource_helper() {
        let resource = api_resource("posts");
        assert!(resource.is_api_only());
        assert_eq!(resource.actions().len(), 5);
    }

    #[test]
    fn test_resource_only_helper() {
        let resource = resource_only("posts", vec![ControllerAction::Index]);
        assert_eq!(resource.actions().len(), 1);
    }

    #[test]
    fn test_resource_except_helper() {
        let resource = resource_except("posts", vec![ControllerAction::Destroy]);
        let actions = resource.actions();
        assert!(!actions.contains(&ControllerAction::Destroy));
    }
}
