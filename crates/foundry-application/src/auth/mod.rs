pub mod guard;
pub mod jwt;
pub mod registry;
pub mod session;
pub mod user;
pub mod database;
pub mod middleware;
pub mod permissions;
pub mod authorization;

pub use guard::{AuthError, Authenticatable, Credentials, Guard, Provider};
pub use jwt::{Claims, JwtConfig, JwtService, TokenPair, TokenType};
pub use registry::GuardRegistry;
pub use session::{Session, SessionGuard, SessionStore, InMemorySessionStore};
pub use user::{InMemoryUserProvider, User};
pub use database::{DatabaseUserProvider, DatabaseSessionStore};
pub use middleware::{JwtAuthLayer, RequireAuth};
pub use permissions::{Permission, Role, HasPermission, HasRole};
pub use authorization::{Gate, Policy, AuthorizationError, AuthorizationResult};
