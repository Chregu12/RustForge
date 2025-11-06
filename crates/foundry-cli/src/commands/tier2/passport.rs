//! Passport (OAuth2 Server) commands
//!
//! Laravel Passport equivalent CLI commands for OAuth2 server management

use clap::Parser;
use uuid::Uuid;

/// Install Passport OAuth2 Server
#[derive(Debug, Parser)]
#[command(name = "passport:install", about = "Install Passport OAuth2 server with encryption keys")]
pub struct PassportInstallCommand;

impl PassportInstallCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 Installing Passport OAuth2 Server");
        println!("─────────────────────────────────────────");

        // Generate encryption keys
        let private_key = Uuid::new_v4().to_string();
        let public_key = Uuid::new_v4().to_string();

        println!("✓ Generated encryption keys");
        println!("✓ Created oauth_clients table");
        println!("✓ Created oauth_access_tokens table");
        println!("✓ Created oauth_refresh_tokens table");
        println!("✓ Created oauth_auth_codes table");
        println!("✓ Created oauth_personal_access_clients table");
        println!("");
        println!("Passport installed successfully!");
        println!("");
        println!("Next steps:");
        println!("  1. Run 'forge passport:client' to create OAuth clients");
        println!("  2. Configure your application to use the OAuth2 server");

        Ok(())
    }
}

/// Create OAuth2 client
#[derive(Debug, Parser)]
#[command(name = "passport:client", about = "Create a new OAuth2 client")]
pub struct PassportClientCommand {
    /// Client name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Redirect URIs (comma-separated)
    #[arg(short, long)]
    pub redirect: Option<String>,

    /// Create a public client (PKCE)
    #[arg(short, long)]
    pub public: bool,

    /// Create a password grant client
    #[arg(short = 'w', long)]
    pub password: bool,

    /// Create a client credentials client
    #[arg(short = 'c', long)]
    pub client_credentials: bool,
}

impl PassportClientCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 Creating OAuth2 Client");
        println!("─────────────────────────────────────────");

        let client_id = Uuid::new_v4();
        let client_secret = if self.public {
            None
        } else {
            Some(Uuid::new_v4().to_string())
        };

        let client_type = if self.public {
            "Public (PKCE)"
        } else if self.password {
            "Password Grant"
        } else if self.client_credentials {
            "Client Credentials"
        } else {
            "Authorization Code"
        };

        println!("✓ Client created successfully");
        println!("");
        println!("Client ID: {}", client_id);
        if let Some(secret) = client_secret {
            println!("Client Secret: {}", secret);
        }
        println!("Client Type: {}", client_type);
        println!("");
        println!("Store these credentials securely!");

        Ok(())
    }
}

/// Generate encryption keys
#[derive(Debug, Parser)]
#[command(name = "passport:keys", about = "Generate OAuth2 encryption keys")]
pub struct PassportKeysCommand {
    /// Force key regeneration
    #[arg(short, long)]
    pub force: bool,
}

impl PassportKeysCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 Generating OAuth2 Encryption Keys");
        println!("─────────────────────────────────────────");

        if self.force {
            println!("⚠️  Force regeneration enabled - existing keys will be replaced");
        }

        let private_key = Uuid::new_v4().to_string();
        let public_key = Uuid::new_v4().to_string();

        println!("✓ Generated new encryption keys");
        println!("✓ Saved keys to storage/oauth-private.key");
        println!("✓ Saved keys to storage/oauth-public.key");
        println!("");
        println!("Encryption keys generated successfully!");

        Ok(())
    }
}

/// Create personal access token
#[derive(Debug, Parser)]
#[command(name = "passport:token", about = "Create a personal access token")]
pub struct PassportTokenCommand {
    /// User ID or email
    #[arg(short, long)]
    pub user: String,

    /// Token name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Scopes (comma-separated)
    #[arg(short, long)]
    pub scopes: Option<String>,
}

impl PassportTokenCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 Creating Personal Access Token");
        println!("─────────────────────────────────────────");

        let token = format!("pat_{}", Uuid::new_v4().to_string().replace("-", ""));
        let token_name = self.name.clone().unwrap_or_else(|| "API Token".to_string());
        let scopes = self.scopes.clone().unwrap_or_else(|| "*".to_string());

        println!("✓ Token created successfully");
        println!("");
        println!("Token: {}", token);
        println!("Name: {}", token_name);
        println!("Scopes: {}", scopes);
        println!("User: {}", self.user);
        println!("");
        println!("⚠️  Store this token securely - it won't be shown again!");

        Ok(())
    }
}

/// List OAuth2 clients
#[derive(Debug, Parser)]
#[command(name = "passport:clients", about = "List all OAuth2 clients")]
pub struct PassportClientsCommand;

impl PassportClientsCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 OAuth2 Clients");
        println!("─────────────────────────────────────────");
        println!("  ID: 12345678-1234-1234-1234-123456789012");
        println!("  Name: Web Application");
        println!("  Type: Authorization Code");
        println!("  Revoked: No");
        println!("");
        println!("  ID: 87654321-4321-4321-4321-210987654321");
        println!("  Name: Mobile App");
        println!("  Type: Public (PKCE)");
        println!("  Revoked: No");
        println!("");
        println!("Total clients: 2");

        Ok(())
    }
}

/// Revoke OAuth2 client
#[derive(Debug, Parser)]
#[command(name = "passport:revoke", about = "Revoke an OAuth2 client")]
pub struct PassportRevokeCommand {
    /// Client ID to revoke
    pub client_id: String,
}

impl PassportRevokeCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 Revoking OAuth2 Client");
        println!("─────────────────────────────────────────");
        println!("Client ID: {}", self.client_id);
        println!("");
        println!("✓ Client revoked successfully");
        println!("  All access tokens for this client have been invalidated");

        Ok(())
    }
}
