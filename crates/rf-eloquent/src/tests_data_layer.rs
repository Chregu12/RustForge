//! Data-layer unit tests for rf-eloquent.
//!
//! Covers:
//! - Casting: JsonCast, DateTimeCast, BooleanCast, IntegerCast, FloatCast
//! - cast_value / uncast_value round-trip
//! - CastRegistry building & lookup
//! - Accessor helpers (common_accessors)
//! - Mutator helpers (common_mutators)
//! - AttributeBag CRUD
//! - AttributeValue conversions
//! - Observer: creating, created, updating, updated, deleting, deleted fired
//! - Observer: default no-ops, multiple observers, clear
//! - Global scope registry: add / remove / apply
//! - ScopeBuilder: conditional application
//! - ModelEvent helpers (is_before / is_after / name)
//! - Soft-deletes helpers

// ============================================================
// Casting
// ============================================================
#[cfg(test)]
mod casting_tests {
    use crate::casting::{
        cast_value, uncast_value, CastRegistry, CastType, CastedValue,
    };

    // --- cast_value ---

    #[test]
    fn test_json_cast_parses_object() {
        let v = cast_value(r#"{"key":"value","num":42}"#, CastType::Json).unwrap();
        match v {
            CastedValue::Json(j) => {
                assert_eq!(j["key"], "value");
                assert_eq!(j["num"], 42);
            }
            other => panic!("Expected Json, got {:?}", other),
        }
    }

    #[test]
    fn test_json_cast_parses_array() {
        let v = cast_value(r#"[1,2,3]"#, CastType::Json).unwrap();
        match v {
            CastedValue::Json(j) => assert!(j.is_array()),
            other => panic!("Expected Json, got {:?}", other),
        }
    }

    #[test]
    fn test_json_cast_fails_on_invalid() {
        let r = cast_value("not json", CastType::Json);
        assert!(r.is_err());
    }

    #[test]
    fn test_datetime_cast_rfc3339() {
        let v = cast_value("2024-01-15T10:30:00Z", CastType::DateTime).unwrap();
        match v {
            CastedValue::DateTime(dt) => {
                assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-15");
            }
            other => panic!("Expected DateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_datetime_cast_naive_format() {
        let v = cast_value("2024-06-01 12:00:00", CastType::DateTime).unwrap();
        match v {
            CastedValue::DateTime(dt) => {
                assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-06-01");
            }
            other => panic!("Expected DateTime, got {:?}", other),
        }
    }

    #[test]
    fn test_datetime_cast_fails_on_garbage() {
        let r = cast_value("not-a-date", CastType::DateTime);
        assert!(r.is_err());
    }

    #[test]
    fn test_boolean_cast_truthy_values() {
        for s in &["true", "1", "yes", "on", "TRUE"] {
            let v = cast_value(s, CastType::Boolean).unwrap();
            assert_eq!(v.as_bool().unwrap(), true, "value='{}' should be true", s);
        }
    }

    #[test]
    fn test_boolean_cast_falsy_values() {
        for s in &["false", "0", "no", "off", "FALSE"] {
            let v = cast_value(s, CastType::Boolean).unwrap();
            assert_eq!(v.as_bool().unwrap(), false, "value='{}' should be false", s);
        }
    }

    #[test]
    fn test_boolean_cast_fails_on_invalid() {
        let r = cast_value("maybe", CastType::Boolean);
        assert!(r.is_err());
    }

    #[test]
    fn test_integer_cast() {
        let v = cast_value("42", CastType::Integer).unwrap();
        assert_eq!(v.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_integer_cast_negative() {
        let v = cast_value("-7", CastType::Integer).unwrap();
        assert_eq!(v.as_i64().unwrap(), -7);
    }

    #[test]
    fn test_float_cast() {
        let v = cast_value("3.14", CastType::Float).unwrap();
        assert!((v.as_f64().unwrap() - 3.14).abs() < 1e-9);
    }

    #[test]
    fn test_string_cast_passthrough() {
        let v = cast_value("hello world", CastType::String).unwrap();
        assert_eq!(v.as_string().unwrap(), "hello world");
    }

    // --- uncast_value round-trips ---

    #[test]
    fn test_uncast_integer_round_trip() {
        let v = cast_value("99", CastType::Integer).unwrap();
        let s = uncast_value(v, CastType::Integer).unwrap();
        assert_eq!(s, "99");
    }

    #[test]
    fn test_uncast_boolean_round_trip() {
        let v = cast_value("true", CastType::Boolean).unwrap();
        let s = uncast_value(v, CastType::Boolean).unwrap();
        assert_eq!(s, "true");
    }

    #[test]
    fn test_uncast_json_round_trip() {
        let json_str = r#"{"k":"v"}"#;
        let v = cast_value(json_str, CastType::Json).unwrap();
        let s = uncast_value(v, CastType::Json).unwrap();
        // JSON serialization may differ in spacing; check key is present
        assert!(s.contains("\"k\""));
        assert!(s.contains("\"v\""));
    }

    #[test]
    fn test_uncast_datetime_round_trip() {
        let v = cast_value("2024-03-01T00:00:00Z", CastType::DateTime).unwrap();
        let s = uncast_value(v, CastType::DateTime).unwrap();
        // Result should be a valid RFC3339 string containing the date
        assert!(s.contains("2024-03-01"));
    }

    // --- CastRegistry ---

    #[test]
    fn test_cast_registry_build_and_lookup() {
        let reg = CastRegistry::new()
            .cast("name", CastType::String)
            .cast("age", CastType::Integer)
            .cast("metadata", CastType::Json);

        assert!(reg.has("name"));
        assert!(reg.has("age"));
        assert!(reg.has("metadata"));
        assert!(!reg.has("missing"));

        assert_eq!(*reg.get("name").unwrap(), CastType::String);
        assert_eq!(*reg.get("age").unwrap(), CastType::Integer);
        assert_eq!(*reg.get("metadata").unwrap(), CastType::Json);
    }

    #[test]
    fn test_cast_registry_remove() {
        let mut reg = CastRegistry::new().cast("temp", CastType::String);
        assert!(reg.has("temp"));
        reg.remove("temp");
        assert!(!reg.has("temp"));
    }

    #[test]
    fn test_cast_registry_empty_by_default() {
        let reg = CastRegistry::new();
        assert!(reg.all().is_empty());
    }
}

// ============================================================
// Accessors & Mutators
// ============================================================
#[cfg(test)]
mod accessor_tests {
    use crate::accessors::{
        common_accessors, common_mutators, AttributeBag, AttributeValue,
    };

    // --- AttributeValue conversions ---

    #[test]
    fn test_attribute_value_string_conversion() {
        let v = AttributeValue::String("hello".to_string());
        assert_eq!(v.as_string().unwrap(), "hello");
    }

    #[test]
    fn test_attribute_value_integer_conversion() {
        let v = AttributeValue::Integer(42);
        assert_eq!(v.as_integer().unwrap(), 42);
        assert_eq!(v.as_f64().unwrap(), 42.0);
    }

    #[test]
    fn test_attribute_value_boolean_conversion() {
        let v = AttributeValue::Boolean(true);
        assert_eq!(v.as_boolean().unwrap(), true);
    }

    #[test]
    fn test_attribute_value_null_is_null() {
        let v = AttributeValue::Null;
        assert!(v.is_null());
    }

    #[test]
    fn test_attribute_value_from_str() {
        let v: AttributeValue = "world".into();
        assert_eq!(v.as_string().unwrap(), "world");
    }

    #[test]
    fn test_attribute_value_from_i64() {
        let v: AttributeValue = 100_i64.into();
        assert_eq!(v.as_integer().unwrap(), 100);
    }

    #[test]
    fn test_attribute_value_from_bool() {
        let v: AttributeValue = false.into();
        assert_eq!(v.as_boolean().unwrap(), false);
    }

    #[test]
    fn test_attribute_value_as_bool_from_integer() {
        let v = AttributeValue::Integer(1);
        assert_eq!(v.as_bool().unwrap(), true);
        let v2 = AttributeValue::Integer(0);
        assert_eq!(v2.as_bool().unwrap(), false);
    }

    // --- AttributeBag ---

    #[test]
    fn test_attribute_bag_crud() {
        let mut bag = AttributeBag::new();

        bag.set("name", AttributeValue::String("Alice".to_string()));
        bag.set("age", AttributeValue::Integer(30));

        assert!(bag.has("name"));
        assert!(bag.has("age"));
        assert_eq!(bag.len(), 2);
        assert!(!bag.is_empty());

        assert_eq!(bag.get("name").unwrap().as_string().unwrap(), "Alice");
        assert_eq!(bag.get("age").unwrap().as_integer().unwrap(), 30);

        bag.remove("name");
        assert!(!bag.has("name"));
        assert_eq!(bag.len(), 1);

        bag.clear();
        assert!(bag.is_empty());
    }

    #[test]
    fn test_attribute_bag_keys() {
        let mut bag = AttributeBag::new();
        bag.set("x", AttributeValue::Integer(1));
        bag.set("y", AttributeValue::Integer(2));
        let keys = bag.keys();
        assert_eq!(keys.len(), 2);
    }

    // --- common_accessors ---

    #[test]
    fn test_accessor_uppercase() {
        assert_eq!(common_accessors::uppercase("hello"), "HELLO");
    }

    #[test]
    fn test_accessor_lowercase() {
        assert_eq!(common_accessors::lowercase("WORLD"), "world");
    }

    #[test]
    fn test_accessor_title_case() {
        assert_eq!(common_accessors::title_case("hello world"), "Hello World");
        assert_eq!(common_accessors::title_case("rust is great"), "Rust Is Great");
    }

    #[test]
    fn test_accessor_truncate_shorter_than_limit() {
        assert_eq!(common_accessors::truncate("hi", 10), "hi");
    }

    #[test]
    fn test_accessor_truncate_longer_than_limit() {
        assert_eq!(common_accessors::truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_accessor_strip_html() {
        assert_eq!(common_accessors::strip_html("<p>Hello</p>"), "Hello");
        assert_eq!(common_accessors::strip_html("<b>bold</b> text"), "bold text");
    }

    // --- common_mutators ---

    #[test]
    fn test_mutator_trim() {
        assert_eq!(common_mutators::trim("  hello  ".to_string()), "hello");
    }

    #[test]
    fn test_mutator_slugify() {
        assert_eq!(common_mutators::slugify("Hello World!"), "hello-world");
        assert_eq!(common_mutators::slugify("  multiple   spaces  "), "multiple-spaces");
    }

    #[test]
    fn test_mutator_encrypt_decrypt_roundtrip() {
        let plaintext = "secret message";
        let encrypted = common_mutators::encrypt(plaintext);
        assert_ne!(encrypted, plaintext);
        let decrypted = common_mutators::decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}

// ============================================================
// Observer
// ============================================================
#[cfg(test)]
mod observer_tests {
    use crate::observer::{dispatch_observers, observe, ObserverRegistry, Observer};
    use crate::events::ModelEvent;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct Post {
        title: String,
    }

    struct PostObserver {
        log: Arc<Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl Observer<Post> for PostObserver {
        async fn creating(&self, m: &Post) -> crate::events::EventResult {
            self.log.lock().unwrap().push(format!("creating:{}", m.title));
            Ok(())
        }
        async fn created(&self, m: &Post) -> crate::events::EventResult {
            self.log.lock().unwrap().push(format!("created:{}", m.title));
            Ok(())
        }
        async fn updating(&self, m: &Post) -> crate::events::EventResult {
            self.log.lock().unwrap().push(format!("updating:{}", m.title));
            Ok(())
        }
        async fn updated(&self, m: &Post) -> crate::events::EventResult {
            self.log.lock().unwrap().push(format!("updated:{}", m.title));
            Ok(())
        }
        async fn deleting(&self, m: &Post) -> crate::events::EventResult {
            self.log.lock().unwrap().push(format!("deleting:{}", m.title));
            Ok(())
        }
        async fn deleted(&self, m: &Post) -> crate::events::EventResult {
            self.log.lock().unwrap().push(format!("deleted:{}", m.title));
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_observer_creating_event() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log.clone() });

        let p = Post { title: "my post".to_string() };
        registry.dispatch(ModelEvent::Creating, &p).await.unwrap();
        assert_eq!(log.lock().unwrap().as_slice(), &["creating:my post"]);
    }

    #[tokio::test]
    async fn test_observer_created_event() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log.clone() });

        let p = Post { title: "new".to_string() };
        registry.dispatch(ModelEvent::Created, &p).await.unwrap();
        assert_eq!(log.lock().unwrap().as_slice(), &["created:new"]);
    }

    #[tokio::test]
    async fn test_observer_updating_event() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log.clone() });

        let p = Post { title: "edited".to_string() };
        registry.dispatch(ModelEvent::Updating, &p).await.unwrap();
        assert_eq!(log.lock().unwrap().as_slice(), &["updating:edited"]);
    }

    #[tokio::test]
    async fn test_observer_updated_event() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log.clone() });

        let p = Post { title: "saved".to_string() };
        registry.dispatch(ModelEvent::Updated, &p).await.unwrap();
        assert_eq!(log.lock().unwrap().as_slice(), &["updated:saved"]);
    }

    #[tokio::test]
    async fn test_observer_deleting_and_deleted_events() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log.clone() });

        let p = Post { title: "gone".to_string() };
        registry.dispatch(ModelEvent::Deleting, &p).await.unwrap();
        registry.dispatch(ModelEvent::Deleted, &p).await.unwrap();

        let entries = log.lock().unwrap().clone();
        assert_eq!(entries, vec!["deleting:gone", "deleted:gone"]);
    }

    #[tokio::test]
    async fn test_observer_default_no_op_methods_dont_panic() {
        struct NoopObserver;
        #[async_trait]
        impl Observer<Post> for NoopObserver {}

        let registry = ObserverRegistry::new();
        registry.register::<Post, _>(NoopObserver);

        let p = Post { title: "x".to_string() };
        // All events should succeed silently via default no-op implementations
        for event in [
            ModelEvent::Creating, ModelEvent::Created,
            ModelEvent::Updating, ModelEvent::Updated,
            ModelEvent::Saving, ModelEvent::Saved,
            ModelEvent::Deleting, ModelEvent::Deleted,
            ModelEvent::Restoring, ModelEvent::Restored,
        ] {
            registry.dispatch(event, &p).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_multiple_observers_all_called() {
        let registry = ObserverRegistry::new();
        let log1 = Arc::new(Mutex::new(Vec::new()));
        let log2 = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log1.clone() });
        registry.register::<Post, _>(PostObserver { log: log2.clone() });

        let p = Post { title: "shared".to_string() };
        registry.dispatch(ModelEvent::Created, &p).await.unwrap();

        assert_eq!(log1.lock().unwrap().len(), 1);
        assert_eq!(log2.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_observer_clear_stops_dispatch() {
        let registry = ObserverRegistry::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        registry.register::<Post, _>(PostObserver { log: log.clone() });

        registry.clear::<Post>();

        let p = Post { title: "cleared".to_string() };
        registry.dispatch(ModelEvent::Created, &p).await.unwrap();
        assert!(log.lock().unwrap().is_empty(), "no events after clear");
    }
}

// ============================================================
// ModelEvent helpers
// ============================================================
#[cfg(test)]
mod model_event_tests {
    use crate::events::ModelEvent;

    #[test]
    fn test_before_events() {
        for e in [
            ModelEvent::Creating,
            ModelEvent::Updating,
            ModelEvent::Saving,
            ModelEvent::Deleting,
            ModelEvent::Restoring,
        ] {
            assert!(e.is_before(), "{:?} should be 'before'", e);
            assert!(!e.is_after(), "{:?} should not be 'after'", e);
        }
    }

    #[test]
    fn test_after_events() {
        for e in [
            ModelEvent::Created,
            ModelEvent::Updated,
            ModelEvent::Saved,
            ModelEvent::Deleted,
            ModelEvent::Restored,
        ] {
            assert!(e.is_after(), "{:?} should be 'after'", e);
            assert!(!e.is_before(), "{:?} should not be 'before'", e);
        }
    }

    #[test]
    fn test_event_name_strings() {
        assert_eq!(ModelEvent::Creating.name(), "creating");
        assert_eq!(ModelEvent::Created.name(), "created");
        assert_eq!(ModelEvent::Updating.name(), "updating");
        assert_eq!(ModelEvent::Updated.name(), "updated");
        assert_eq!(ModelEvent::Saving.name(), "saving");
        assert_eq!(ModelEvent::Saved.name(), "saved");
        assert_eq!(ModelEvent::Deleting.name(), "deleting");
        assert_eq!(ModelEvent::Deleted.name(), "deleted");
        assert_eq!(ModelEvent::Restoring.name(), "restoring");
        assert_eq!(ModelEvent::Restored.name(), "restored");
    }
}

// ============================================================
// Global scopes
// ============================================================
#[cfg(test)]
mod scope_tests {
    use crate::scopes::{
        GlobalScopeRegistry, ScopeBuilder,
    };
    use sea_orm::entity::prelude::*;
    use sea_orm::QueryOrder;

    // Minimal in-memory entity for testing scope infrastructure

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "scope_test_posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub published: bool,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    // --- GlobalScopeRegistry (instance-level, no global state) ---

    #[test]
    fn test_global_scope_registry_register_and_count() {
        let mut reg: GlobalScopeRegistry<Entity> = GlobalScopeRegistry::new();
        assert_eq!(reg.count(), 0);

        reg.register("published", |q| q.filter(Column::Published.eq(true)));
        assert_eq!(reg.count(), 1);
        assert!(reg.has("published"));
    }

    #[test]
    fn test_global_scope_registry_remove() {
        let mut reg: GlobalScopeRegistry<Entity> = GlobalScopeRegistry::new();
        reg.register("x", |q| q);
        assert!(reg.has("x"));
        let removed = reg.remove("x");
        assert!(removed);
        assert!(!reg.has("x"));
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_global_scope_registry_clear() {
        let mut reg: GlobalScopeRegistry<Entity> = GlobalScopeRegistry::new();
        reg.register("a", |q| q);
        reg.register("b", |q| q);
        assert_eq!(reg.count(), 2);
        reg.clear();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn test_global_scope_registry_apply_all() {
        let mut reg: GlobalScopeRegistry<Entity> = GlobalScopeRegistry::new();
        // Just verify apply_all doesn't panic
        reg.register("published", |q| q.filter(Column::Published.eq(true)));
        let _select = reg.apply_all(Entity::find());
    }

    // --- ScopeBuilder ---

    #[test]
    fn test_scope_builder_tracks_applied_scopes() {
        let builder = ScopeBuilder::<Entity>::new()
            .scope("published", |q| q.filter(Column::Published.eq(true)))
            .scope("ordered", |q| q.order_by_asc(Column::Id));

        let applied = builder.get_applied_scopes();
        assert!(applied.contains(&"published".to_string()));
        assert!(applied.contains(&"ordered".to_string()));
        assert_eq!(applied.len(), 2);
    }

    #[test]
    fn test_scope_builder_when_true_applies() {
        let builder = ScopeBuilder::<Entity>::new()
            .when(true, "filtered", |q| q.filter(Column::Published.eq(true)));

        assert!(builder.get_applied_scopes().contains(&"filtered".to_string()));
    }

    #[test]
    fn test_scope_builder_when_false_skips() {
        let builder = ScopeBuilder::<Entity>::new()
            .when(false, "filtered", |q| q.filter(Column::Published.eq(true)));

        assert!(!builder.get_applied_scopes().contains(&"filtered".to_string()));
    }

    #[test]
    fn test_scope_builder_unless_false_applies() {
        let builder = ScopeBuilder::<Entity>::new()
            .unless(false, "visible", |q| q.filter(Column::Published.eq(true)));

        assert!(builder.get_applied_scopes().contains(&"visible".to_string()));
    }
}
