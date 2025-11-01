//! Translation commands

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "make:translation", about = "Create a new translation file")]
pub struct MakeTranslationCommand {
    /// Translation namespace
    pub namespace: String,

    /// Locale code (e.g., en, de, fr)
    #[arg(long, short = 'l', default_value = "en")]
    pub locale: String,
}

impl MakeTranslationCommand {
    pub async fn execute(&self) -> anyhow::Result<()> {
        println!("🌐 Creating translation file: {}/{}", self.namespace, self.locale);

        let dir = format!("resources/lang/{}", self.locale);
        std::fs::create_dir_all(&dir)?;

        let content = serde_json::json!({
            "example": "This is an example translation",
            "welcome": "Welcome to RustForge",
        });

        let filename = format!("{}/{}.json", dir, self.namespace);
        std::fs::write(&filename, serde_json::to_string_pretty(&content)?)?;

        println!("✓ Translation file created: {}", filename);
        Ok(())
    }
}
