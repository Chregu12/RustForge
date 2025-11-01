use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// GraphQL context that provides access to the database connection
/// and other shared resources
#[derive(Clone)]
pub struct GraphQLContext {
    pub db: Arc<DatabaseConnection>,
}

impl GraphQLContext {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db: Arc::new(db) }
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }
}
