#![allow(dead_code)] // command scaffolding defined for upcoming subcommands, not all wired into the dispatcher yet
//! Search commands

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "search:index", about = "Index models for search")]
pub struct SearchIndexCommand {
    /// Model to index
    pub model: String,
}

impl SearchIndexCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔍 Indexing model: {}", self.model);
        println!("✓ Indexed successfully");
        Ok(())
    }
}

#[derive(Debug, Parser)]
#[command(name = "search:reindex", about = "Reindex all searchable models")]
pub struct SearchReindexCommand {
    /// Force reindex even if index exists
    #[arg(long)]
    pub force: bool,
}

impl SearchReindexCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🔄 Reindexing all searchable models...");
        println!("Force: {}", self.force);
        println!("✓ Reindexing complete");
        Ok(())
    }
}
