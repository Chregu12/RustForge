use foundry_plugins::ValidationRules;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StoreProduct {
    // TODO: Anfrage-Payload-Felder hier definieren
}

impl StoreProduct {
    pub fn rules() -> ValidationRules {
        ValidationRules {
            rules: serde_json::json!({
                "required": ["field_name"],
                "fields": {
                    "field_name": { "min_length": 3 }
                }
            }),
        }
    }
}
