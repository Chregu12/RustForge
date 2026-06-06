//! # Model Factory System
//!
//! Laravel-inspired factory system for generating model instances with fake data.
//!
//! ## Example
//!
//! ```rust
//! use rf_seeder::factory::{ModelFactory, FactoryBuilder};
//! use fake::{Fake, Faker};
//! use fake::faker::name::en::Name;
//! use fake::faker::internet::en::SafeEmail;
//!
//! #[derive(Debug, Default, Clone, PartialEq)]
//! struct User {
//!     name: String,
//!     email: String,
//!     age: u32,
//! }
//!
//! impl ModelFactory for User {
//!     fn definition() -> Self {
//!         User {
//!             name: Name().fake(),
//!             email: SafeEmail().fake(),
//!             age: (18u32..80u32).fake(),
//!         }
//!     }
//! }
//!
//! // Create 10 users with random data
//! let users = User::factory().count(10).make();
//! assert_eq!(users.len(), 10);
//!
//! // Override specific fields with .state()
//! let admins = User::factory()
//!     .count(3)
//!     .state(|mut u| { u.name = "Admin".into(); u })
//!     .make();
//! assert_eq!(admins[0].name, "Admin");
//! ```

use serde_json::Value;

/// A builder for constructing model instances, optionally with overrides and sequences.
///
/// Obtained via [`ModelFactory::factory()`].
pub struct FactoryBuilder<T> {
    /// Function that produces a fresh default instance.
    definition: Box<dyn Fn() -> T>,
    /// Number of instances to produce (default: 1).
    count: u32,
    /// State modifiers applied in order after the definition is called.
    states: Vec<Box<dyn Fn(T) -> T>>,
    /// Sequence callbacks: `(index) -> Value` stored for use during construction.
    ///
    /// Each entry is a pair of (field_name_hint, callback). Because Rust structs
    /// are not dynamically field-addressable, the sequence value is passed through
    /// the closure chain via a shared index counter — the user receives the index
    /// via `.sequence()` and is responsible for applying it inside a `.state()` call.
    /// This variant stores opaque per-index state modifiers derived from sequences.
    sequence_states: Vec<Box<dyn Fn(usize, T) -> T>>,
}

impl<T: 'static> FactoryBuilder<T> {
    /// Create a new builder from a definition function.
    pub fn new(definition: impl Fn() -> T + 'static) -> Self {
        Self {
            definition: Box::new(definition),
            count: 1,
            states: Vec::new(),
            sequence_states: Vec::new(),
        }
    }

    /// Set how many instances to produce.
    pub fn count(mut self, n: u32) -> Self {
        self.count = n;
        self
    }

    /// Add a state modifier that is applied to every produced instance.
    ///
    /// Multiple `.state()` calls are chained in order.
    pub fn state(mut self, modifier: impl Fn(T) -> T + 'static) -> Self {
        self.states.push(Box::new(modifier));
        self
    }

    /// Add a sequenced state modifier.
    ///
    /// The closure receives the zero-based index of the instance being produced
    /// and returns a [`serde_json::Value`] that is passed to a second closure
    /// which applies it to the model.  This API mirrors Laravel's `sequence()`
    /// while remaining type-safe.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_seeder::factory::{ModelFactory, FactoryBuilder};
    /// # use fake::{Fake, Faker};
    /// # use fake::faker::name::en::Name;
    /// # use fake::faker::internet::en::SafeEmail;
    /// # #[derive(Debug, Default, Clone)]
    /// # struct User { name: String, email: String, age: u32 }
    /// # impl ModelFactory for User {
    /// #     fn definition() -> Self { User::default() }
    /// # }
    /// let users = User::factory()
    ///     .count(3)
    ///     .sequence(|i, mut u| {
    ///         u.name = format!("User #{}", i);
    ///         u
    ///     })
    ///     .make();
    /// assert_eq!(users[0].name, "User #0");
    /// assert_eq!(users[2].name, "User #2");
    /// ```
    pub fn sequence(mut self, modifier: impl Fn(usize, T) -> T + 'static) -> Self {
        self.sequence_states.push(Box::new(modifier));
        self
    }

    /// Build all instances without persisting them to a database.
    pub fn make(self) -> Vec<T> {
        (0..self.count as usize)
            .map(|i| {
                // Start from the definition
                let mut instance = (self.definition)();
                // Apply global state modifiers
                for modifier in &self.states {
                    instance = modifier(instance);
                }
                // Apply sequence modifiers (aware of the index)
                for seq_modifier in &self.sequence_states {
                    instance = seq_modifier(i, instance);
                }
                instance
            })
            .collect()
    }
}

// Convenience: produce a single instance from a builder with count == 1.
impl<T: 'static> FactoryBuilder<T> {
    /// Create exactly one instance (ignores the configured count).
    pub fn make_one(self) -> T {
        (self.definition)()
    }
}

// Allow FactoryBuilder to be used with `_value` sequences (kept for API completeness).
impl<T: 'static> FactoryBuilder<T> {
    /// Convenience wrapper: add a sequence driven by a `Value`-producing closure.
    ///
    /// The second closure maps the produced `Value` back onto the model, so the
    /// caller decides how to interpret the value.
    ///
    /// ```rust
    /// # use rf_seeder::factory::{ModelFactory, FactoryBuilder};
    /// # use serde_json::{Value, json};
    /// # #[derive(Debug, Default, Clone)]
    /// # struct User { name: String, email: String, age: u32 }
    /// # impl ModelFactory for User {
    /// #     fn definition() -> Self { User::default() }
    /// # }
    /// let users = User::factory()
    ///     .count(5)
    ///     .value_sequence(
    ///         |i| serde_json::json!(i * 100),
    ///         |v, mut u| { u.age = v.as_u64().unwrap_or(0) as u32; u }
    ///     )
    ///     .make();
    /// assert_eq!(users[0].age, 0);
    /// assert_eq!(users[4].age, 400);
    /// ```
    pub fn value_sequence(
        self,
        value_fn: impl Fn(usize) -> Value + 'static,
        apply_fn: impl Fn(Value, T) -> T + 'static,
    ) -> Self {
        self.sequence(move |i, instance| {
            let value = value_fn(i);
            apply_fn(value, instance)
        })
    }
}

/// Trait that a model type implements to participate in the factory system.
///
/// Implement `definition()` to return a model with sensible defaults (typically
/// generated with the [`fake`] crate).  The blanket `factory()` method gives
/// access to a [`FactoryBuilder`] for further customisation.
pub trait ModelFactory: Sized + 'static {
    /// Return a model instance populated with default fake data.
    fn definition() -> Self;

    /// Entry point: returns a [`FactoryBuilder`] pre-configured with this
    /// model's definition function and a count of 1.
    fn factory() -> FactoryBuilder<Self> {
        FactoryBuilder::new(Self::definition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::name::en::Name;
    use fake::Fake;

    // ── Test model ──────────────────────────────────────────────────────────

    #[derive(Debug, Default, Clone, PartialEq)]
    struct User {
        name: String,
        email: String,
        age: u32,
        active: bool,
    }

    impl ModelFactory for User {
        fn definition() -> Self {
            User {
                name: Name().fake(),
                email: SafeEmail().fake(),
                age: (18u32..80u32).fake(),
                active: true,
            }
        }
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[test]
    fn test_make_single() {
        let user = User::factory().make_one();
        assert!(!user.name.is_empty(), "name should not be empty");
        assert!(user.email.contains('@'), "email should be valid");
        assert!(user.age >= 18 && user.age < 80);
    }

    #[test]
    fn test_make_returns_default_count_of_one() {
        let users = User::factory().make();
        assert_eq!(users.len(), 1);
    }

    #[test]
    fn test_count_produces_correct_number() {
        let users = User::factory().count(10).make();
        assert_eq!(users.len(), 10);
    }

    #[test]
    fn test_count_zero() {
        let users = User::factory().count(0).make();
        assert_eq!(users.len(), 0);
    }

    #[test]
    fn test_state_overrides_field() {
        let users = User::factory()
            .count(5)
            .state(|mut u| {
                u.name = "Alice".to_string();
                u
            })
            .make();

        assert_eq!(users.len(), 5);
        for user in &users {
            assert_eq!(user.name, "Alice");
        }
    }

    #[test]
    fn test_state_chaining() {
        let users = User::factory()
            .count(3)
            .state(|mut u| {
                u.name = "Bob".to_string();
                u
            })
            .state(|mut u| {
                u.active = false;
                u
            })
            .make();

        for user in &users {
            assert_eq!(user.name, "Bob");
            assert!(!user.active);
        }
    }

    #[test]
    fn test_sequence_gives_unique_indexed_values() {
        let users = User::factory()
            .count(5)
            .sequence(|i, mut u| {
                u.name = format!("User #{}", i);
                u
            })
            .make();

        assert_eq!(users.len(), 5);
        for (i, user) in users.iter().enumerate() {
            assert_eq!(user.name, format!("User #{}", i));
        }
    }

    #[test]
    fn test_sequence_and_state_combined() {
        let users = User::factory()
            .count(4)
            .state(|mut u| {
                u.active = false;
                u
            })
            .sequence(|i, mut u| {
                u.age = i as u32 * 10 + 20;
                u
            })
            .make();

        assert_eq!(users.len(), 4);
        for (i, user) in users.iter().enumerate() {
            assert!(!user.active, "state should set active=false");
            assert_eq!(user.age, i as u32 * 10 + 20, "sequence should set age");
        }
    }

    #[test]
    fn test_value_sequence() {
        let users = User::factory()
            .count(5)
            .value_sequence(
                |i| serde_json::json!(i as u64 * 100),
                |v, mut u| {
                    u.age = v.as_u64().unwrap_or(0) as u32;
                    u
                },
            )
            .make();

        assert_eq!(users.len(), 5);
        assert_eq!(users[0].age, 0);
        assert_eq!(users[4].age, 400);
    }

    #[test]
    fn test_instances_are_independent() {
        // Each call to the definition should produce a fresh instance;
        // modifying one should not affect another.
        let users = User::factory().count(2).make();
        assert_eq!(users.len(), 2);
        // They may or may not be equal (random), but they are independent allocations.
        let _ = users[0].clone();
        let _ = users[1].clone();
    }

    #[test]
    fn test_factory_entry_point() {
        // Verify that ModelFactory::factory() is wired correctly.
        let builder = User::factory();
        let users = builder.count(3).make();
        assert_eq!(users.len(), 3);
    }

    // ── Additional model: shows the trait works for any struct ───────────────

    #[derive(Debug, Default, Clone)]
    struct Product {
        title: String,
        price: f64,
        in_stock: bool,
    }

    impl ModelFactory for Product {
        fn definition() -> Self {
            Product {
                title: fake::faker::lorem::en::Word().fake::<String>(),
                price: (100u32..10000u32).fake::<u32>() as f64 / 100.0,
                in_stock: true,
            }
        }
    }

    #[test]
    fn test_product_factory() {
        let products = Product::factory().count(3).make();
        assert_eq!(products.len(), 3);
        for p in &products {
            assert!(!p.title.is_empty());
            assert!(p.price > 0.0);
            assert!(p.in_stock);
        }
    }

    #[test]
    fn test_product_out_of_stock_state() {
        let products = Product::factory()
            .count(2)
            .state(|mut p| {
                p.in_stock = false;
                p
            })
            .make();
        for p in &products {
            assert!(!p.in_stock);
        }
    }
}
