//! Error types for dependency injection container

use thiserror::Error;

/// Container operation errors
#[derive(Debug, Error)]
pub enum ContainerError {
    /// Service not registered in container
    #[error("Service of type '{type_name}' not found in container")]
    ServiceNotFound { type_name: String },

    /// Service factory failed to create instance
    #[error("Failed to create service '{type_name}': {source}")]
    FactoryFailed {
        type_name: String,
        #[source]
        source: anyhow::Error,
    },

    /// Type downcast failed
    #[error("Failed to downcast service to type '{type_name}'")]
    DowncastFailed { type_name: String },

    /// Circular dependency detected
    #[error("Circular dependency detected while resolving '{type_name}'")]
    CircularDependency { type_name: String },
}

/// Result type for container operations
pub type ContainerResult<T> = Result<T, ContainerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ContainerError::ServiceNotFound {
            type_name: "DatabasePool".to_string(),
        };
        assert!(err.to_string().contains("DatabasePool"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_factory_error() {
        let source = anyhow::anyhow!("Connection failed");
        let err = ContainerError::FactoryFailed {
            type_name: "DatabasePool".to_string(),
            source,
        };
        assert!(err.to_string().contains("Failed to create"));
    }

    #[test]
    fn test_downcast_error() {
        let err = ContainerError::DowncastFailed {
            type_name: "String".to_string(),
        };
        assert!(err.to_string().contains("downcast"));
    }
}
