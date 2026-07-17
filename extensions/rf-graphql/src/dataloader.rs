//! DataLoader implementation for N+1 query prevention
//!
//! Provides efficient batch loading of related data.

use std::collections::HashMap;
use std::hash::Hash;

pub use async_graphql::dataloader::{DataLoader, HashMapCache, NoCache};

/// Trait for batch loading entities by ID
///
/// Implement this trait to create custom loaders that can be used with DataLoader.
/// Your loader should implement async_graphql::dataloader::Loader directly.
#[async_trait::async_trait]
pub trait BatchLoader<K, V>
where
    K: Send + Sync + Hash + Eq + Clone + 'static,
    V: Send + Sync + Clone + 'static,
{
    type Error: std::error::Error + Send + Sync + 'static;

    /// Batch load entities by their keys
    async fn batch_load(&self, keys: &[K]) -> Result<HashMap<K, V>, Self::Error>;
}

// Note: Implement async_graphql::dataloader::Loader directly on your structs
// to use with DataLoader. Example in tests below.

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::dataloader::Loader;
    use std::sync::Arc;

    #[derive(Debug, Clone, PartialEq)]
    struct User {
        id: i64,
        name: String,
    }

    struct UserLoader;

    impl Loader<i64> for UserLoader {
        type Value = User;
        type Error = Arc<std::io::Error>;

        fn load(
            &self,
            keys: &[i64],
        ) -> impl std::future::Future<Output = Result<HashMap<i64, Self::Value>, Self::Error>> + Send
        {
            let keys = keys.to_vec();
            async move {
                // Simulate database batch query
                let users = keys
                    .iter()
                    .map(|&id| {
                        (
                            id,
                            User {
                                id,
                                name: format!("User {}", id),
                            },
                        )
                    })
                    .collect();

                Ok(users)
            }
        }
    }

    #[tokio::test]
    async fn test_dataloader_batch_load() {
        let loader = DataLoader::new(UserLoader, tokio::spawn);

        let user1 = loader.load_one(1).await.unwrap();
        let user2 = loader.load_one(2).await.unwrap();

        assert_eq!(user1.unwrap().name, "User 1");
        assert_eq!(user2.unwrap().name, "User 2");
    }

    #[tokio::test]
    async fn test_dataloader_prevents_n_plus_1() {
        let loader = DataLoader::new(UserLoader, tokio::spawn);

        // Load multiple users - should be batched
        let futures = vec![loader.load_one(1), loader.load_one(2), loader.load_one(3)];

        let results = futures::future::join_all(futures).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
