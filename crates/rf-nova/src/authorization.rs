//! Authorization integration for Nova
//!
//! Integrates with rf-authorization for policy-based access control.

use serde_json::Value;

/// Resource policy trait
pub trait ResourcePolicy: Send + Sync {
    /// Can the user view any resources?
    fn view_any(_user: Option<&Value>) -> bool {
        true
    }

    /// Can the user view this specific resource?
    fn view(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    /// Can the user create resources?
    fn create(_user: Option<&Value>) -> bool {
        true
    }

    /// Can the user update this specific resource?
    fn update(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    /// Can the user delete this specific resource?
    fn delete(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    /// Can the user restore this specific resource? (soft deletes)
    fn restore(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    /// Can the user force delete this specific resource?
    fn force_delete(_user: Option<&Value>, _model: &Value) -> bool {
        false
    }

    /// Can the user run this action?
    fn run_action(_user: Option<&Value>, _action_uri: &str) -> bool {
        true
    }
}

/// Default policy that allows everything
pub struct AllowAllPolicy;

impl ResourcePolicy for AllowAllPolicy {
    fn view_any(_user: Option<&Value>) -> bool {
        true
    }

    fn view(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    fn create(_user: Option<&Value>) -> bool {
        true
    }

    fn update(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    fn delete(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }
}

/// Admin-only policy
pub struct AdminOnlyPolicy;

impl ResourcePolicy for AdminOnlyPolicy {
    fn view_any(user: Option<&Value>) -> bool {
        is_admin(user)
    }

    fn view(user: Option<&Value>, _model: &Value) -> bool {
        is_admin(user)
    }

    fn create(user: Option<&Value>) -> bool {
        is_admin(user)
    }

    fn update(user: Option<&Value>, _model: &Value) -> bool {
        is_admin(user)
    }

    fn delete(user: Option<&Value>, _model: &Value) -> bool {
        is_admin(user)
    }
}

/// Helper to check if user is admin
fn is_admin(user: Option<&Value>) -> bool {
    if let Some(user) = user {
        if let Some(is_admin) = user.get("is_admin").and_then(|v| v.as_bool()) {
            return is_admin;
        }
        if let Some(role) = user.get("role").and_then(|v| v.as_str()) {
            return role == "admin" || role == "administrator";
        }
    }
    false
}

/// Read-only policy
pub struct ReadOnlyPolicy;

impl ResourcePolicy for ReadOnlyPolicy {
    fn view_any(_user: Option<&Value>) -> bool {
        true
    }

    fn view(_user: Option<&Value>, _model: &Value) -> bool {
        true
    }

    fn create(_user: Option<&Value>) -> bool {
        false
    }

    fn update(_user: Option<&Value>, _model: &Value) -> bool {
        false
    }

    fn delete(_user: Option<&Value>, _model: &Value) -> bool {
        false
    }
}

/// Owner-based policy - users can only edit their own records
pub struct OwnerPolicy {
    pub owner_field: String,
}

impl OwnerPolicy {
    pub fn new() -> Self {
        Self {
            owner_field: "user_id".to_string(),
        }
    }

    pub fn owner_field(mut self, field: impl Into<String>) -> Self {
        self.owner_field = field.into();
        self
    }

    fn is_owner(&self, user: Option<&Value>, model: &Value) -> bool {
        if let Some(user) = user {
            if let (Some(user_id), Some(owner_id)) = (
                user.get("id"),
                model.get(&self.owner_field),
            ) {
                return user_id == owner_id;
            }
        }
        false
    }
}

impl Default for OwnerPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcePolicy for OwnerPolicy {
    fn view_any(_user: Option<&Value>) -> bool {
        true
    }

    fn view(user: Option<&Value>, model: &Value) -> bool {
        let policy = OwnerPolicy::new();
        policy.is_owner(user, model)
    }

    fn create(_user: Option<&Value>) -> bool {
        true
    }

    fn update(user: Option<&Value>, model: &Value) -> bool {
        let policy = OwnerPolicy::new();
        policy.is_owner(user, model)
    }

    fn delete(user: Option<&Value>, model: &Value) -> bool {
        let policy = OwnerPolicy::new();
        policy.is_owner(user, model)
    }
}
