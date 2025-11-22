pub mod context;
pub mod error;
pub mod resolvers;
pub mod schema;
pub mod server;
pub mod types;

pub use context::GraphQLContext;
pub use schema::{build_schema, RustForgeSchema};
pub use server::graphql_handler;

use async_graphql::{EmptySubscription, MergedObject, Schema};

pub type GraphQLSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    resolvers::product::ProductQuery,
    resolvers::account::AccountQuery,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    resolvers::product::ProductMutation,
    resolvers::account::AccountMutation,
);
