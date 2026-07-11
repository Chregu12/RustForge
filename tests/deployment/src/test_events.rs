//! Deployment tests for rf-events

#[cfg(test)]
mod tests {
    use rf_events::{EventFacade, EventDispatcher, Event, EventListenerFor, EventResult};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Serializes every EventFacade test. All tests share the process-global event
    /// manager: a concurrent `forget_all()` removes listeners registered by another
    /// in-flight test, so the dispatch call finds no listener and `called` stays false.
    /// `into_inner` ignores poisoning so a failing test doesn't cascade.
    static EVENT_GLOBAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // ── EventFacade ──────────────────────────────────────────────

    #[test]
    fn event_facade_dispatch_and_listen() {
        let _guard = EVENT_GLOBAL_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::forget_all();

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        EventFacade::listen("user.created", move |_data| {
            called_clone.store(true, Ordering::SeqCst);
        });

        assert!(EventFacade::has_listeners("user.created"));
        assert_eq!(EventFacade::listener_count("user.created"), 1);

        EventFacade::dispatch("user.created", json!({"id": 1})).expect("dispatch");
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn event_facade_forget() {
        let _guard = EVENT_GLOBAL_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::forget_all();
        EventFacade::listen("temp.event", |_| {});
        assert!(EventFacade::has_listeners("temp.event"));
        EventFacade::forget("temp.event");
        assert!(!EventFacade::has_listeners("temp.event"));
    }

    #[test]
    fn event_facade_forget_all() {
        let _guard = EVENT_GLOBAL_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::forget_all();
        EventFacade::listen("a", |_| {});
        EventFacade::listen("b", |_| {});
        EventFacade::forget_all();
        assert!(!EventFacade::has_listeners("a"));
        assert!(!EventFacade::has_listeners("b"));
    }

    #[test]
    fn event_facade_history() {
        let _guard = EVENT_GLOBAL_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::forget_all();
        EventFacade::clear_history();
        EventFacade::dispatch("test.event", json!({"key": "value"})).expect("dispatch");
        let history = EventFacade::history();
        assert!(!history.is_empty());
    }

    // ── EventDispatcher ──────────────────────────────────────────

    #[derive(Clone)]
    struct UserCreated {
        _user_id: i64,
    }

    impl Event for UserCreated {}

    struct LogListener;

    #[async_trait]
    impl EventListenerFor<UserCreated> for LogListener {
        async fn handle(&self, _event: &UserCreated) -> EventResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn event_dispatcher_typed() {
        let dispatcher = EventDispatcher::new();
        dispatcher.listen(LogListener).await;

        let count = dispatcher.listener_count::<UserCreated>().await;
        assert_eq!(count, 1);

        let event = UserCreated { _user_id: 42 };
        dispatcher.dispatch(event).await.expect("dispatch");
    }

    // ── Multiple Listeners ───────────────────────────────────────

    #[test]
    fn event_facade_multiple_listeners() {
        let _guard = EVENT_GLOBAL_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        EventFacade::forget_all();
        EventFacade::listen("multi", |_| {});
        EventFacade::listen("multi", |_| {});
        EventFacade::listen("multi", |_| {});
        assert_eq!(EventFacade::listener_count("multi"), 3);
    }
}
