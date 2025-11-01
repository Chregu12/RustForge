use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Factory für Test-Daten-Generierung von `Product`
///
/// Diese Factory erstellt Test-Instanzen mit realistischen, aber deterministischen Daten.
/// Nutze `build()` für einzelne Instanzen oder `build_many(n)` für mehrere.
#[derive(Debug, Clone)]
pub struct ProductFactory {
    sequence: &'static AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
}

impl Default for ProductFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductFactory {
    /// Erstellt eine neue Factory-Instanz
    pub fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        Self {
            sequence: &SEQUENCE,
        }
    }

    /// Generiert eine einzelne Test-Instanz
    pub fn build(&self) -> Product {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        Product {
            id: seq as i64,
            name: self.fake_name(seq),
            description: self.fake_description(seq),
            created_at: self.fake_timestamp(seq),
        }
    }

    /// Generiert mehrere Test-Instanzen
    pub fn build_many(&self, count: usize) -> Vec<Product> {
        (0..count).map(|_| self.build()).collect()
    }

    // Faker-ähnliche Helper-Methoden

    fn fake_name(&self, seq: u64) -> String {
        let prefixes = ["Test", "Demo", "Sample", "Example", "Mock"];
        let suffixes = ["Item", "Entity", "Object", "Record", "Entry"];
        let prefix = prefixes[(seq as usize) % prefixes.len()];
        let suffix = suffixes[(seq as usize / prefixes.len()) % suffixes.len()];
        format!("{} {} #{}", prefix, suffix, seq)
    }

    fn fake_description(&self, seq: u64) -> String {
        let templates = [
            "A comprehensive description for testing purposes",
            "Sample data entry for integration tests",
            "Mock object with realistic attributes",
            "Generated test fixture for validation",
            "Automated test data with unique identifier",
        ];
        let template = templates[(seq as usize) % templates.len()];
        format!("{}.  ID: {}", template, seq)
    }

    fn fake_timestamp(&self, seq: u64) -> String {
        // Generiert deterministische Timestamps basierend auf Sequenz
        let base_year = 2024;
        let month = ((seq % 12) + 1).max(1).min(12);
        let day = ((seq % 28) + 1).max(1).min(28);
        let hour = (seq % 24).max(0).min(23);
        let minute = ((seq * 17) % 60).max(0).min(59);
        let second = ((seq * 37) % 60).max(0).min(59);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            base_year, month, day, hour, minute, second
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_builds_unique_instances() {
        let factory = ProductFactory::new();
        let item1 = factory.build();
        let item2 = factory.build();

        assert_ne!(item1.id, item2.id, "IDs should be unique");
        assert_ne!(item1.name, item2.name, "Names should be unique");
    }

    #[test]
    fn factory_builds_multiple_instances() {
        let factory = ProductFactory::new();
        let items = factory.build_many(5);

        assert_eq!(items.len(), 5);
        // Überprüfe dass alle IDs unique sind
        let ids: std::collections::HashSet<_> = items.iter().map(|i| i.id).collect();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn factory_generates_realistic_data() {
        let factory = ProductFactory::new();
        let item = factory.build();

        assert!(!item.name.is_empty());
        assert!(!item.description.is_empty());
        assert!(item.created_at.contains('T')); // ISO8601 format
    }
}
