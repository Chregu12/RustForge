//! Installer for Breeze scaffolding
//!
//! Handles file generation and installation of authentication components.

use crate::{BreezeResult, InstallOptions};
use crate::templates::{controllers::*, middleware::*, routes::*, views::*};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Breeze installer
pub struct BreezeInstaller {
    base_path: PathBuf,
}

impl BreezeInstaller {
    /// Create a new installer
    pub fn new<P: AsRef<Path>>(base_path: P) -> BreezeResult<Self> {
        Ok(Self {
            base_path: base_path.as_ref().to_path_buf(),
        })
    }

    /// Install all authentication components
    pub async fn install_all(&self, options: &InstallOptions) -> BreezeResult<()> {
        // Create directory structure
        self.create_directories(options).await?;

        // Install views
        self.install_views(options).await?;

        // Install controllers
        self.install_controllers(options).await?;

        // Install routes
        self.install_routes(options).await?;

        // Install middleware
        self.install_middleware(options).await?;

        Ok(())
    }

    /// Create directory structure
    async fn create_directories(&self, options: &InstallOptions) -> BreezeResult<()> {
        let base = self.get_output_dir(options);

        // Create views directories
        fs::create_dir_all(base.join("resources/views/auth")).await?;
        fs::create_dir_all(base.join("resources/views/layouts")).await?;

        // Create controllers directory
        fs::create_dir_all(base.join("src/controllers/auth")).await?;

        // Create routes directory
        fs::create_dir_all(base.join("src/routes")).await?;

        // Create middleware directory
        fs::create_dir_all(base.join("src/middleware")).await?;

        Ok(())
    }

    /// Install view templates
    pub async fn install_views(&self, options: &InstallOptions) -> BreezeResult<()> {
        let views_dir = self.get_output_dir(options).join("resources/views");

        // Install layout
        self.write_file(&views_dir.join("layouts/app.blade.html"), LAYOUT_APP).await?;

        // Install auth views
        self.write_file(&views_dir.join("auth/login.blade.html"), LOGIN_VIEW).await?;
        self.write_file(&views_dir.join("auth/register.blade.html"), REGISTER_VIEW).await?;

        if options.with_password_reset {
            self.write_file(&views_dir.join("auth/forgot-password.blade.html"), FORGOT_PASSWORD_VIEW).await?;
            self.write_file(&views_dir.join("auth/reset-password.blade.html"), RESET_PASSWORD_VIEW).await?;
        }

        if options.with_email_verification {
            self.write_file(&views_dir.join("auth/verify-email.blade.html"), VERIFY_EMAIL_VIEW).await?;
        }

        // Install dashboard view
        self.write_file(&views_dir.join("dashboard.blade.html"), DASHBOARD_VIEW).await?;

        Ok(())
    }

    /// Install controller templates
    pub async fn install_controllers(&self, options: &InstallOptions) -> BreezeResult<()> {
        let controllers_dir = self.get_output_dir(options).join("src/controllers/auth");

        // Install base controllers
        self.write_file(&controllers_dir.join("login.rs"), LOGIN_CONTROLLER).await?;
        self.write_file(&controllers_dir.join("register.rs"), REGISTER_CONTROLLER).await?;

        if options.with_password_reset {
            self.write_file(&controllers_dir.join("password_reset.rs"), PASSWORD_RESET_CONTROLLER).await?;
        }

        if options.with_email_verification {
            self.write_file(&controllers_dir.join("email_verification.rs"), EMAIL_VERIFICATION_CONTROLLER).await?;
        }

        // Install dashboard controller
        let dashboard_dir = self.get_output_dir(options).join("src/controllers");
        self.write_file(&dashboard_dir.join("dashboard.rs"), DASHBOARD_CONTROLLER).await?;

        // Create mod.rs for auth controllers
        self.create_auth_mod_file(&controllers_dir, options).await?;

        Ok(())
    }

    /// Install route templates
    pub async fn install_routes(&self, options: &InstallOptions) -> BreezeResult<()> {
        let routes_dir = self.get_output_dir(options).join("src/routes");

        // Choose appropriate route template
        let routes_content = if options.with_email_verification {
            AUTH_ROUTES_FULL
        } else if options.with_password_reset {
            AUTH_ROUTES_WITH_PASSWORD_RESET
        } else {
            AUTH_ROUTES
        };

        self.write_file(&routes_dir.join("auth.rs"), routes_content).await?;

        // Install API routes if requested
        if options.with_api {
            self.write_file(&routes_dir.join("api.rs"), API_AUTH_ROUTES).await?;
        }

        Ok(())
    }

    /// Install middleware templates
    pub async fn install_middleware(&self, options: &InstallOptions) -> BreezeResult<()> {
        let middleware_dir = self.get_output_dir(options).join("src/middleware");

        // Install base middleware
        self.write_file(&middleware_dir.join("auth.rs"), AUTH_MIDDLEWARE).await?;
        self.write_file(&middleware_dir.join("guest.rs"), GUEST_MIDDLEWARE).await?;

        if options.with_email_verification {
            self.write_file(&middleware_dir.join("verified.rs"), VERIFIED_MIDDLEWARE).await?;
        }

        // Install role middleware
        self.write_file(&middleware_dir.join("role.rs"), ROLE_MIDDLEWARE).await?;

        // Create mod.rs for middleware
        self.create_middleware_mod_file(&middleware_dir, options).await?;

        Ok(())
    }

    /// Create mod.rs for auth controllers
    async fn create_auth_mod_file(
        &self,
        controllers_dir: &Path,
        options: &InstallOptions,
    ) -> BreezeResult<()> {
        let mut content = String::from("pub mod login;\npub mod register;\n");

        if options.with_password_reset {
            content.push_str("pub mod password_reset;\n");
        }

        if options.with_email_verification {
            content.push_str("pub mod email_verification;\n");
        }

        self.write_file(&controllers_dir.join("mod.rs"), &content).await
    }

    /// Create mod.rs for middleware
    async fn create_middleware_mod_file(
        &self,
        middleware_dir: &Path,
        options: &InstallOptions,
    ) -> BreezeResult<()> {
        let mut content = String::from("pub mod auth;\npub mod guest;\npub mod role;\n");

        if options.with_email_verification {
            content.push_str("pub mod verified;\n");
        }

        self.write_file(&middleware_dir.join("mod.rs"), &content).await
    }

    /// Write content to file
    async fn write_file(&self, path: &Path, content: &str) -> BreezeResult<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;
        Ok(())
    }

    /// Get output directory
    fn get_output_dir(&self, options: &InstallOptions) -> PathBuf {
        options
            .output_dir
            .clone()
            .unwrap_or_else(|| self.base_path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_installer_creation() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        assert_eq!(installer.base_path, temp.path());
    }

    #[tokio::test]
    async fn test_create_directories() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions::default();

        installer.create_directories(&options).await.unwrap();

        assert!(temp.path().join("resources/views/auth").exists());
        assert!(temp.path().join("resources/views/layouts").exists());
        assert!(temp.path().join("src/controllers/auth").exists());
        assert!(temp.path().join("src/routes").exists());
        assert!(temp.path().join("src/middleware").exists());
    }

    #[tokio::test]
    async fn test_install_views() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions::default();

        installer.create_directories(&options).await.unwrap();
        installer.install_views(&options).await.unwrap();

        assert!(temp.path().join("resources/views/layouts/app.blade.html").exists());
        assert!(temp.path().join("resources/views/auth/login.blade.html").exists());
        assert!(temp.path().join("resources/views/auth/register.blade.html").exists());
        assert!(temp.path().join("resources/views/dashboard.blade.html").exists());
    }

    #[tokio::test]
    async fn test_install_controllers() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions::default();

        installer.create_directories(&options).await.unwrap();
        installer.install_controllers(&options).await.unwrap();

        assert!(temp.path().join("src/controllers/auth/login.rs").exists());
        assert!(temp.path().join("src/controllers/auth/register.rs").exists());
        assert!(temp.path().join("src/controllers/auth/mod.rs").exists());
        assert!(temp.path().join("src/controllers/dashboard.rs").exists());
    }

    #[tokio::test]
    async fn test_install_routes() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions::default();

        installer.create_directories(&options).await.unwrap();
        installer.install_routes(&options).await.unwrap();

        assert!(temp.path().join("src/routes/auth.rs").exists());
    }

    #[tokio::test]
    async fn test_install_middleware() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions::default();

        installer.create_directories(&options).await.unwrap();
        installer.install_middleware(&options).await.unwrap();

        assert!(temp.path().join("src/middleware/auth.rs").exists());
        assert!(temp.path().join("src/middleware/guest.rs").exists());
        assert!(temp.path().join("src/middleware/role.rs").exists());
        assert!(temp.path().join("src/middleware/mod.rs").exists());
    }

    #[tokio::test]
    async fn test_install_all() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions::default();

        installer.install_all(&options).await.unwrap();

        // Check views
        assert!(temp.path().join("resources/views/auth/login.blade.html").exists());

        // Check controllers
        assert!(temp.path().join("src/controllers/auth/login.rs").exists());

        // Check routes
        assert!(temp.path().join("src/routes/auth.rs").exists());

        // Check middleware
        assert!(temp.path().join("src/middleware/auth.rs").exists());
    }

    #[tokio::test]
    async fn test_install_with_password_reset() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions {
            with_password_reset: true,
            ..Default::default()
        };

        installer.install_all(&options).await.unwrap();

        assert!(temp.path().join("resources/views/auth/forgot-password.blade.html").exists());
        assert!(temp.path().join("resources/views/auth/reset-password.blade.html").exists());
        assert!(temp.path().join("src/controllers/auth/password_reset.rs").exists());
    }

    #[tokio::test]
    async fn test_install_with_email_verification() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions {
            with_email_verification: true,
            ..Default::default()
        };

        installer.install_all(&options).await.unwrap();

        assert!(temp.path().join("resources/views/auth/verify-email.blade.html").exists());
        assert!(temp.path().join("src/controllers/auth/email_verification.rs").exists());
        assert!(temp.path().join("src/middleware/verified.rs").exists());
    }

    #[tokio::test]
    async fn test_install_with_api() {
        let temp = TempDir::new().unwrap();
        let installer = BreezeInstaller::new(temp.path()).unwrap();
        let options = InstallOptions {
            with_api: true,
            ..Default::default()
        };

        installer.install_all(&options).await.unwrap();

        assert!(temp.path().join("src/routes/api.rs").exists());
    }
}
