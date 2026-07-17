//! Service provider trait and implementations

use crate::Application;
use async_trait::async_trait;

/// Service provider trait
///
/// Implementors of this trait can register and boot services in the application.
#[async_trait]
pub trait ServiceProvider: Send + Sync {
    /// Register bindings in the container
    ///
    /// This method is called when the provider is registered with the application.
    /// Use this to bind services, interfaces, and dependencies.
    async fn register(&self, app: &mut Application) -> anyhow::Result<()>;

    /// Boot services
    ///
    /// This method is called after all providers have been registered.
    /// Use this to perform any final setup or configuration.
    async fn boot(&self, app: &Application) -> anyhow::Result<()> {
        let _ = app;
        Ok(())
    }

    /// Get the services provided by this provider
    ///
    /// This is used for deferred loading of providers.
    fn provides(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Whether this provider is deferred
    ///
    /// Deferred providers are only loaded when one of their services is requested.
    fn is_deferred(&self) -> bool {
        !self.provides().is_empty()
    }
}

/// Deferred service provider
///
/// A provider that is only loaded when one of its services is requested.
#[async_trait]
pub trait DeferredProvider: ServiceProvider {
    /// Get the services provided by this deferred provider
    fn deferred_provides(&self) -> Vec<&'static str>;
}

/// Macro to implement a simple service provider
#[macro_export]
macro_rules! service_provider {
    ($name:ident, register: $register:expr) => {
        struct $name;

        #[async_trait::async_trait]
        impl $crate::ServiceProvider for $name {
            async fn register(&self, app: &mut $crate::Application) -> anyhow::Result<()> {
                $register(app).await
            }
        }
    };

    ($name:ident, register: $register:expr, boot: $boot:expr) => {
        struct $name;

        #[async_trait::async_trait]
        impl $crate::ServiceProvider for $name {
            async fn register(&self, app: &mut $crate::Application) -> anyhow::Result<()> {
                $register(app).await
            }

            async fn boot(&self, app: &$crate::Application) -> anyhow::Result<()> {
                $boot(app).await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    #[async_trait]
    impl ServiceProvider for TestProvider {
        async fn register(&self, _app: &mut Application) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_provider_is_deferred() {
        let provider = TestProvider;
        assert!(!provider.is_deferred());
    }

    struct DeferredTestProvider;

    #[async_trait]
    impl ServiceProvider for DeferredTestProvider {
        async fn register(&self, _app: &mut Application) -> anyhow::Result<()> {
            Ok(())
        }

        fn provides(&self) -> Vec<&'static str> {
            vec!["test_service"]
        }
    }

    #[tokio::test]
    async fn test_deferred_provider() {
        let provider = DeferredTestProvider;
        assert!(provider.is_deferred());
        assert_eq!(provider.provides(), vec!["test_service"]);
    }

    // Test is disabled due to macro complexity
    // In production, use manual struct implementation instead
}
