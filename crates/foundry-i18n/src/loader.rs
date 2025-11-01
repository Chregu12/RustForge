//! Translation file loader

use std::collections::HashMap;
use std::path::PathBuf;
use crate::Result;

pub trait TranslationLoader {
    fn load(&self, locale: &str) -> Result<HashMap<String, String>>;
}

pub struct FileLoader {
    base_path: PathBuf,
}

impl FileLoader {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

impl TranslationLoader for FileLoader {
    fn load(&self, locale: &str) -> Result<HashMap<String, String>> {
        let path = self.base_path.join(format!("{}.json", locale));

        if let Ok(content) = std::fs::read_to_string(&path) {
            serde_json::from_str(&content)
                .map_err(|e| crate::I18nError::LoadError(e.to_string()))
        } else {
            Ok(HashMap::new())
        }
    }
}
