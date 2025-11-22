use async_trait::async_trait;
use chrono::Utc;
use rf_sanctum::{
    LoadFromToken, PersonalAccessToken, SanctumAuth, SanctumError, TokenRepository, Tokenable,
};
use sea_orm::{Database, DatabaseConnection, DbErr};

// Test user model
#[derive(Clone, Debug)]
struct TestUser {
    id: i64,
    name: String,
}

#[async_trait]
impl Tokenable for TestUser {
    fn tokenable_type() -> &'static str {
        "TestUser"
    }

    fn tokenable_id(&self) -> i64 {
        self.id
    }
}

#[async_trait]
impl LoadFromToken for TestUser {
    async fn load_from_token(
        tokenable_id: i64,
        _db: &DatabaseConnection,
    ) -> Result<Self, SanctumError> {
        Ok(TestUser {
            id: tokenable_id,
            name: format!("User {}", tokenable_id),
        })
    }
}

async fn setup_test_db() -> Result<DatabaseConnection, DbErr> {
    // Use in-memory SQLite for testing
    let db = Database::connect("sqlite::memory:").await?;

    // Run migrations
    let sql = include_str!("../migrations/create_personal_access_tokens.sql");
    // Convert PostgreSQL syntax to SQLite
    let sql = sql
        .replace("BIGSERIAL", "INTEGER")
        .replace("BIGINT", "INTEGER")
        .replace("VARCHAR(255)", "TEXT")
        .replace("VARCHAR(64)", "TEXT")
        .replace("TIMESTAMP WITH TIME ZONE", "TEXT")
        .replace("JSON", "TEXT")
        .replace("IF NOT EXISTS", "");

    for statement in sql.split(';').filter(|s| !s.trim().is_empty()) {
        sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Sqlite, statement.to_string())
            .execute(&db)
            .await?;
    }

    Ok(db)
}

#[tokio::test]
async fn test_token_generation() {
    let token = PersonalAccessToken::generate_token();
    assert_eq!(token.len(), 80);
    assert!(token.chars().all(|c| c.is_alphanumeric()));
}

#[tokio::test]
async fn test_token_hashing() {
    let token = "my-secret-token";
    let hash1 = PersonalAccessToken::hash_token(token);
    let hash2 = PersonalAccessToken::hash_token(token);

    assert_eq!(hash1, hash2); // Same input = same hash
    assert_eq!(hash1.len(), 64); // SHA256 = 64 hex chars
}

#[tokio::test]
async fn test_create_token() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    let new_token = user
        .create_token("test-app", vec!["read:posts", "write:posts"], None, &db)
        .await?;

    assert_eq!(new_token.access_token.len(), 80);
    assert_eq!(new_token.token.name, "test-app");
    assert_eq!(new_token.token.abilities.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_token_abilities() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    let new_token = user
        .create_token("test-app", vec!["read:posts", "write:posts"], None, &db)
        .await?;

    assert!(new_token.token.can("read:posts"));
    assert!(new_token.token.can("write:posts"));
    assert!(!new_token.token.can("delete:posts"));

    Ok(())
}

#[tokio::test]
async fn test_wildcard_ability() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    let new_token = user.create_token("admin-app", vec!["*"], None, &db).await?;

    assert!(new_token.token.can("anything"));
    assert!(new_token.token.can("read:posts"));
    assert!(new_token.token.can("delete:everything"));

    Ok(())
}

#[tokio::test]
async fn test_find_token_by_hash() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    let new_token = user
        .create_token("test-app", vec!["read:posts"], None, &db)
        .await?;

    // Hash the plaintext token
    let hashed = PersonalAccessToken::hash_token(&new_token.access_token);

    // Find token by hash
    let repo = TokenRepository::new(&db);
    let found = repo.find_by_token(&hashed).await?;

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.name, "test-app");

    Ok(())
}

#[tokio::test]
async fn test_revoke_token() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    let new_token = user
        .create_token("test-app", vec!["read:posts"], None, &db)
        .await?;

    // Revoke the token
    user.revoke_token(new_token.token.id, &db).await?;

    // Try to find it
    let hashed = PersonalAccessToken::hash_token(&new_token.access_token);
    let repo = TokenRepository::new(&db);
    let found = repo.find_by_token(&hashed).await?;

    assert!(found.is_none());

    Ok(())
}

#[tokio::test]
async fn test_token_expiration() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    // Create token that expires in the past
    let expired_time = Utc::now() - chrono::Duration::hours(1);
    let new_token = user
        .create_token("test-app", vec!["read:posts"], Some(expired_time), &db)
        .await?;

    assert!(new_token.token.is_expired());

    Ok(())
}

#[tokio::test]
async fn test_multiple_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    // Create multiple tokens
    let _token1 = user
        .create_token("app1", vec!["read:posts"], None, &db)
        .await?;
    let _token2 = user
        .create_token("app2", vec!["write:posts"], None, &db)
        .await?;
    let _token3 = user.create_token("app3", vec!["*"], None, &db).await?;

    // Get all tokens
    let tokens = user.tokens(&db).await?;
    assert_eq!(tokens.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_revoke_all_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let db = setup_test_db().await?;

    let user = TestUser {
        id: 1,
        name: "Test User".to_string(),
    };

    // Create multiple tokens
    let _token1 = user
        .create_token("app1", vec!["read:posts"], None, &db)
        .await?;
    let _token2 = user
        .create_token("app2", vec!["write:posts"], None, &db)
        .await?;

    // Revoke all
    user.revoke_all_tokens(&db).await?;

    // Check they're gone
    let tokens = user.tokens(&db).await?;
    assert_eq!(tokens.len(), 0);

    Ok(())
}
