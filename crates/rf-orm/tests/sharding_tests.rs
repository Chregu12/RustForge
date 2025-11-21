//! Comprehensive tests for database sharding

use rf_orm::sharding::manager::{ShardManager, ShardStrategy};
use rf_orm::sharding::strategies::{GeographicStrategy, HashStrategy, RangeStrategy, TenantStrategy};
use sea_orm::{Database, DatabaseConnection};
use std::collections::HashMap;
use std::sync::Arc;

async fn setup_test_db() -> DatabaseConnection {
    Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to test database")
}

async fn setup_multiple_dbs(count: usize) -> Vec<DatabaseConnection> {
    let mut dbs = Vec::new();
    for _ in 0..count {
        dbs.push(setup_test_db().await);
    }
    dbs
}

#[tokio::test]
async fn test_hash_strategy_shard_selection() {
    let strategy = HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
        "shard3".to_string(),
    ]);

    // Same key should always return same shard
    let shard1 = strategy.get_shard("user_123").await.unwrap();
    let shard2 = strategy.get_shard("user_123").await.unwrap();
    assert_eq!(shard1, shard2);

    // Different keys may return different shards
    let shard_a = strategy.get_shard("user_456").await.unwrap();
    let shard_b = strategy.get_shard("user_789").await.unwrap();

    // Both should be valid shard names
    assert!(["shard1", "shard2", "shard3"].contains(&shard_a.as_str()));
    assert!(["shard1", "shard2", "shard3"].contains(&shard_b.as_str()));
}

#[tokio::test]
async fn test_hash_strategy_consistency() {
    let strategy = HashStrategy::new(vec!["shard1".to_string(), "shard2".to_string()]);

    // Test consistency across multiple calls
    let mut results = Vec::new();
    for _ in 0..100 {
        results.push(strategy.get_shard("consistent_key").await.unwrap());
    }

    // All results should be the same
    assert!(results.iter().all(|x| x == &results[0]));
}

#[tokio::test]
async fn test_range_strategy_shard_selection() {
    let strategy = RangeStrategy::new(vec![
        (1, 1000, "shard1".to_string()),
        (1001, 2000, "shard2".to_string()),
        (2001, 3000, "shard3".to_string()),
    ]);

    assert_eq!(strategy.get_shard("500").await.unwrap(), "shard1");
    assert_eq!(strategy.get_shard("1500").await.unwrap(), "shard2");
    assert_eq!(strategy.get_shard("2500").await.unwrap(), "shard3");
}

#[tokio::test]
async fn test_range_strategy_boundaries() {
    let strategy = RangeStrategy::new(vec![
        (1, 1000, "shard1".to_string()),
        (1001, 2000, "shard2".to_string()),
    ]);

    // Test exact boundaries
    assert_eq!(strategy.get_shard("1").await.unwrap(), "shard1");
    assert_eq!(strategy.get_shard("1000").await.unwrap(), "shard1");
    assert_eq!(strategy.get_shard("1001").await.unwrap(), "shard2");
    assert_eq!(strategy.get_shard("2000").await.unwrap(), "shard2");
}

#[tokio::test]
async fn test_range_strategy_out_of_range() {
    let strategy = RangeStrategy::new(vec![(1, 1000, "shard1".to_string())]);

    let result = strategy.get_shard("5000").await;
    assert!(result.is_err());

    let result = strategy.get_shard("0").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tenant_strategy_shard_selection() {
    let mut map = HashMap::new();
    map.insert("tenant_a".to_string(), "shard_premium".to_string());
    map.insert("tenant_b".to_string(), "shard_standard".to_string());
    map.insert("tenant_c".to_string(), "shard_premium".to_string());

    let strategy = TenantStrategy::new(map);

    assert_eq!(
        strategy.get_shard("tenant_a").await.unwrap(),
        "shard_premium"
    );
    assert_eq!(
        strategy.get_shard("tenant_b").await.unwrap(),
        "shard_standard"
    );
    assert_eq!(
        strategy.get_shard("tenant_c").await.unwrap(),
        "shard_premium"
    );
}

#[tokio::test]
async fn test_tenant_strategy_with_default() {
    let mut map = HashMap::new();
    map.insert("tenant_a".to_string(), "shard1".to_string());

    let strategy = TenantStrategy::with_default(map, "default_shard".to_string());

    assert_eq!(strategy.get_shard("tenant_a").await.unwrap(), "shard1");
    assert_eq!(
        strategy.get_shard("unknown_tenant").await.unwrap(),
        "default_shard"
    );
}

#[tokio::test]
async fn test_tenant_isolation() {
    let mut map = HashMap::new();
    map.insert("tenant_1".to_string(), "shard1".to_string());
    map.insert("tenant_2".to_string(), "shard2".to_string());

    let strategy = TenantStrategy::new(map);

    let shard1 = strategy.get_shard("tenant_1").await.unwrap();
    let shard2 = strategy.get_shard("tenant_2").await.unwrap();

    // Different tenants should be on different shards
    assert_ne!(shard1, shard2);
}

#[tokio::test]
async fn test_shard_manager_creation() {
    let strategy = Arc::new(HashStrategy::new(vec!["shard1".to_string()]));
    let manager = ShardManager::new(strategy);

    assert_eq!(manager.shard_count(), 0);
}

#[tokio::test]
async fn test_shard_manager_add_shard() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs = setup_multiple_dbs(2).await;

    manager.add_shard("shard1".to_string(), Arc::new(dbs[0].clone()));
    manager.add_shard("shard2".to_string(), Arc::new(dbs[1].clone()));

    assert_eq!(manager.shard_count(), 2);
    assert!(manager.has_shard("shard1"));
    assert!(manager.has_shard("shard2"));
}

#[tokio::test]
async fn test_shard_manager_connection_for() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs = setup_multiple_dbs(2).await;

    manager.add_shard("shard1".to_string(), Arc::new(dbs[0].clone()));
    manager.add_shard("shard2".to_string(), Arc::new(dbs[1].clone()));

    // Should successfully get a connection
    let result = manager.connection_for("user_123").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shard_manager_get_all_shards() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs = setup_multiple_dbs(2).await;

    manager.add_shard("shard1".to_string(), Arc::new(dbs[0].clone()));
    manager.add_shard("shard2".to_string(), Arc::new(dbs[1].clone()));

    let all_shards = manager.all_shards();
    assert_eq!(all_shards.len(), 2);
}

#[tokio::test]
async fn test_shard_manager_execute_on_all() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs = setup_multiple_dbs(2).await;

    manager.add_shard("shard1".to_string(), Arc::new(dbs[0].clone()));
    manager.add_shard("shard2".to_string(), Arc::new(dbs[1].clone()));

    // Execute a simple query on all shards
    let results: Vec<i32> = manager
        .execute_on_all(|_db| {
            Box::pin(async move {
                // Simulate a count query
                Ok(10)
            })
        })
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], 10);
    assert_eq!(results[1], 10);
}

#[tokio::test]
async fn test_shard_manager_execute_on_specific_shards() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
        "shard3".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs: Vec<_> = setup_multiple_dbs(3).await.into_iter().map(Arc::new).collect();

    manager.add_shard("shard1".to_string(), Arc::clone(&dbs[0]));
    manager.add_shard("shard2".to_string(), Arc::clone(&dbs[1]));
    manager.add_shard("shard3".to_string(), Arc::clone(&dbs[2]));

    // Execute only on specific shards
    let results: Vec<i32> = manager
        .execute_on_shards(
            vec!["shard1".to_string(), "shard3".to_string()],
            |_db| {
                Box::pin(async move {
                    Ok(42)
                })
            },
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_performance_with_multiple_shards() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
        "shard3".to_string(),
        "shard4".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs: Vec<_> = setup_multiple_dbs(4).await.into_iter().map(Arc::new).collect();

    for (i, db) in dbs.iter().enumerate() {
        manager.add_shard(format!("shard{}", i + 1), Arc::clone(db));
    }

    // Test routing performance
    let start = std::time::Instant::now();
    for i in 0..1000 {
        let key = format!("user_{}", i);
        let _ = manager.connection_for(&key).await;
    }
    let duration = start.elapsed();

    // Should complete quickly (< 100ms for 1000 routing decisions)
    assert!(duration.as_millis() < 100);
}

#[tokio::test]
async fn test_geographic_strategy() {
    let mut map = HashMap::new();
    map.insert("US".to_string(), "shard_us_east".to_string());
    map.insert("EU".to_string(), "shard_eu_west".to_string());
    map.insert("APAC".to_string(), "shard_asia".to_string());

    let strategy = GeographicStrategy::new(map);

    assert_eq!(
        strategy.get_shard("US").await.unwrap(),
        "shard_us_east"
    );
    assert_eq!(
        strategy.get_shard("EU").await.unwrap(),
        "shard_eu_west"
    );
    assert_eq!(
        strategy.get_shard("APAC").await.unwrap(),
        "shard_asia"
    );
}

#[tokio::test]
async fn test_shard_manager_remove_shard() {
    let strategy = Arc::new(HashStrategy::new(vec!["shard1".to_string()]));
    let mut manager = ShardManager::new(strategy);

    let db = setup_test_db().await;
    manager.add_shard("shard1".to_string(), Arc::new(db));

    assert_eq!(manager.shard_count(), 1);

    let removed = manager.remove_shard("shard1");
    assert!(removed.is_some());
    assert_eq!(manager.shard_count(), 0);
}

#[tokio::test]
async fn test_shard_manager_get_shard() {
    let strategy = Arc::new(HashStrategy::new(vec!["shard1".to_string()]));
    let mut manager = ShardManager::new(strategy);

    let db = setup_test_db().await;
    manager.add_shard("shard1".to_string(), Arc::new(db));

    let result = manager.get_shard("shard1");
    assert!(result.is_ok());

    let result = manager.get_shard("nonexistent");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_shard_manager_shard_names() {
    let strategy = Arc::new(HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
    ]));
    let mut manager = ShardManager::new(strategy);

    let dbs = setup_multiple_dbs(2).await;

    manager.add_shard("shard1".to_string(), Arc::new(dbs[0].clone()));
    manager.add_shard("shard2".to_string(), Arc::new(dbs[1].clone()));

    let names = manager.shard_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"shard1".to_string()));
    assert!(names.contains(&"shard2".to_string()));
}

#[tokio::test]
async fn test_shard_manager_execute_with_key() {
    let strategy = Arc::new(HashStrategy::new(vec!["shard1".to_string()]));
    let mut manager = ShardManager::new(strategy);

    let db = setup_test_db().await;
    manager.add_shard("shard1".to_string(), Arc::new(db));

    let result: i32 = manager
        .execute_with_key("user_123", |_db| {
            Box::pin(async move { Ok(100) })
        })
        .await
        .unwrap();

    assert_eq!(result, 100);
}

#[tokio::test]
async fn test_range_strategy_add_range() {
    let mut strategy = RangeStrategy::new(vec![(1, 1000, "shard1".to_string())]);

    strategy.add_range(1001, 2000, "shard2".to_string());

    assert_eq!(strategy.get_shard("500").await.unwrap(), "shard1");
    assert_eq!(strategy.get_shard("1500").await.unwrap(), "shard2");
}

#[tokio::test]
async fn test_tenant_strategy_add_remove() {
    let mut strategy = TenantStrategy::new(HashMap::new());

    strategy.add_tenant("tenant_x".to_string(), "shard_x".to_string());
    assert_eq!(
        strategy.get_shard("tenant_x").await.unwrap(),
        "shard_x"
    );

    strategy.remove_tenant("tenant_x");
    assert!(strategy.get_shard("tenant_x").await.is_err());
}

#[tokio::test]
async fn test_tenant_strategy_set_default() {
    let mut strategy = TenantStrategy::new(HashMap::new());

    strategy.set_default_shard("default_shard".to_string());

    assert_eq!(
        strategy.get_shard("any_tenant").await.unwrap(),
        "default_shard"
    );
}

#[tokio::test]
async fn test_hash_distribution() {
    let strategy = HashStrategy::new(vec![
        "shard1".to_string(),
        "shard2".to_string(),
        "shard3".to_string(),
    ]);

    let mut shard_counts: HashMap<String, usize> = HashMap::new();

    // Test distribution across 1000 keys
    for i in 0..1000 {
        let key = format!("user_{}", i);
        let shard = strategy.get_shard(&key).await.unwrap();
        *shard_counts.entry(shard).or_insert(0) += 1;
    }

    // Each shard should have approximately 1/3 of the keys
    // Allow for some variance (between 250 and 450 per shard)
    for (_shard, count) in shard_counts.iter() {
        assert!(*count > 250 && *count < 450, "Uneven distribution: {}", count);
    }
}
