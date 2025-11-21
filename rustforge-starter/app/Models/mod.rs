/// Database Models
///
/// Models represent your database tables and provide an elegant API
/// for querying and manipulating data.
///
/// Example:
/// ```rust
/// use rf_orm::{Model, HasMany};
///
/// #[derive(Model)]
/// #[table_name = "users"]
/// pub struct User {
///     pub id: i64,
///     pub name: String,
///     pub email: String,
///     pub created_at: DateTime<Utc>,
///     pub updated_at: DateTime<Utc>,
/// }
///
/// impl User {
///     pub fn posts(&self) -> HasMany<Post> {
///         self.has_many()
///     }
/// }
/// ```

// Add your models here
