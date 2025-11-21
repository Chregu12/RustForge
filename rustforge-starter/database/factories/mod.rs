/// Model Factories
///
/// Factories allow you to generate fake data for testing and seeding.
///
/// Example:
/// ```rust
/// use rf_testing::Factory;
/// use fake::{Faker, Fake};
///
/// pub struct UserFactory;
///
/// impl Factory for UserFactory {
///     type Model = User;
///
///     fn definition(&self) -> Self::Model {
///         User {
///             id: 0,
///             name: Faker.fake(),
///             email: Faker.fake(),
///             password: hash_password("password"),
///             created_at: Utc::now(),
///             updated_at: Utc::now(),
///         }
///     }
/// }
/// ```

// Add your factories here
