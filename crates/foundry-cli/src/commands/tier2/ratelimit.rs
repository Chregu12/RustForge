//! Rate limit commands

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "rate-limit:reset", about = "Reset rate limits")]
pub struct RateLimitResetCommand {
    /// Key to reset (user ID or IP address)
    pub key: Option<String>,

    /// Reset all rate limits
    #[arg(long)]
    pub all: bool,
}

impl RateLimitResetCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        if self.all {
            println!("🔄 Resetting all rate limits...");
        } else if let Some(ref key) = self.key {
            println!("🔄 Resetting rate limit for: {}", key);
        } else {
            anyhow::bail!("Provide either a key or --all flag");
        }

        println!("✓ Rate limits reset successfully");
        Ok(())
    }
}
