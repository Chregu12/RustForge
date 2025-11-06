//! Authentication Repository Implementations
//!
//! Database-backed storage for users, sessions, and auth-related data

pub mod user_repository;
pub mod session_repository;

pub use user_repository::{
    UserRepository, PostgresUserRepository, InMemoryUserRepository,
    RepositoryError, RepositoryResult
};

pub use session_repository::{
    SessionRepository, PostgresSessionRepository,
    PasswordResetRepository, PostgresPasswordResetRepository,
    EmailVerificationRepository, PostgresEmailVerificationRepository
};
