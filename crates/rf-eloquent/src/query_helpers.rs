//! # Query Helper Functions for Eloquent Relationships
//!
//! This module provides standalone query builder functions that execute REAL database queries
//! for loading relationships. Unlike the stub implementations in `HasRelationships` trait,
//! these functions actually query the database using SeaORM.
//!
//! ## Design Philosophy
//!
//! These functions are designed as a practical MVP approach:
//! - They execute real SeaORM queries
//! - They return actual data from the database
//! - They provide a simple, functional API
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use rf_eloquent::query_helpers::*;
//! use sea_orm::*;
//!
//! # async fn example(db: &DatabaseConnection, user: &User) -> Result<(), DbErr> {
//! # struct User { id: i32 }
//! # mod post {
//! #     pub use sea_orm::entity::prelude::*;
//! #     pub struct Entity;
//! #     pub struct Model { pub id: i32, pub user_id: i32, pub title: String }
//! #     pub enum Column { UserId }
//! # }
//! // Load has-many relationship: User -> Posts
//! let posts = has_many::<post::Entity, post::Model, _>(
//!     db,
//!     user.id,
//!     post::Column::UserId
//! ).await?;
//!
//! // Load belongs-to relationship: Post -> User
//! # mod user {
//! #     pub use sea_orm::entity::prelude::*;
//! #     pub struct Entity;
//! #     pub struct Model { pub id: i32 }
//! #     pub enum Column { Id }
//! # }
//! # let post_user_id = 1;
//! let author = belongs_to::<user::Entity, user::Model, _>(
//!     db,
//!     post_user_id,
//!     user::Column::Id
//! ).await?;
//! # Ok(())
//! # }
//! ```

use sea_orm::*;

/// Load has-one relationship
///
/// Executes a database query to load a single related model where the foreign key
/// matches the parent ID.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `foreign_key` - The column in the related table that references the parent
///
/// # Returns
///
/// An Option containing the related model if found, None otherwise.
///
/// # Example
///
/// ```rust,ignore
/// # use rf_eloquent::query_helpers::*;
/// # use sea_orm::*;
/// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
/// # mod user { pub struct Model { pub id: i32 } }
/// # mod profile {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     impl EntityName for Entity { fn table_name(&self) -> &str { "profiles" } }
/// #     pub struct Model { pub id: i32, pub user_id: i32, pub bio: String }
/// #     pub enum Column { UserId }
/// #     impl ColumnTrait for Column {
/// #         type EntityName = Entity;
/// #         fn def(&self) -> ColumnDef { todo!() }
/// #     }
/// # }
/// let user = user::Model { id: 1 };
/// let profile = has_one::<profile::Entity, profile::Model, _>(
///     db,
///     user.id,
///     profile::Column::UserId
/// ).await?;
/// assert!(profile.is_some()); // User has one profile
/// # Ok(())
/// # }
/// ```
pub async fn has_one<E, M, K>(
    db: &DatabaseConnection,
    parent_id: K,
    foreign_key: E::Column,
) -> Result<Option<M>, DbErr>
where
    E: EntityTrait,
    M: FromQueryResult + Sized + Send,
    K: Into<Value> + Clone,
    <E as EntityTrait>::Column: ColumnTrait,
{
    E::find()
        .filter(foreign_key.eq(parent_id))
        .into_model::<M>()
        .one(db)
        .await
}

/// Load has-many relationship
///
/// Executes a database query to load all related models where the foreign key
/// matches the parent ID.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `foreign_key` - The column in the related table that references the parent
///
/// # Returns
///
/// A vector of related models. Returns an empty vector if no related models exist.
///
/// # Example
///
/// ```rust,ignore
/// # use rf_eloquent::query_helpers::*;
/// # use sea_orm::*;
/// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
/// # mod user { pub struct Model { pub id: i32 } }
/// # mod post {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     impl EntityName for Entity { fn table_name(&self) -> &str { "posts" } }
/// #     pub struct Model { pub id: i32, pub user_id: i32 }
/// #     pub enum Column { UserId }
/// #     impl ColumnTrait for Column {
/// #         type EntityName = Entity;
/// #         fn def(&self) -> ColumnDef { todo!() }
/// #     }
/// # }
/// let user = user::Model { id: 1 };
/// let posts = has_many::<post::Entity, post::Model, _>(
///     db,
///     user.id,
///     post::Column::UserId
/// ).await?;
/// assert_eq!(posts.len(), 3); // User has 3 posts
/// # Ok(())
/// # }
/// ```
pub async fn has_many<E, M, K>(
    db: &DatabaseConnection,
    parent_id: K,
    foreign_key: E::Column,
) -> Result<Vec<M>, DbErr>
where
    E: EntityTrait,
    M: FromQueryResult + Sized + Send,
    K: Into<Value> + Clone,
    <E as EntityTrait>::Column: ColumnTrait,
{
    E::find()
        .filter(foreign_key.eq(parent_id))
        .into_model::<M>()
        .all(db)
        .await
}

/// Load belongs-to relationship
///
/// Executes a database query to load the parent model by its primary key.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `foreign_key_value` - The foreign key value from the child model
/// * `primary_key` - The primary key column of the parent table
///
/// # Returns
///
/// An Option containing the parent model if found, None otherwise.
///
/// # Example
///
/// ```rust,ignore
/// # use rf_eloquent::query_helpers::*;
/// # use sea_orm::*;
/// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
/// # mod user {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     pub struct Model { pub id: i32, pub name: String }
/// #     pub enum Column { Id }
/// # }
/// # mod post { pub struct Model { pub user_id: i32 } }
/// let post = post::Model { user_id: 42 };
/// let author = belongs_to::<user::Entity, user::Model, _>(
///     db,
///     post.user_id,
///     user::Column::Id
/// ).await?;
/// assert!(author.is_some()); // Post has an author
/// # Ok(())
/// # }
/// ```
pub async fn belongs_to<E, M, K>(
    db: &DatabaseConnection,
    foreign_key_value: K,
    primary_key: E::Column,
) -> Result<Option<M>, DbErr>
where
    E: EntityTrait,
    M: FromQueryResult + Sized + Send,
    K: Into<Value> + Clone,
    <E as EntityTrait>::Column: ColumnTrait,
{
    E::find()
        .filter(primary_key.eq(foreign_key_value))
        .into_model::<M>()
        .one(db)
        .await
}

/// Load belongs-to-many relationship (with pivot table)
///
/// Executes a two-step query to load related models through a pivot table:
/// 1. Query the pivot table to find related IDs
/// 2. Query the related table to load the actual models
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `pivot_entity` - The pivot table entity type
/// * `foreign_pivot_key` - Column in pivot table that references the parent
/// * `related_pivot_key` - Column in pivot table that references the related model
/// * `related_primary_key` - Primary key column of the related table
///
/// # Returns
///
/// A vector of related models. Returns an empty vector if no related models exist.
///
/// # Example
///
/// ```rust,ignore
/// # use rf_eloquent::query_helpers::*;
/// # use sea_orm::*;
/// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
/// # mod user { pub struct Model { pub id: i32 } }
/// # mod role {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     pub struct Model { pub id: i32, pub name: String }
/// #     pub enum Column { Id }
/// # }
/// # mod user_role {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     pub struct Model { pub user_id: i32, pub role_id: i32 }
/// #     pub enum Column { UserId, RoleId }
/// # }
/// let user = user::Model { id: 1 };
/// let roles = belongs_to_many::<role::Entity, user_role::Entity, role::Model, _>(
///     db,
///     user.id,
///     user_role::Column::UserId,
///     user_role::Column::RoleId,
///     role::Column::Id
/// ).await?;
/// assert_eq!(roles.len(), 2); // User has 2 roles
/// # Ok(())
/// # }
/// ```
pub async fn belongs_to_many<RE, PE, M, K>(
    db: &DatabaseConnection,
    parent_id: K,
    foreign_pivot_key: PE::Column,
    related_pivot_key: PE::Column,
    related_primary_key: RE::Column,
) -> Result<Vec<M>, DbErr>
where
    RE: EntityTrait, // Related Entity
    PE: EntityTrait, // Pivot Entity
    M: FromQueryResult + Sized + Send,
    K: Into<Value> + Clone,
    <RE as EntityTrait>::Column: ColumnTrait,
    <PE as EntityTrait>::Column: ColumnTrait,
    PE::Model: ModelTrait,
{
    use sea_orm::sea_query::Query;

    // Step 1: Query pivot table to get related IDs using a subquery approach
    // We use the IN subquery pattern to load related models through the pivot table
    // This is equivalent to:
    // SELECT * FROM related WHERE id IN (SELECT related_id FROM pivot WHERE parent_id = ?)

    // Execute the main query with IN (subquery)
    let related_models = RE::find()
        .filter(
            related_primary_key.in_subquery(
                Query::select()
                    .expr(sea_orm::sea_query::Expr::col((
                        PE::default(),
                        related_pivot_key,
                    )))
                    .from(PE::default())
                    .and_where(
                        sea_orm::sea_query::Expr::col((PE::default(), foreign_pivot_key))
                            .eq(parent_id.clone()),
                    )
                    .to_owned(),
            ),
        )
        .into_model::<M>()
        .all(db)
        .await?;

    Ok(related_models)
}

/// Attach a relationship in a pivot table (many-to-many)
///
/// Inserts a row into the pivot table to create a relationship between two models.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `related_id` - The ID of the related model
/// * `foreign_pivot_key` - Column in pivot table that references the parent
/// * `related_pivot_key` - Column in pivot table that references the related model
///
/// # Returns
///
/// Ok(()) if the relationship was attached successfully
pub async fn attach<PE, PK, RK>(
    db: &DatabaseConnection,
    parent_id: PK,
    related_id: RK,
    foreign_pivot_key: PE::Column,
    related_pivot_key: PE::Column,
) -> Result<(), DbErr>
where
    PE: EntityTrait,
    PK: Into<Value> + Clone,
    RK: Into<Value> + Clone,
    <PE as EntityTrait>::Column: ColumnTrait,
{
    use sea_orm::sea_query::{Expr, Query};

    // Build INSERT INTO pivot_table (foreign_key, related_key) VALUES (?, ?)
    let insert_stmt = Query::insert()
        .into_table(PE::default())
        .columns([foreign_pivot_key, related_pivot_key])
        .values_panic(vec![
            Expr::value(parent_id.into()),
            Expr::value(related_id.into()),
        ])
        .to_owned();

    db.execute(db.get_database_backend().build(&insert_stmt))
        .await?;

    Ok(())
}

/// Detach a relationship from a pivot table (many-to-many)
///
/// Deletes a row from the pivot table to remove a relationship between two models.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `related_id` - The ID of the related model (if None, detaches all)
/// * `foreign_pivot_key` - Column in pivot table that references the parent
/// * `related_pivot_key` - Column in pivot table that references the related model
///
/// # Returns
///
/// Ok(()) if the relationship was detached successfully
pub async fn detach<PE, PK, RK>(
    db: &DatabaseConnection,
    parent_id: PK,
    related_id: Option<RK>,
    foreign_pivot_key: PE::Column,
    related_pivot_key: PE::Column,
) -> Result<(), DbErr>
where
    PE: EntityTrait,
    PK: Into<Value> + Clone,
    RK: Into<Value> + Clone,
    <PE as EntityTrait>::Column: ColumnTrait,
{
    // If related_id is provided, delete specific relationship
    // Otherwise, delete all relationships for this parent
    if let Some(rid) = related_id {
        PE::delete_many()
            .filter(foreign_pivot_key.eq(parent_id.clone()))
            .filter(related_pivot_key.eq(rid))
            .exec(db)
            .await?;
    } else {
        PE::delete_many()
            .filter(foreign_pivot_key.eq(parent_id.clone()))
            .exec(db)
            .await?;
    }

    Ok(())
}

/// Sync relationships in a pivot table (many-to-many)
///
/// Replaces all existing relationships for a parent with a new set of relationships.
/// This is done by:
/// 1. Detaching all existing relationships
/// 2. Attaching all new relationships
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `related_ids` - The IDs of the related models to sync
/// * `foreign_pivot_key` - Column in pivot table that references the parent
/// * `related_pivot_key` - Column in pivot table that references the related model
///
/// # Returns
///
/// Ok(()) if the sync was successful
pub async fn sync<PE, PK, RK>(
    db: &DatabaseConnection,
    parent_id: PK,
    related_ids: Vec<RK>,
    foreign_pivot_key: PE::Column,
    related_pivot_key: PE::Column,
) -> Result<(), DbErr>
where
    PE: EntityTrait,
    PK: Into<Value> + Clone,
    RK: Into<Value> + Clone,
    <PE as EntityTrait>::Column: ColumnTrait,
{
    // Step 1: Detach all existing relationships
    detach::<PE, _, RK>(
        db,
        parent_id.clone(),
        None,
        foreign_pivot_key,
        related_pivot_key,
    )
    .await?;

    // Step 2: Attach all new relationships
    for related_id in related_ids {
        attach::<PE, _, _>(
            db,
            parent_id.clone(),
            related_id,
            foreign_pivot_key,
            related_pivot_key,
        )
        .await?;
    }

    Ok(())
}

/// Load has-many-through relationship
///
/// Executes queries to load related models through an intermediate table.
/// For example: Country -> Users -> Posts (Country has many Posts through Users)
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `through_foreign_key` - Column in through table that references the parent
/// * `final_foreign_key` - Column in final table that references the through model
/// * `through_primary_key` - Primary key of the through table
///
/// # Returns
///
/// A vector of related models. Returns an empty vector if no related models exist.
///
/// # Example
///
/// ```rust,ignore
/// # use rf_eloquent::query_helpers::*;
/// # use sea_orm::*;
/// # async fn example(db: &DatabaseConnection) -> Result<(), DbErr> {
/// # mod country { pub struct Model { pub id: i32 } }
/// # mod user {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     pub struct Model { pub id: i32, pub country_id: i32 }
/// #     pub enum Column { CountryId, Id }
/// # }
/// # mod post {
/// #     pub use sea_orm::entity::prelude::*;
/// #     pub struct Entity;
/// #     pub struct Model { pub id: i32, pub user_id: i32 }
/// #     pub enum Column { UserId }
/// # }
/// let country = country::Model { id: 1 };
/// let posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
///     db,
///     country.id,
///     user::Column::CountryId,
///     post::Column::UserId,
///     user::Column::Id
/// ).await?;
/// assert!(posts.len() > 0); // Country has posts through users
/// # Ok(())
/// # }
/// ```
pub async fn has_many_through<FE, TE, M, K>(
    db: &DatabaseConnection,
    parent_id: K,
    through_foreign_key: TE::Column,
    final_foreign_key: FE::Column,
    through_primary_key: TE::Column,
) -> Result<Vec<M>, DbErr>
where
    FE: EntityTrait, // Final Entity (e.g., Post)
    TE: EntityTrait, // Through Entity (e.g., User)
    M: FromQueryResult + Sized + Send,
    K: Into<Value> + Clone,
    <FE as EntityTrait>::Column: ColumnTrait,
    <TE as EntityTrait>::Column: ColumnTrait,
{
    use sea_orm::sea_query::Query;

    // Implementation using subquery approach (same pattern as belongs_to_many):
    // SELECT * FROM final_table
    // WHERE final_foreign_key IN (
    //     SELECT through_primary_key FROM through_table
    //     WHERE through_foreign_key = parent_id
    // )

    // Query final table with subquery filtering
    let final_models = FE::find()
        .filter(
            final_foreign_key.in_subquery(
                Query::select()
                    .expr(sea_orm::sea_query::Expr::col((
                        TE::default(),
                        through_primary_key,
                    )))
                    .from(TE::default())
                    .and_where(
                        sea_orm::sea_query::Expr::col((TE::default(), through_foreign_key))
                            .eq(parent_id.clone()),
                    )
                    .to_owned(),
            ),
        )
        .into_model::<M>()
        .all(db)
        .await?;

    Ok(final_models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_helpers_module_exists() {
        // This test ensures the module compiles
        assert!(true);
    }
}
