//! Authentication Repository Implementations
//!
//! Database-backed storage for users, sessions, and auth-related data

pub mod session_repository;
pub mod user_repository;

pub use user_repository::{
    InMemoryUserRepository, PostgresUserRepository, RepositoryError, RepositoryResult,
    UserRepository,
};

pub use session_repository::{
    EmailVerificationRepository, PasswordResetRepository, PostgresEmailVerificationRepository,
    PostgresPasswordResetRepository, PostgresSessionRepository, SessionRepository,
};
