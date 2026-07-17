//! Typed, synchronous, in-process event bus.
//!
//! This is the engine behind the vision's single-argument event surface:
//! `event(payload)` dispatches by the payload's *type* (keyed by [`TypeId`]),
//! synchronously invoking every listener registered for that type in-process.
//! No `.await`, no serialization, no runtime required — the listener receives a
//! reference to the concrete event value.
//!
//! This is distinct from the string-keyed [`crate::Event`] facade
//! (`Event::dispatch("name", data)`), which routes JSON payloads by event name.
//! Both are real; this module adds the typed surface the `event!` / `dispatch!`
//! macros expand to.
//!
//! ```
//! use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
//!
//! struct UserRegistered { id: u64 }
//!
//! let hits = Arc::new(AtomicUsize::new(0));
//! let h = hits.clone();
//! rf_event_facade::listen::<UserRegistered, _>(move |e: &UserRegistered| {
//!     assert_eq!(e.id, 7);
//!     h.fetch_add(1, Ordering::SeqCst);
//! });
//!
//! let fired = rf_event_facade::event(UserRegistered { id: 7 });
//! assert_eq!(fired, 1);
//! assert_eq!(hits.load(Ordering::SeqCst), 1);
//! ```

use once_cell::sync::Lazy;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A type-erased listener. Receives the event as `&dyn Any` and downcasts to the
/// concrete type it was registered for.
type ErasedListener = Arc<dyn Fn(&(dyn Any + Send + Sync)) + Send + Sync>;

/// Global registry mapping an event's [`TypeId`] to its listeners.
static TYPED_BUS: Lazy<RwLock<HashMap<TypeId, Vec<ErasedListener>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register a listener for events of type `E`.
///
/// The closure is invoked (synchronously, in-process) every time an `E` value is
/// dispatched via [`event`]. Multiple listeners for the same type are all fired,
/// in registration order.
pub fn listen<E, F>(callback: F)
where
    E: Send + Sync + 'static,
    F: Fn(&E) + Send + Sync + 'static,
{
    let erased: ErasedListener = Arc::new(move |any: &(dyn Any + Send + Sync)| {
        // Downcast can only fail if the bus is misused via unsafe; the public
        // API guarantees the stored TypeId matches E, so this always succeeds.
        if let Some(event) = any.downcast_ref::<E>() {
            callback(event);
        }
    });

    TYPED_BUS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .entry(TypeId::of::<E>())
        .or_default()
        .push(erased);
}

/// Dispatch `payload` to every listener registered for its type.
///
/// Returns the number of listeners that were invoked. Dispatching an event with
/// no registered listeners is a safe no-op that returns `0`.
///
/// Listeners are cloned out from under the lock before being invoked, so a
/// listener may itself call [`listen`] or [`event`] without deadlocking.
pub fn event<E>(payload: E) -> usize
where
    E: Send + Sync + 'static,
{
    let listeners: Vec<ErasedListener> = {
        let bus = TYPED_BUS.read().unwrap_or_else(|e| e.into_inner());
        match bus.get(&TypeId::of::<E>()) {
            Some(list) => list.clone(),
            None => return 0,
        }
    };

    let erased: &(dyn Any + Send + Sync) = &payload;
    for listener in &listeners {
        listener(erased);
    }
    listeners.len()
}

/// Dispatch `payload` to its type's listeners after `delay` has elapsed.
///
/// A real delayed dispatch: a background OS thread sleeps for `delay` and then
/// performs a normal synchronous [`event`] dispatch. Fire-and-forget; requires no
/// async runtime. `payload` must be `Send + 'static` so it can cross the thread
/// boundary.
pub fn event_later<E>(payload: E, delay: std::time::Duration)
where
    E: Send + Sync + 'static,
{
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        event(payload);
    });
}

/// Number of listeners currently registered for event type `E`.
pub fn typed_listener_count<E: 'static>() -> usize {
    TYPED_BUS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(&TypeId::of::<E>())
        .map(|l| l.len())
        .unwrap_or(0)
}

/// Remove all listeners registered for event type `E`.
pub fn forget<E: 'static>() {
    TYPED_BUS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&TypeId::of::<E>());
}

/// Remove every typed listener for every event type.
pub fn forget_all() {
    TYPED_BUS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Serializes tests: they share the process-global TYPED_BUS and use
    // forget/forget_all. Distinct event structs per test keep types isolated,
    // but forget_all would still race, so hold this guard.
    static GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[derive(Clone)]
    struct Greeted {
        who: String,
    }

    #[test]
    fn single_listener_receives_payload() {
        let _g = GUARD.lock().unwrap_or_else(|e| e.into_inner());
        forget::<Greeted>();

        let seen = Arc::new(RwLock::new(Vec::<String>::new()));
        let s = seen.clone();
        listen::<Greeted, _>(move |e| s.write().unwrap().push(e.who.clone()));

        let fired = event(Greeted { who: "alice".into() });
        assert_eq!(fired, 1);
        assert_eq!(*seen.read().unwrap(), vec!["alice".to_string()]);
        forget::<Greeted>();
    }

    #[derive(Clone)]
    struct Pinged;

    #[test]
    fn multiple_listeners_all_fire() {
        let _g = GUARD.lock().unwrap_or_else(|e| e.into_inner());
        forget::<Pinged>();

        let count = Arc::new(AtomicUsize::new(0));
        for _ in 0..3 {
            let c = count.clone();
            listen::<Pinged, _>(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert_eq!(typed_listener_count::<Pinged>(), 3);

        let fired = event(Pinged);
        assert_eq!(fired, 3);
        assert_eq!(count.load(Ordering::SeqCst), 3);
        forget::<Pinged>();
    }

    struct Orphan;

    #[test]
    fn no_listeners_is_noop() {
        let _g = GUARD.lock().unwrap_or_else(|e| e.into_inner());
        forget::<Orphan>();
        assert_eq!(typed_listener_count::<Orphan>(), 0);
        assert_eq!(event(Orphan), 0);
    }

    struct Delayed {
        n: u64,
    }

    #[test]
    fn event_later_dispatches_on_background_thread() {
        let _g = GUARD.lock().unwrap_or_else(|e| e.into_inner());
        forget::<Delayed>();

        let got = Arc::new(AtomicUsize::new(0));
        let g = got.clone();
        listen::<Delayed, _>(move |e| {
            g.store(e.n as usize, Ordering::SeqCst);
        });

        event_later(Delayed { n: 42 }, std::time::Duration::from_millis(20));

        // Poll for up to ~1s for the background thread to fire.
        let mut fired = false;
        for _ in 0..100 {
            if got.load(Ordering::SeqCst) == 42 {
                fired = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(fired, "event_later never dispatched");
        forget::<Delayed>();
    }
}
