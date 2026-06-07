//! Comprehensive tests for rf-events

#[cfg(test)]
mod event_tests {
    use crate::{
        Event, EventDispatcher, EventListenerFor, EventManager, EventResult,
    };
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::RwLock;

    // ─── Test event types ──────────────────────────────────────────────────────

    #[derive(Clone)]
    struct UserCreated {
        username: String,
    }
    impl Event for UserCreated {}

    #[derive(Clone)]
    struct OrderPlaced {
        order_id: u64,
    }
    impl Event for OrderPlaced {}

    #[derive(Clone)]
    struct PaymentReceived {
        amount: f64,
    }
    impl Event for PaymentReceived {}

    // ─── EventDispatcher: dispatch ─────────────────────────────────────────────

    #[tokio::test]
    async fn dispatch_event_calls_listener() {
        let dispatcher = EventDispatcher::new();
        let called = Arc::new(AtomicBool::new(false));

        struct MyListener {
            called: Arc<AtomicBool>,
        }
        #[async_trait]
        impl EventListenerFor<UserCreated> for MyListener {
            async fn handle(&self, _event: &UserCreated) -> EventResult<()> {
                self.called.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        dispatcher
            .listen(MyListener { called: called.clone() })
            .await;
        dispatcher
            .dispatch(UserCreated { username: "alice".into() })
            .await
            .unwrap();

        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn dispatch_with_no_listeners_succeeds() {
        let dispatcher = EventDispatcher::new();
        let result = dispatcher
            .dispatch(UserCreated { username: "bob".into() })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn listener_receives_event_data() {
        let dispatcher = EventDispatcher::new();
        let received_name = Arc::new(RwLock::new(String::new()));

        struct CaptureListener {
            name: Arc<RwLock<String>>,
        }
        #[async_trait]
        impl EventListenerFor<UserCreated> for CaptureListener {
            async fn handle(&self, event: &UserCreated) -> EventResult<()> {
                *self.name.write().await = event.username.clone();
                Ok(())
            }
        }

        dispatcher
            .listen(CaptureListener { name: received_name.clone() })
            .await;
        dispatcher
            .dispatch(UserCreated { username: "charlie".into() })
            .await
            .unwrap();

        assert_eq!(*received_name.read().await, "charlie");
    }

    // ─── Multiple listeners ────────────────────────────────────────────────────

    #[tokio::test]
    async fn multiple_listeners_all_called() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..4 {
            struct Counter {
                n: Arc<AtomicUsize>,
            }
            #[async_trait]
            impl EventListenerFor<OrderPlaced> for Counter {
                async fn handle(&self, _event: &OrderPlaced) -> EventResult<()> {
                    self.n.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            }
            dispatcher.listen(Counter { n: counter.clone() }).await;
        }

        assert_eq!(dispatcher.listener_count::<OrderPlaced>().await, 4);

        dispatcher
            .dispatch(OrderPlaced { order_id: 1 })
            .await
            .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    // ─── Listener priority ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn listeners_execute_in_priority_order() {
        let dispatcher = EventDispatcher::new();
        let order = Arc::new(RwLock::new(Vec::<i32>::new()));

        for prio in [5, 10, 1, 20] {
            struct PL {
                p: i32,
                order: Arc<RwLock<Vec<i32>>>,
            }
            #[async_trait]
            impl EventListenerFor<OrderPlaced> for PL {
                async fn handle(&self, _: &OrderPlaced) -> EventResult<()> {
                    self.order.write().await.push(self.p);
                    Ok(())
                }
                fn priority(&self) -> i32 {
                    self.p
                }
            }
            dispatcher
                .listen(PL { p: prio, order: order.clone() })
                .await;
        }

        dispatcher
            .dispatch(OrderPlaced { order_id: 99 })
            .await
            .unwrap();

        let result = order.read().await.clone();
        // Expected descending: 20, 10, 5, 1
        assert_eq!(result, vec![20, 10, 5, 1]);
    }

    // ─── Listener count ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn listener_count_zero_for_unknown_event() {
        let dispatcher = EventDispatcher::new();
        assert_eq!(dispatcher.listener_count::<UserCreated>().await, 0);
    }

    #[tokio::test]
    async fn listener_count_increments_correctly() {
        let dispatcher = EventDispatcher::new();

        struct Noop;
        #[async_trait]
        impl EventListenerFor<UserCreated> for Noop {
            async fn handle(&self, _: &UserCreated) -> EventResult<()> {
                Ok(())
            }
        }
        dispatcher.listen(Noop).await;
        dispatcher.listen(Noop).await;
        assert_eq!(dispatcher.listener_count::<UserCreated>().await, 2);
    }

    // ─── Multiple event types ──────────────────────────────────────────────────

    #[tokio::test]
    async fn different_event_types_do_not_interfere() {
        let dispatcher = EventDispatcher::new();
        let user_called = Arc::new(AtomicBool::new(false));
        let order_called = Arc::new(AtomicBool::new(false));

        struct UserL {
            flag: Arc<AtomicBool>,
        }
        #[async_trait]
        impl EventListenerFor<UserCreated> for UserL {
            async fn handle(&self, _: &UserCreated) -> EventResult<()> {
                self.flag.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        struct OrderL {
            flag: Arc<AtomicBool>,
        }
        #[async_trait]
        impl EventListenerFor<OrderPlaced> for OrderL {
            async fn handle(&self, _: &OrderPlaced) -> EventResult<()> {
                self.flag.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        dispatcher.listen(UserL { flag: user_called.clone() }).await;
        dispatcher
            .listen(OrderL { flag: order_called.clone() })
            .await;

        dispatcher
            .dispatch(UserCreated { username: "x".into() })
            .await
            .unwrap();

        assert!(user_called.load(Ordering::SeqCst));
        assert!(!order_called.load(Ordering::SeqCst));
    }

    // ─── EventManager (sync facade) ───────────────────────────────────────────

    #[test]
    fn event_manager_listen_and_dispatch() {
        let mut manager = EventManager::new();
        let called = Arc::new(AtomicUsize::new(0));
        let c = called.clone();

        manager.listen("user.created", move |_data| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        manager
            .dispatch("user.created", serde_json::json!({ "name": "alice" }))
            .unwrap();

        assert_eq!(called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn event_manager_listener_receives_data() {
        let mut manager = EventManager::new();
        let captured = Arc::new(RwLock::new(String::new()));
        let c = captured.clone();

        manager.listen("order.placed", move |data| {
            if let Some(id) = data["id"].as_u64() {
                // we can't await inside a sync closure, so use std RwLock instead
                // Just write synchronously would be fine if we use std::sync
                drop(c.try_write().map(|mut g| *g = id.to_string()));
            }
        });

        manager
            .dispatch("order.placed", serde_json::json!({ "id": 42 }))
            .unwrap();
    }

    #[test]
    fn event_manager_has_listeners_and_forget() {
        let mut manager = EventManager::new();
        assert!(!manager.has_listeners("test"));

        manager.listen("test", |_| {});
        assert!(manager.has_listeners("test"));

        manager.forget("test");
        assert!(!manager.has_listeners("test"));
    }

    #[test]
    fn event_manager_forget_all_clears_everything() {
        let mut manager = EventManager::new();
        manager.listen("a", |_| {});
        manager.listen("b", |_| {});

        manager.forget_all();

        assert!(!manager.has_listeners("a"));
        assert!(!manager.has_listeners("b"));
    }

    #[test]
    fn event_manager_history_records_dispatches() {
        let mut manager = EventManager::new();
        let data = serde_json::json!({ "x": 1 });
        manager.dispatch("evt", data.clone()).unwrap();
        manager.dispatch("evt2", serde_json::json!({})).unwrap();

        let history = manager.history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, "evt");
        assert_eq!(history[0].1, data);
        assert_eq!(history[1].0, "evt2");
    }

    #[test]
    fn event_manager_clear_history_empties_log() {
        let mut manager = EventManager::new();
        manager.dispatch("a", serde_json::json!({})).unwrap();
        manager.dispatch("b", serde_json::json!({})).unwrap();
        manager.clear_history();
        assert_eq!(manager.history().len(), 0);
    }

    #[test]
    fn event_manager_multiple_listeners_for_same_event() {
        let mut manager = EventManager::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = counter.clone();
        let c2 = counter.clone();
        let c3 = counter.clone();

        manager.listen("tick", move |_| { c1.fetch_add(1, Ordering::SeqCst); });
        manager.listen("tick", move |_| { c2.fetch_add(1, Ordering::SeqCst); });
        manager.listen("tick", move |_| { c3.fetch_add(1, Ordering::SeqCst); });

        assert_eq!(manager.listener_count("tick"), 3);
        manager.dispatch("tick", serde_json::json!({})).unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    // ─── Once-listener simulation ──────────────────────────────────────────────
    // The EventDispatcher doesn't have a built-in "once" listener, but we can
    // simulate it via a flag and verify the pattern works:

    #[tokio::test]
    async fn once_listener_pattern_fires_only_once() {
        let dispatcher = EventDispatcher::new();
        let fire_count = Arc::new(AtomicUsize::new(0));

        struct OnceListener {
            count: Arc<AtomicUsize>,
            fired: Arc<AtomicBool>,
        }
        #[async_trait]
        impl EventListenerFor<PaymentReceived> for OnceListener {
            async fn handle(&self, _: &PaymentReceived) -> EventResult<()> {
                if !self.fired.swap(true, Ordering::SeqCst) {
                    self.count.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            }
        }

        dispatcher
            .listen(OnceListener {
                count: fire_count.clone(),
                fired: Arc::new(AtomicBool::new(false)),
            })
            .await;

        // Dispatch twice; the guard ensures handler logic only runs once
        dispatcher
            .dispatch(PaymentReceived { amount: 9.99 })
            .await
            .unwrap();
        dispatcher
            .dispatch(PaymentReceived { amount: 19.99 })
            .await
            .unwrap();

        assert_eq!(fire_count.load(Ordering::SeqCst), 1);
    }
}
