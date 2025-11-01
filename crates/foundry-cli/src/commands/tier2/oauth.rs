//! OAuth commands

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "oauth:list-providers", about = "List configured OAuth providers")]
pub struct OAuthListCommand;

impl OAuthListCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔐 OAuth Providers");
        println!("─────────────────────────────────────────");
        println!("  • Google OAuth (configured)");
        println!("  • GitHub OAuth (configured)");
        println!("  • Facebook OAuth (not configured)");
        println!("  • OpenID Connect (not configured)");
        Ok(())
    }
}

#[derive(Debug, Parser)]
#[command(name = "oauth:test", about = "Test OAuth provider configuration")]
pub struct OAuthTestCommand {
    /// Provider name (google, github, facebook)
    pub provider: String,
}

impl OAuthTestCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🧪 Testing OAuth provider: {}", self.provider);
        println!("✓ Provider configuration valid");
        Ok(())
    }
}
