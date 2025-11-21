//! # rf-breeze - Authentication Scaffolding
//!
//! Laravel Breeze-equivalent authentication scaffolding system for RustForge.
//! Provides complete authentication setup with views, controllers, and routes.
//!
//! ## Features
//!
//! - **Complete Auth System**: Login, Register, Password Reset, Email Verification
//! - **Blade Templates**: Pre-built views compatible with rf-blade
//! - **Controller Generation**: Ready-to-use authentication controllers
//! - **Route Setup**: Automatic route registration
//! - **Middleware**: Auth middleware configuration
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_breeze::{BreezeScaffold, InstallOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let breeze = BreezeScaffold::new(".")?;
//!
//! // Install complete authentication system
//! breeze.install(&InstallOptions {
//!     with_api: false,
//!     with_email_verification: true,
//!     with_password_reset: true,
//!     output_dir: None,
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Installation Options
//!
//! ```rust
//! use rf_breeze::{BreezeScaffold, InstallOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let breeze = BreezeScaffold::new(".")?;
//!
//! // Install only views
//! breeze.install_views().await?;
//!
//! // Install only controllers
//! breeze.install_controllers().await?;
//!
//! // Install only routes
//! breeze.install_routes().await?;
//! # Ok(())
//! # }
//! ```

use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod templates;
pub mod installer;

pub use installer::BreezeInstaller;

/// Breeze scaffold errors
#[derive(Error, Debug)]
pub enum BreezeError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Installation error: {0}")]
    InstallationError(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Handlebars error: {0}")]
    HandlebarsError(#[from] handlebars::RenderError),
}

pub type BreezeResult<T> = Result<T, BreezeError>;

/// Installation options for Breeze scaffolding
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Include API authentication routes
    pub with_api: bool,

    /// Include email verification flow
    pub with_email_verification: bool,

    /// Include password reset flow
    pub with_password_reset: bool,

    /// Custom output directory (defaults to project root)
    pub output_dir: Option<PathBuf>,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            with_api: false,
            with_email_verification: true,
            with_password_reset: true,
            output_dir: None,
        }
    }
}

/// Main Breeze scaffold interface
///
/// # Example
///
/// ```rust,no_run
/// use rf_breeze::{BreezeScaffold, InstallOptions};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let breeze = BreezeScaffold::new(".")?;
///
/// breeze.install(&InstallOptions {
///     with_api: false,
///     with_email_verification: true,
///     with_password_reset: true,
///     output_dir: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct BreezeScaffold {
    /// Base project directory
    base_path: PathBuf,

    /// Internal installer
    installer: BreezeInstaller,
}

impl BreezeScaffold {
    /// Create a new Breeze scaffold
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_breeze::BreezeScaffold;
    ///
    /// let breeze = BreezeScaffold::new(".").unwrap();
    /// ```
    pub fn new<P: AsRef<Path>>(base_path: P) -> BreezeResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        if !base_path.exists() {
            return Err(BreezeError::InvalidPath(
                format!("Base path does not exist: {}", base_path.display())
            ));
        }

        let installer = BreezeInstaller::new(&base_path)?;

        Ok(Self {
            base_path,
            installer,
        })
    }

    /// Install complete authentication system
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_breeze::{BreezeScaffold, InstallOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let breeze = BreezeScaffold::new(".")?;
    ///
    /// breeze.install(&InstallOptions::default()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install(&self, options: &InstallOptions) -> BreezeResult<()> {
        self.installer.install_all(options).await
    }

    /// Install only views
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_breeze::BreezeScaffold;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let breeze = BreezeScaffold::new(".")?;
    /// breeze.install_views().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install_views(&self) -> BreezeResult<()> {
        self.installer.install_views(&InstallOptions::default()).await
    }

    /// Install only controllers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_breeze::BreezeScaffold;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let breeze = BreezeScaffold::new(".")?;
    /// breeze.install_controllers().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install_controllers(&self) -> BreezeResult<()> {
        self.installer.install_controllers(&InstallOptions::default()).await
    }

    /// Install only routes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_breeze::BreezeScaffold;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let breeze = BreezeScaffold::new(".")?;
    /// breeze.install_routes().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install_routes(&self) -> BreezeResult<()> {
        self.installer.install_routes(&InstallOptions::default()).await
    }

    /// Install only middleware
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_breeze::BreezeScaffold;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let breeze = BreezeScaffold::new(".")?;
    /// breeze.install_middleware().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install_middleware(&self) -> BreezeResult<()> {
        self.installer.install_middleware(&InstallOptions::default()).await
    }

    /// Get the base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_breeze_scaffold_creation() {
        let temp = TempDir::new().unwrap();
        let breeze = BreezeScaffold::new(temp.path()).unwrap();
        assert_eq!(breeze.base_path(), temp.path());
    }

    #[tokio::test]
    async fn test_breeze_scaffold_invalid_path() {
        let result = BreezeScaffold::new("/nonexistent/path");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_install_options_default() {
        let options = InstallOptions::default();
        assert!(!options.with_api);
        assert!(options.with_email_verification);
        assert!(options.with_password_reset);
        assert!(options.output_dir.is_none());
    }
}
