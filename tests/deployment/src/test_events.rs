//! Deployment tests for rf-events

#[cfg(test)]
mod tests {
    use rf_events::{EventFacade, EventDispatcher, Event, EventListenerFor, EventResult};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // ── EventFacade ──────────────────────────────────────────────

    #[test]
    fn event_facade_dispatch_and_listen() {
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
        EventFacade::forget_all();
        EventFacade::listen("temp.event", |_| {});
        assert!(EventFacade::has_listeners("temp.event"));
        EventFacade::forget("temp.event");
        assert!(!EventFacade::has_listeners("temp.event"));
    }

    #[test]
    fn event_facade_forget_all() {
        EventFacade::forget_all();
        EventFacade::listen("a", |_| {});
        EventFacade::listen("b", |_| {});
        EventFacade::forget_all();
        assert!(!EventFacade::has_listeners("a"));
        assert!(!EventFacade::has_listeners("b"));
    }

    #[test]
    fn event_facade_history() {
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
        EventFacade::forget_all();
        EventFacade::listen("multi", |_| {});
        EventFacade::listen("multi", |_| {});
        EventFacade::listen("multi", |_| {});
        assert_eq!(EventFacade::listener_count("multi"), 3);
    }
}
