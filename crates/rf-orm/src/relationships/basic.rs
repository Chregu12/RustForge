use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, LoaderTrait, ModelTrait, Related};

/// Simplified Relationship helpers for Laravel-like syntax
///
/// Note: SeaORM already has a powerful relationship system.
/// These helpers provide Laravel-style convenience methods.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::RelationshipHelpers;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef {
/// #             Entity::belongs_to(super::user::Entity)
/// #                 .from(Column::UserId).to(super::user::Column::Id).into()
/// #         }
/// #     }
/// #     impl Related<super::user::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::User.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// # use sea_orm::EntityTrait;
/// let post = post::Entity::find_by_id(1).one(&db).await?.unwrap();
/// // Use Laravel-style helpers
/// let user = post.load_belongs_to::<user::Entity>(&db).await?;
/// # Ok(())
/// # }
/// ```
/// Helper trait for loading relationships
#[async_trait]
pub trait RelationshipHelpers: ModelTrait + Sized {
    /// Load a BelongsTo relationship
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::RelationshipHelpers;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef {
    /// #             Entity::belongs_to(super::user::Entity)
    /// #                 .from(Column::UserId).to(super::user::Column::Id).into()
    /// #         }
    /// #     }
    /// #     impl Related<super::user::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::User.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// # use sea_orm::EntityTrait;
    /// let post = post::Entity::find_by_id(1).one(&db).await?.unwrap();
    /// let author = post.load_belongs_to::<user::Entity>(&db).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_belongs_to<R>(&self, db: &DatabaseConnection) -> Result<Option<R::Model>, DbErr>
    where
        R: EntityTrait,
        Self::Entity: Related<R>,
    {
        self.find_related(R::default()).one(db).await
    }

    /// Load a HasMany relationship
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::RelationshipHelpers;
    /// # fn main() {}
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef {
    /// #             Entity::belongs_to(super::user::Entity)
    /// #                 .from(Column::UserId).to(super::user::Column::Id).into()
    /// #         }
    /// #     }
    /// #     impl Related<super::user::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::User.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { Post }
    /// #     impl RelationTrait for Relation {
    /// #         fn def(&self) -> RelationDef {
    /// #             Entity::has_many(super::post::Entity).into()
    /// #         }
    /// #     }
    /// #     impl Related<super::post::Entity> for Entity {
    /// #         fn to() -> RelationDef { Relation::Post.def() }
    /// #     }
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// # use sea_orm::EntityTrait;
    /// let user = user::Entity::find_by_id(1).one(&db).await?.unwrap();
    /// let posts = user.load_has_many::<post::Entity>(&db).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_has_many<R>(&self, db: &DatabaseConnection) -> Result<Vec<R::Model>, DbErr>
    where
        R: EntityTrait,
        Self::Entity: Related<R>,
    {
        self.find_related(R::default()).all(db).await
    }
}

// Blanket implementation for all ModelTrait types
impl<T> RelationshipHelpers for T where T: ModelTrait {}

/// Eager loading helper: batch-load a `has_many` / `belongs_to` relation for a
/// whole collection of parent models in a SINGLE query (no N+1), returning each
/// parent paired with its loaded children.
///
/// This is the ergonomic "attach the relation" surface: the returned
/// `Vec<(parent, Vec<child>)>` lets a caller consume `parent.1` as the populated
/// relation (e.g. `user.posts`) directly. Rust cannot write a preloaded value
/// into an arbitrary user struct field without codegen, so the loaded children
/// are handed back alongside each parent instead of mutated into a bare field
/// (a `#[derive]` for bare-field population is a future extension).
///
/// Internally this uses SeaORM's [`LoaderTrait::load_many`], which issues one
/// `SELECT ... WHERE parent_id IN (...)` and returns the children aligned by
/// index with the parents — so the pairing is exact and requires no per-parent
/// query.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::eager_load;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub name: String }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter)] pub enum Relation { User }
/// #     impl RelationTrait for Relation {
/// #         fn def(&self) -> RelationDef {
/// #             Entity::belongs_to(super::user::Entity)
/// #                 .from(Column::UserId).to(super::user::Column::Id).into()
/// #         }
/// #     }
/// #     impl Related<super::user::Entity> for Entity {
/// #         fn to() -> RelationDef { Relation::User.def() }
/// #     }
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// # use sea_orm::EntityTrait;
/// let posts = post::Entity::find().all(&db).await?;
/// let with_authors = eager_load::<post::Entity, user::Entity>(posts, &db).await?;
/// # Ok(())
/// # }
/// ```
pub async fn eager_load<E, R>(
    models: Vec<E::Model>,
    db: &DatabaseConnection,
) -> Result<Vec<(E::Model, Vec<R::Model>)>, DbErr>
where
    E: EntityTrait,
    R: EntityTrait,
    E: Related<R>,
    E::Model: ModelTrait + Sync,
    R::Model: Send + Sync,
{
    if models.is_empty() {
        return Ok(Vec::new());
    }

    // Single batched query for ALL parents (no N+1): `load_many` collects every
    // parent key, issues one `WHERE parent_key IN (...)` query, and returns the
    // children aligned by index with `models`.
    let related: Vec<Vec<R::Model>> = models.load_many(R::default(), db).await?;

    // Pair each parent with its (already grouped) children.
    Ok(models.into_iter().zip(related).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationships() {
        // Trait definitions compile
    }
}
