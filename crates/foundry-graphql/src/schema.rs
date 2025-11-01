use crate::{GraphQLContext, GraphQLSchema, MutationRoot, QueryRoot};
use async_graphql::{EmptySubscription, Schema};
use sea_orm::DatabaseConnection;

pub type RustForgeSchema = GraphQLSchema;

/// Build the GraphQL schema with the provided database connection
pub fn build_schema(db: DatabaseConnection) -> RustForgeSchema {
    let context = GraphQLContext::new(db);

    Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription,
    )
    .data(context)
    .finish()
}
