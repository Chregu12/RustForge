//! Time travel testing utilities
//!
//! Provides a fake Clock implementation that allows controlling time in tests,
//! inspired by Laravel's Carbon::setTestNow().

use chrono::{DateTime, Duration, Utc};
use std::sync::{Arc, Mutex, RwLock};

thread_local! {
    /// Thread-local storage for the global test clock
    static TEST_CLOCK: RwLock<Option<Box<dyn Clock>>> = RwLock::new(None);
}

/// Trait for clock implementations
pub trait Clock: Send + Sync {
    /// Get the current time according to this clock
    fn now(&self) -> DateTime<Utc>;
    /// Clone this clock into a Box
    fn clone_box(&self) -> Box<dyn Clock>;
}

/// System clock that returns real time
#[derive(Debug, Clone)]
pub struct SystemClock;

impl SystemClock {
    /// Create a new system clock
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn clone_box(&self) -> Box<dyn Clock> {
        Box::new(self.clone())
    }
}

/// State of the fake clock
#[derive(Debug, Clone)]
enum ClockState {
    Frozen(DateTime<Utc>),
    Real,
}

/// Fake clock for testing
#[derive(Clone)]
pub struct FakeClock {
    state: Arc<Mutex<ClockState>>,
}

impl FakeClock {
    /// Create a new fake clock in real-time mode
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ClockState::Real)),
        }
    }

    /// Create a new fake clock frozen at the current time
    pub fn frozen() -> Self {
        let now = Utc::now();
        Self {
            state: Arc::new(Mutex::new(ClockState::Frozen(now))),
        }
    }

    /// Create a new fake clock frozen at a specific time
    pub fn frozen_at(time: DateTime<Utc>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ClockState::Frozen(time))),
        }
    }

    /// Freeze time at the current moment
    pub fn freeze(&self) {
        let now = match *self.state.lock().unwrap() {
            ClockState::Frozen(time) => time,
            ClockState::Real => Utc::now(),
        };
        *self.state.lock().unwrap() = ClockState::Frozen(now);
    }

    /// Freeze time at a specific moment
    pub fn freeze_at(&self, time: DateTime<Utc>) {
        *self.state.lock().unwrap() = ClockState::Frozen(time);
    }

    /// Travel to a specific time (same as freeze_at)
    pub fn travel_to(&self, time: DateTime<Utc>) {
        self.freeze_at(time);
    }

    /// Unfreeze and return to real time
    pub fn unfreeze(&self) {
        *self.state.lock().unwrap() = ClockState::Real;
    }

    /// Travel back to real time (same as unfreeze)
    pub fn travel_back(&self) {
        self.unfreeze();
    }

    /// Check if time is currently frozen
    pub fn is_frozen(&self) -> bool {
        matches!(*self.state.lock().unwrap(), ClockState::Frozen(_))
    }

    /// Add seconds to the current time
    pub fn add_seconds(&self, seconds: i64) {
        self.add_duration(Duration::seconds(seconds));
    }

    /// Add minutes to the current time
    pub fn add_minutes(&self, minutes: i64) {
        self.add_duration(Duration::minutes(minutes));
    }

    /// Add hours to the current time
    pub fn add_hours(&self, hours: i64) {
        self.add_duration(Duration::hours(hours));
    }

    /// Add days to the current time
    pub fn add_days(&self, days: i64) {
        self.add_duration(Duration::days(days));
    }

    /// Subtract seconds from the current time
    pub fn sub_seconds(&self, seconds: i64) {
        self.add_duration(Duration::seconds(-seconds));
    }

    /// Subtract minutes from the current time
    pub fn sub_minutes(&self, minutes: i64) {
        self.add_duration(Duration::minutes(-minutes));
    }

    /// Subtract hours from the current time
    pub fn sub_hours(&self, hours: i64) {
        self.add_duration(Duration::hours(-hours));
    }

    /// Subtract days from the current time
    pub fn sub_days(&self, days: i64) {
        self.add_duration(Duration::days(-days));
    }

    fn add_duration(&self, duration: Duration) {
        let mut state = self.state.lock().unwrap();
        match *state {
            ClockState::Frozen(time) => {
                *state = ClockState::Frozen(time + duration);
            }
            ClockState::Real => {
                panic!("Cannot add time to unfrozen clock. Call freeze() first.");
            }
        }
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        match *self.state.lock().unwrap() {
            ClockState::Frozen(time) => time,
            ClockState::Real => Utc::now(),
        }
    }

    fn clone_box(&self) -> Box<dyn Clock> {
        Box::new(self.clone())
    }
}

impl std::fmt::Debug for FakeClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeClock")
            .field("state", &*self.state.lock().unwrap())
            .finish()
    }
}

/// High-level time fake API
#[derive(Clone)]
pub struct TimeFake {
    clock: FakeClock,
}

impl TimeFake {
    /// Create a new time fake
    pub fn new() -> Self {
        Self {
            clock: FakeClock::new(),
        }
    }

    /// Freeze time at the current moment
    pub fn freeze(&self) {
        self.clock.freeze();
    }

    /// Freeze time at a specific moment
    pub fn freeze_at(&self, time: DateTime<Utc>) {
        self.clock.freeze_at(time);
    }

    /// Travel to a specific time
    pub fn travel_to(&self, time: DateTime<Utc>) {
        self.clock.travel_to(time);
    }

    /// Travel back to real time
    pub fn travel_back(&self) {
        self.clock.travel_back();
    }

    /// Unfreeze and return to real time
    pub fn unfreeze(&self) {
        self.clock.unfreeze();
    }

    /// Check if time is currently frozen
    pub fn is_frozen(&self) -> bool {
        self.clock.is_frozen()
    }

    /// Get the current time according to this fake
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Add seconds to the current frozen time
    pub fn add_seconds(&self, seconds: i64) {
        self.clock.add_seconds(seconds);
    }

    /// Add minutes to the current frozen time
    pub fn add_minutes(&self, minutes: i64) {
        self.clock.add_minutes(minutes);
    }

    /// Add hours to the current frozen time
    pub fn add_hours(&self, hours: i64) {
        self.clock.add_hours(hours);
    }

    /// Add days to the current frozen time
    pub fn add_days(&self, days: i64) {
        self.clock.add_days(days);
    }

    /// Subtract seconds from the current frozen time
    pub fn sub_seconds(&self, seconds: i64) {
        self.clock.sub_seconds(seconds);
    }

    /// Subtract minutes from the current frozen time
    pub fn sub_minutes(&self, minutes: i64) {
        self.clock.sub_minutes(minutes);
    }

    /// Subtract hours from the current frozen time
    pub fn sub_hours(&self, hours: i64) {
        self.clock.sub_hours(hours);
    }

    /// Subtract days from the current frozen time
    pub fn sub_days(&self, days: i64) {
        self.clock.sub_days(days);
    }

    /// Get the underlying clock
    pub fn clock(&self) -> &FakeClock {
        &self.clock
    }
}

impl Default for TimeFake {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TimeFake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimeFake")
            .field("clock", &self.clock)
            .finish()
    }
}

// Global test clock functions

/// Set the global test clock
pub fn set_test_clock(clock: Box<dyn Clock>) {
    TEST_CLOCK.with(|c| {
        *c.write().unwrap() = Some(clock);
    });
}

/// Reset the global test clock to system time
pub fn reset_test_clock() {
    TEST_CLOCK.with(|c| {
        *c.write().unwrap() = None;
    });
}

/// Get the current time from the global test clock, or system time if none is set
pub fn current_time() -> DateTime<Utc> {
    TEST_CLOCK.with(|c| {
        let guard = c.read().unwrap();
        match guard.as_ref() {
            Some(clock) => clock.now(),
            None => Utc::now(),
        }
    })
}

/// Freeze time for the duration of a block
#[macro_export]
macro_rules! freeze_time {
    ($($body:tt)*) => {
        {
            let __clock = $crate::fakes::time::FakeClock::frozen();
            $crate::fakes::time::set_test_clock(Box::new(__clock.clone()));
            let __result = { $($body)* };
            $crate::fakes::time::reset_test_clock();
            __result
        }
    };
}

/// Travel to a specific time for the duration of a block
#[macro_export]
macro_rules! travel_to {
    ($time:expr, $($body:tt)*) => {
        {
            let __clock = $crate::fakes::time::FakeClock::frozen_at($time);
            $crate::fakes::time::set_test_clock(Box::new(__clock.clone()));
            let __result = { $($body)* };
            $crate::fakes::time::reset_test_clock();
            __result
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::thread::sleep;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_system_clock() {
        let clock = SystemClock::new();
        let now1 = clock.now();
        sleep(StdDuration::from_millis(10));
        let now2 = clock.now();
        assert!(now2 > now1, "System clock should advance");
    }

    #[test]
    fn test_fake_clock_new() {
        let clock = FakeClock::new();
        assert!(!clock.is_frozen());
    }

    #[test]
    fn test_fake_clock_frozen() {
        let clock = FakeClock::frozen();
        assert!(clock.is_frozen());

        let now1 = clock.now();
        sleep(StdDuration::from_millis(100));
        let now2 = clock.now();

        assert_eq!(now1, now2, "Frozen clock should not advance");
    }

    #[test]
    fn test_fake_clock_frozen_at() {
        let target = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();
        let clock = FakeClock::frozen_at(target);

        assert!(clock.is_frozen());
        assert_eq!(clock.now(), target);
    }

    #[test]
    fn test_freeze() {
        let clock = FakeClock::new();
        assert!(!clock.is_frozen());

        clock.freeze();
        assert!(clock.is_frozen());

        let now1 = clock.now();
        sleep(StdDuration::from_millis(50));
        let now2 = clock.now();

        assert_eq!(now1, now2);
    }

    #[test]
    fn test_freeze_at() {
        let clock = FakeClock::new();
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        clock.freeze_at(target);
        assert!(clock.is_frozen());
        assert_eq!(clock.now(), target);
    }

    #[test]
    fn test_unfreeze() {
        let clock = FakeClock::frozen();
        assert!(clock.is_frozen());

        clock.unfreeze();
        assert!(!clock.is_frozen());
    }

    #[test]
    fn test_travel_to_and_back() {
        let clock = FakeClock::new();
        let target = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();

        clock.travel_to(target);
        assert_eq!(clock.now(), target);

        clock.travel_back();
        assert!(!clock.is_frozen());
    }

    #[test]
    fn test_add_seconds() {
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let clock = FakeClock::frozen_at(target);

        clock.add_seconds(30);
        let expected = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 30).unwrap();
        assert_eq!(clock.now(), expected);
    }

    #[test]
    fn test_add_minutes() {
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let clock = FakeClock::frozen_at(target);

        clock.add_minutes(45);
        let expected = Utc.with_ymd_and_hms(2024, 1, 1, 12, 45, 0).unwrap();
        assert_eq!(clock.now(), expected);
    }

    #[test]
    fn test_add_hours() {
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let clock = FakeClock::frozen_at(target);

        clock.add_hours(3);
        let expected = Utc.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap();
        assert_eq!(clock.now(), expected);
    }

    #[test]
    fn test_add_days() {
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let clock = FakeClock::frozen_at(target);

        clock.add_days(7);
        let expected = Utc.with_ymd_and_hms(2024, 1, 8, 12, 0, 0).unwrap();
        assert_eq!(clock.now(), expected);
    }

    #[test]
    fn test_sub_hours() {
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 15, 0, 0).unwrap();
        let clock = FakeClock::frozen_at(target);

        clock.sub_hours(2);
        let expected = Utc.with_ymd_and_hms(2024, 1, 1, 13, 0, 0).unwrap();
        assert_eq!(clock.now(), expected);
    }

    #[test]
    #[should_panic(expected = "Cannot add time to unfrozen clock")]
    fn test_add_to_unfrozen_panics() {
        let clock = FakeClock::new();
        clock.add_seconds(10);
    }

    #[test]
    fn test_time_fake() {
        let fake = TimeFake::new();
        assert!(!fake.is_frozen());

        fake.freeze();
        assert!(fake.is_frozen());

        let now1 = fake.now();
        sleep(StdDuration::from_millis(50));
        let now2 = fake.now();

        assert_eq!(now1, now2);

        fake.unfreeze();
        assert!(!fake.is_frozen());
    }

    #[test]
    fn test_time_fake_travel() {
        let fake = TimeFake::new();
        let target = Utc.with_ymd_and_hms(2024, 6, 15, 10, 30, 0).unwrap();

        fake.travel_to(target);
        assert_eq!(fake.now(), target);

        fake.add_hours(2);
        let expected = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();
        assert_eq!(fake.now(), expected);

        fake.travel_back();
        assert!(!fake.is_frozen());
    }

    #[test]
    fn test_global_test_clock() {
        let before = current_time();
        sleep(StdDuration::from_millis(10));
        let after = current_time();
        assert!(after > before);

        let target = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let clock = FakeClock::frozen_at(target);
        set_test_clock(Box::new(clock));

        let t1 = current_time();
        sleep(StdDuration::from_millis(50));
        let t2 = current_time();

        assert_eq!(t1, target);
        assert_eq!(t2, target);

        reset_test_clock();
    }

    #[test]
    fn test_chaining_time_operations() {
        let target = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let fake = TimeFake::new();

        fake.freeze_at(target);
        fake.add_days(1);
        fake.add_hours(6);
        fake.add_minutes(30);

        let expected = Utc.with_ymd_and_hms(2024, 1, 2, 18, 30, 0).unwrap();
        assert_eq!(fake.now(), expected);

        fake.sub_hours(2);
        let expected = Utc.with_ymd_and_hms(2024, 1, 2, 16, 30, 0).unwrap();
        assert_eq!(fake.now(), expected);
    }
}
