use async_trait::async_trait;
use crate::container::Container;
use crate::error::Result;
use crate::provider::ServiceProvider;

/// Auth service provider for authentication and authorization services
pub struct AuthServiceProvider;

impl AuthServiceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceProvider for AuthServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        // Register auth driver
        container
            .singleton("auth.driver", || {
                Ok(std::env::var("AUTH_DRIVER").unwrap_or_else(|_| "session".to_string()))
            })
            .await?;

        // Register JWT secret
        container
            .singleton("auth.jwt_secret", || {
                Ok(std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                    std::env::var("APP_KEY").unwrap_or_default()
                }))
            })
            .await?;

        // Register JWT expiration
        container
            .singleton("auth.jwt_ttl", || {
                Ok(std::env::var("JWT_TTL")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse::<u64>()
                    .unwrap_or(60))
            })
            .await?;

        // Register session lifetime
        container
            .singleton("auth.session_lifetime", || {
                Ok(std::env::var("SESSION_LIFETIME")
                    .unwrap_or_else(|_| "120".to_string())
                    .parse::<u64>()
                    .unwrap_or(120))
            })
            .await?;

        // Register password hashing algorithm
        container
            .singleton("auth.password_algo", || {
                Ok(std::env::var("PASSWORD_ALGO").unwrap_or_else(|_| "bcrypt".to_string()))
            })
            .await?;

        Ok(())
    }

    async fn boot(&self, _container: &Container) -> Result<()> {
        // Could initialize auth guards, middleware, etc.
        Ok(())
    }

    fn name(&self) -> &str {
        "AuthServiceProvider"
    }

    fn defer(&self) -> Vec<String> {
        // Auth services can be lazy loaded
        vec![
            "auth.driver".to_string(),
            "auth.jwt_secret".to_string(),
            "auth.jwt_ttl".to_string(),
            "auth.session_lifetime".to_string(),
            "auth.password_algo".to_string(),
        ]
    }
}

impl Default for AuthServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
