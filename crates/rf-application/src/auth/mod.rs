pub mod authorization;
pub mod database;
pub mod guard;
pub mod jwt;
pub mod middleware;
pub mod permissions;
pub mod registry;
pub mod session;
pub mod user;

pub use authorization::{AuthorizationError, AuthorizationResult, Gate, Policy};
pub use database::{DatabaseSessionStore, DatabaseUserProvider};
pub use guard::{AuthError, Authenticatable, Credentials, Guard, Provider};
pub use jwt::{Claims, JwtConfig, JwtService, TokenPair, TokenType};
pub use middleware::{JwtAuthLayer, RequireAuth};
pub use permissions::{HasPermission, HasRole, Permission, Role};
pub use registry::GuardRegistry;
pub use session::{InMemorySessionStore, Session, SessionGuard, SessionStore};
pub use user::{InMemoryUserProvider, User};
