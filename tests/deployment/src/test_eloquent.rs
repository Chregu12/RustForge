//! Deployment tests for rf-eloquent

#[cfg(test)]
mod tests {
    use rf_eloquent::{AttributeBag, AttributeValue, CastRegistry, CastType, EventDispatcher, ModelEvent};

    // ── AttributeBag ─────────────────────────────────────────────

    #[test]
    fn attribute_bag_set_get() {
        let mut bag = AttributeBag::new();
        bag.set("name", AttributeValue::String("John".into()));
        bag.set("age", AttributeValue::Integer(30));

        match bag.get("name") {
            Some(AttributeValue::String(s)) => assert_eq!(s, "John"),
            other => panic!("Expected String('John'), got {:?}", other),
        }
        match bag.get("age") {
            Some(AttributeValue::Integer(n)) => assert_eq!(*n, 30),
            other => panic!("Expected Integer(30), got {:?}", other),
        }
        assert!(bag.get("missing").is_none());
    }

    #[test]
    fn attribute_bag_has_remove() {
        let mut bag = AttributeBag::new();
        bag.set("key", AttributeValue::String("value".into()));
        assert!(bag.has("key"));
        bag.remove("key");
        assert!(!bag.has("key"));
    }

    #[test]
    fn attribute_value_variants() {
        let _ = AttributeValue::String("hello".into());
        let _ = AttributeValue::Integer(42);
        let _ = AttributeValue::Float(3.14);
        let _ = AttributeValue::Boolean(true);
        let _ = AttributeValue::Json(serde_json::json!({"key": "val"}));
        let _ = AttributeValue::Null;
    }

    // ── CastRegistry ─────────────────────────────────────────────

    #[test]
    fn cast_registry_builder_and_check() {
        let registry = CastRegistry::new()
            .cast("age", CastType::Integer)
            .cast("name", CastType::String)
            .cast("active", CastType::Boolean)
            .cast("settings", CastType::Json);

        assert!(registry.has("age"));
        assert!(registry.has("name"));
        assert!(registry.has("active"));
        assert!(registry.has("settings"));
        assert!(!registry.has("nonexistent"));

        assert_eq!(registry.get("age"), Some(&CastType::Integer));
    }

    #[test]
    fn cast_type_variants() {
        let types = vec![
            CastType::String,
            CastType::Integer,
            CastType::Float,
            CastType::Boolean,
            CastType::Json,
            CastType::DateTime,
            CastType::Date,
            CastType::Encrypted,
            CastType::Array,
            CastType::Collection,
        ];
        assert_eq!(types.len(), 10);
    }

    // ── ModelEvent ───────────────────────────────────────────────

    #[test]
    fn model_event_variants() {
        let events = vec![
            ModelEvent::Creating,
            ModelEvent::Created,
            ModelEvent::Updating,
            ModelEvent::Updated,
            ModelEvent::Saving,
            ModelEvent::Saved,
            ModelEvent::Deleting,
            ModelEvent::Deleted,
            ModelEvent::Restoring,
            ModelEvent::Restored,
        ];
        assert_eq!(events.len(), 10);
    }

    // ── EventDispatcher ──────────────────────────────────────────

    #[test]
    fn event_dispatcher_creation() {
        let dispatcher = EventDispatcher::new();
        let _ = dispatcher;
    }
}
