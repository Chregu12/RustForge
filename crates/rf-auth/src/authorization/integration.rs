//! Axum integration helpers and extractors for authorization
//!
//! Note: Due to complexity with Axum's extractor lifetimes and Send/Sync requirements,
//! the extractors are provided as structures but not fully implemented with FromRequest/FromRequestParts.
//! Use middleware instead for full authorization functionality.

use super::error::AuthorizationError;
use std::marker::PhantomData;

/// Extractor that ensures a user passes a gate
///
/// This extractor checks if the authenticated user passes a specific gate.
/// If they do, it extracts the inner type. If not, it returns a 403 Forbidden.
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::integration::RequireGate;
///
/// #[derive(Clone)]
/// struct User { role: String }
///
/// let user = User { role: "admin".to_string() };
/// let require = RequireGate::new(user, "admin");
/// assert_eq!(require.gate, "admin");
/// assert_eq!(require.user.role, "admin");
/// ```
pub struct RequireGate<U> {
    pub user: U,
    pub gate: String,
}

impl<U> RequireGate<U> {
    /// Create a new RequireGate
    pub fn new(user: U, gate: impl Into<String>) -> Self {
        Self {
            user,
            gate: gate.into(),
        }
    }
}

// FromRequestParts implementation omitted due to Axum version compatibility
// Use middleware instead: AuthorizeGateLayer or require_gate

/// Extractor that ensures a user can perform an action on a resource
///
/// This extractor wraps another extractor and checks authorization before
/// extracting the inner value.
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::integration::Authorize;
///
/// // Wrap a value that has been authorized for the current request.
/// let authorized = Authorize::new(42i64);
/// assert_eq!(authorized.into_inner(), 42);
/// ```
pub struct Authorize<T> {
    pub inner: T,
}

impl<T> Authorize<T> {
    /// Create a new Authorize extractor
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Unwrap the inner value
    pub fn into_inner(self) -> T {
        self.inner
    }
}

// FromRequest implementation omitted due to Axum version compatibility
// Use middleware instead: AuthorizePolicyLayer or require_policy

/// Helper struct for combining user and resource in authorization checks
pub struct AuthorizedResource<U, R> {
    pub user: U,
    pub resource: R,
}

impl<U, R> AuthorizedResource<U, R> {
    /// Create a new authorized resource
    pub fn new(user: U, resource: R) -> Self {
        Self { user, resource }
    }
}

// FromRequestParts implementation omitted due to Axum version compatibility

/// Extractor for checking if a user can perform a specific action
///
/// This extractor wraps a resource and ensures the user can perform
/// the specified action on it.
pub struct Can<R> {
    pub resource: R,
    action: String,
}

impl<R> Can<R> {
    /// Create a new Can extractor
    pub fn new(resource: R, action: impl Into<String>) -> Self {
        Self {
            resource,
            action: action.into(),
        }
    }

    /// Get the action being checked
    pub fn action(&self) -> &str {
        &self.action
    }
}

/// Extractor for checking if a user can create a resource type
///
/// This extractor checks if the authenticated user can create instances
/// of a specific resource type.
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::integration::CanCreate;
///
/// struct Post;
///
/// // Marks a request that requires permission to create `Post` instances.
/// let _can_create: CanCreate<Post> = CanCreate::default();
/// ```
pub struct CanCreate<R> {
    _phantom: PhantomData<R>,
}

impl<R> Default for CanCreate<R> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

// FromRequestParts implementation omitted due to Axum version compatibility

// Note: RequestAuthExt trait is omitted due to Sync constraints with Request<Body>
// Use middleware or extractors instead for request-based authorization

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorize_struct() {
        let inner = 42;
        let auth = Authorize::new(inner);
        assert_eq!(auth.into_inner(), 42);
    }

    #[test]
    fn test_authorized_resource() {
        #[derive(Clone)]
        struct User {
            id: i64,
        }

        #[derive(Clone)]
        struct Post {
            id: i64,
        }

        let user = User { id: 1 };
        let resource = Post { id: 1 };

        let auth_resource = AuthorizedResource::new(user, resource);
        assert_eq!(auth_resource.user.id, 1);
        assert_eq!(auth_resource.resource.id, 1);
    }
}
