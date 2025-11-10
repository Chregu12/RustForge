//! Test factories for generating test data
//!
//! Factories provide a convenient way to generate test data with the builder pattern.

use async_trait::async_trait;
use fake::{Fake, Faker};
use std::collections::HashMap;

/// Factory trait for creating test data
#[async_trait]
pub trait Factory: Sized {
    /// The type this factory creates
    type Output;

    /// Create a new factory instance
    fn new() -> Self;

    /// Build the output (async for database operations)
    async fn build(self) -> Self::Output;

    /// Create and save to database (if applicable)
    async fn create(self) -> Self::Output {
        self.build().await
    }
}

/// Factory builder with common test data
pub struct FactoryBuilder<T> {
    attributes: HashMap<String, serde_json::Value>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> FactoryBuilder<T> {
    /// Create a new factory builder
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set an attribute
    pub fn with<K: Into<String>, V: serde::Serialize>(mut self, key: K, value: V) -> Self {
        self.attributes
            .insert(key.into(), serde_json::to_value(value).unwrap());
        self
    }

    /// Get an attribute
    pub fn get<V: serde::de::DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.attributes
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get all attributes
    pub fn attributes(&self) -> &HashMap<String, serde_json::Value> {
        &self.attributes
    }
}

impl<T> Default for FactoryBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Fake data generators
pub struct FakeData;

impl FakeData {
    /// Generate a fake email
    pub fn email() -> String {
        use fake::faker::internet::en::SafeEmail;
        SafeEmail().fake()
    }

    /// Generate a fake name
    pub fn name() -> String {
        use fake::faker::name::en::Name;
        Name().fake()
    }

    /// Generate a fake username
    pub fn username() -> String {
        use fake::faker::internet::en::Username;
        Username().fake()
    }

    /// Generate a fake password
    pub fn password() -> String {
        use fake::faker::internet::en::Password;
        Password(8..16).fake()
    }

    /// Generate a fake phone number
    pub fn phone() -> String {
        use fake::faker::phone_number::en::PhoneNumber;
        PhoneNumber().fake()
    }

    /// Generate a fake address
    pub fn address() -> String {
        use fake::faker::address::en::StreetAddress;
        StreetAddress().fake()
    }

    /// Generate a fake company name
    pub fn company() -> String {
        use fake::faker::company::en::CompanyName;
        CompanyName().fake()
    }

    /// Generate a random string
    pub fn string(len: usize) -> String {
        use rand::Rng;
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(len)
            .map(char::from)
            .collect()
    }

    /// Generate a random number
    pub fn number(min: i32, max: i32) -> i32 {
        use rand::Rng;
        rand::thread_rng().gen_range(min..=max)
    }

    /// Generate a random boolean
    pub fn boolean() -> bool {
        use rand::Rng;
        rand::thread_rng().gen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fake_data_email() {
        let email = FakeData::email();
        assert!(email.contains('@'));
    }

    #[test]
    fn test_fake_data_name() {
        let name = FakeData::name();
        assert!(!name.is_empty());
    }

    #[test]
    fn test_fake_data_username() {
        let username = FakeData::username();
        assert!(!username.is_empty());
    }

    #[test]
    fn test_fake_data_string() {
        let s = FakeData::string(10);
        assert_eq!(s.len(), 10);
    }

    #[test]
    fn test_fake_data_number() {
        let n = FakeData::number(1, 100);
        assert!(n >= 1 && n <= 100);
    }

    #[test]
    fn test_fake_data_boolean() {
        let _ = FakeData::boolean();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_factory_builder() {
        let builder = FactoryBuilder::<String>::new()
            .with("email", "test@example.com")
            .with("name", "Test User");

        let email: String = builder.get("email").unwrap();
        assert_eq!(email, "test@example.com");

        let name: String = builder.get("name").unwrap();
        assert_eq!(name, "Test User");
    }

    #[test]
    fn test_factory_builder_attributes() {
        let builder = FactoryBuilder::<String>::new().with("key", "value");

        let attrs = builder.attributes();
        assert_eq!(attrs.len(), 1);
        assert!(attrs.contains_key("key"));
    }
}
