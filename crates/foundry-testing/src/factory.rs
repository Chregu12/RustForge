//! Model factories for test data generation

use async_trait::async_trait;
use chrono::Utc;
use fake::{Fake, Faker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use uuid::Uuid;

/// Trait for model factories
#[async_trait]
pub trait Factory<T>: Send + Sync
where
    T: Send + Sync,
{
    /// Create a single instance
    async fn create(&self) -> anyhow::Result<T>;

    /// Create multiple instances
    async fn create_many(&self, count: usize) -> anyhow::Result<Vec<T>> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            results.push(self.create().await?);
        }
        Ok(results)
    }

    /// Build without persisting (for testing validations)
    fn build(&self) -> anyhow::Result<T>;

    /// Build multiple instances
    fn build_many(&self, count: usize) -> anyhow::Result<Vec<T>> {
        let mut results = Vec::with_capacity(count);
        for _ in 0..count {
            results.push(self.build()?);
        }
        Ok(results)
    }
}

/// Factory builder with customization
pub struct FactoryBuilder<T> {
    overrides: HashMap<String, serde_json::Value>,
    relationships: Vec<Box<dyn Fn() -> anyhow::Result<serde_json::Value> + Send + Sync>>,
    _phantom: PhantomData<T>,
}

impl<T> FactoryBuilder<T> {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
            relationships: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with<V: Serialize>(mut self, field: impl Into<String>, value: V) -> Self {
        self.overrides.insert(
            field.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    pub fn with_relationship<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> anyhow::Result<serde_json::Value> + Send + Sync + 'static,
    {
        self.relationships.push(Box::new(factory));
        self
    }
}

impl<T> Default for FactoryBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Faker helpers for common data types
pub struct FactoryHelper;

impl FactoryHelper {
    pub fn uuid() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn name() -> String {
        use fake::faker::name::en::*;
        Name().fake()
    }

    pub fn email() -> String {
        use fake::faker::internet::en::*;
        SafeEmail().fake()
    }

    pub fn username() -> String {
        use fake::faker::internet::en::*;
        Username().fake()
    }

    pub fn password() -> String {
        use fake::faker::internet::en::*;
        Password(8..20).fake()
    }

    pub fn phone() -> String {
        use fake::faker::phone_number::en::*;
        PhoneNumber().fake()
    }

    pub fn street() -> String {
        use fake::faker::address::en::*;
        StreetName().fake()
    }

    pub fn city() -> String {
        use fake::faker::address::en::*;
        CityName().fake()
    }

    pub fn country() -> String {
        use fake::faker::address::en::*;
        CountryName().fake()
    }

    pub fn company() -> String {
        use fake::faker::company::en::*;
        CompanyName().fake()
    }

    pub fn sentence() -> String {
        use fake::faker::lorem::en::*;
        Sentence(3..10).fake()
    }

    pub fn paragraph() -> String {
        use fake::faker::lorem::en::*;
        Paragraph(3..5).fake()
    }

    pub fn number(min: i32, max: i32) -> i32 {
        rand::thread_rng().gen_range(min..=max)
    }

    pub fn boolean() -> bool {
        rand::thread_rng().gen_bool(0.5)
    }

    pub fn timestamp() -> String {
        Utc::now().to_rfc3339()
    }
}

/// Example user factory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub created_at: String,
}

pub struct UserFactory;

#[async_trait]
impl Factory<TestUser> for UserFactory {
    async fn create(&self) -> anyhow::Result<TestUser> {
        // In a real implementation, this would save to database
        self.build()
    }

    fn build(&self) -> anyhow::Result<TestUser> {
        Ok(TestUser {
            id: FactoryHelper::uuid(),
            name: FactoryHelper::name(),
            email: FactoryHelper::email(),
            password: FactoryHelper::password(),
            created_at: FactoryHelper::timestamp(),
        })
    }
}

/// Example post factory with relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPost {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub published: bool,
    pub created_at: String,
}

pub struct PostFactory {
    author_id: Option<String>,
}

impl PostFactory {
    pub fn new() -> Self {
        Self { author_id: None }
    }

    pub fn for_author(mut self, author_id: String) -> Self {
        self.author_id = Some(author_id);
        self
    }
}

impl Default for PostFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Factory<TestPost> for PostFactory {
    async fn create(&self) -> anyhow::Result<TestPost> {
        self.build()
    }

    fn build(&self) -> anyhow::Result<TestPost> {
        let author_id = self
            .author_id
            .clone()
            .unwrap_or_else(|| FactoryHelper::uuid());

        Ok(TestPost {
            id: FactoryHelper::uuid(),
            title: FactoryHelper::sentence(),
            content: FactoryHelper::paragraph(),
            author_id,
            published: FactoryHelper::boolean(),
            created_at: FactoryHelper::timestamp(),
        })
    }
}
