use foundry_service_container::{Container, Result};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct Database {
    connection_string: String,
}

#[derive(Debug)]
struct UserRepository {
    db: Arc<Database>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new container
    let container = Container::new();

    // Bind a singleton database connection
    container
        .singleton("database", || {
            Ok(Database {
                connection_string: "postgres://localhost/mydb".to_string(),
            })
        })
        .await?;

    // Bind a repository that depends on the database
    let container_clone = container.clone();
    container
        .bind("user_repository", move || {
            let db: Arc<Database> = tokio::runtime::Handle::current()
                .block_on(container_clone.resolve("database"))?;
            Ok(UserRepository { db })
        })
        .await?;

    // Resolve services
    let db1: Arc<Database> = container.resolve("database").await?;
    let db2: Arc<Database> = container.resolve("database").await?;

    // Singletons share the same instance
    println!("Database instances are the same: {}", Arc::ptr_eq(&db1, &db2));
    println!("Connection string: {}", db1.connection_string);

    let repo: Arc<UserRepository> = container.resolve("user_repository").await?;
    println!("Repository database: {}", repo.db.connection_string);

    Ok(())
}
