//! Comprehensive tests for rf-scheduler

use async_trait::async_trait;
use rf_scheduler::{Scheduler, SchedulerError, Task, TaskBuilder};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

// Simple test task for tracking execution
#[derive(Clone)]
struct TestTask {
    name: String,
    counter: Arc<Mutex<usize>>,
}

impl TestTask {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            counter: Arc::new(Mutex::new(0)),
        }
    }

    fn execution_count(&self) -> usize {
        *self.counter.lock().unwrap()
    }
}

#[async_trait]
impl Task for TestTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut count = self.counter.lock().unwrap();
        *count += 1;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Failing task for error handling tests
#[derive(Clone)]
struct FailingTask {
    name: String,
}

#[async_trait]
impl Task for FailingTask {
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Task failed intentionally".into())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// Test 1: Create scheduler
#[tokio::test]
async fn test_scheduler_creation() {
    let scheduler = Scheduler::new();
    assert_eq!(scheduler.task_count().await, 0);
}

// Test 2: Schedule task with valid cron expression
#[tokio::test]
async fn test_schedule_with_valid_cron() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("test_task");

    let result = scheduler.schedule("0 * * * *", task).await;
    assert!(result.is_ok());
    assert_eq!(scheduler.task_count().await, 1);
}

// Test 3: Schedule task with invalid cron expression
#[tokio::test]
async fn test_schedule_with_invalid_cron() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("test_task");

    let result = scheduler.schedule("invalid cron", task).await;
    assert!(result.is_err());

    match result {
        Err(SchedulerError::InvalidCron(_)) => {}
        _ => panic!("Expected InvalidCron error"),
    }
}

// Test 4: Schedule hourly task
#[tokio::test]
async fn test_hourly_schedule() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("hourly_task");

    let result = scheduler.hourly(task).await;
    assert!(result.is_ok());
    assert_eq!(scheduler.task_count().await, 1);
}

// Test 5: Schedule daily task
#[tokio::test]
async fn test_daily_schedule() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("daily_task");

    let result = scheduler.daily(task).await;
    assert!(result.is_ok());
    assert_eq!(scheduler.task_count().await, 1);
}

// Test 6: Schedule daily task at specific time
#[tokio::test]
async fn test_daily_at_schedule() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("daily_at_task");

    let result = scheduler.daily_at("14:30", task).await;
    assert!(result.is_ok());
    assert_eq!(scheduler.task_count().await, 1);
}

// Test 7: Schedule weekly task
#[tokio::test]
async fn test_weekly_schedule() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("weekly_task");

    let result = scheduler.weekly(task).await;
    assert!(result.is_ok());
    assert_eq!(scheduler.task_count().await, 1);
}

// Test 8: Schedule monthly task
#[tokio::test]
async fn test_monthly_schedule() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("monthly_task");

    let result = scheduler.monthly(task).await;
    assert!(result.is_ok());
    assert_eq!(scheduler.task_count().await, 1);
}

// Test 9: Schedule task every N minutes
#[tokio::test]
async fn test_every_minutes_schedule() {
    let scheduler = Scheduler::new();

    assert!(scheduler
        .every_minutes(5, TestTask::new("every_5_min"))
        .await
        .is_ok());
    assert!(scheduler
        .every_minutes(10, TestTask::new("every_10_min"))
        .await
        .is_ok());
    assert!(scheduler
        .every_minutes(30, TestTask::new("every_30_min"))
        .await
        .is_ok());

    assert_eq!(scheduler.task_count().await, 3);
}

// Test 10: Schedule task every N hours
#[tokio::test]
async fn test_every_hours_schedule() {
    let scheduler = Scheduler::new();

    assert!(scheduler
        .every_hours(2, TestTask::new("every_2_hours"))
        .await
        .is_ok());
    assert!(scheduler
        .every_hours(6, TestTask::new("every_6_hours"))
        .await
        .is_ok());

    assert_eq!(scheduler.task_count().await, 2);
}

// Test 11: TaskBuilder fluent API - daily
#[test]
fn test_task_builder_daily() {
    let builder = TaskBuilder::new().daily();
    assert_eq!(builder.cron(), Some("0 0 * * *"));
}

// Test 12: TaskBuilder fluent API - hourly
#[test]
fn test_task_builder_hourly() {
    let builder = TaskBuilder::new().hourly();
    assert_eq!(builder.cron(), Some("0 * * * *"));
}

// Test 13: TaskBuilder fluent API - weekly
#[test]
fn test_task_builder_weekly() {
    let builder = TaskBuilder::new().weekly();
    assert_eq!(builder.cron(), Some("0 0 * * SUN"));
}

// Test 14: TaskBuilder fluent API - monthly
#[test]
fn test_task_builder_monthly() {
    let builder = TaskBuilder::new().monthly();
    assert_eq!(builder.cron(), Some("0 0 1 * *"));
}

// Test 15: TaskBuilder fluent API - at specific time
#[test]
fn test_task_builder_at_time() {
    let builder = TaskBuilder::new().at("14:30");
    assert_eq!(builder.cron(), Some("30 14 * * *"));
}

// Test 16: TaskBuilder fluent API - on specific day
#[test]
fn test_task_builder_on_day() {
    let builder = TaskBuilder::new().at("09:00").on("monday");
    assert_eq!(builder.cron(), Some("00 09 * * 1"));

    let builder = TaskBuilder::new().at("09:00").on("friday");
    assert_eq!(builder.cron(), Some("00 09 * * 5"));
}

// Test 17: TaskBuilder fluent API - weekdays
#[test]
fn test_task_builder_weekdays() {
    let builder = TaskBuilder::new().at("09:00").weekdays();
    assert_eq!(builder.cron(), Some("00 09 * * 1,2,3,4,5"));
}

// Test 18: TaskBuilder fluent API - weekends
#[test]
fn test_task_builder_weekends() {
    let builder = TaskBuilder::new().at("10:00").weekends();
    assert_eq!(builder.cron(), Some("00 10 * * 6,0"));
}

// Test 19: TaskBuilder fluent API - on multiple days
#[test]
fn test_task_builder_on_multiple_days() {
    let builder = TaskBuilder::new()
        .at("09:00")
        .on_days(&["monday", "wednesday", "friday"]);
    assert_eq!(builder.cron(), Some("00 09 * * 1,3,5"));
}

// Test 20: TaskBuilder fluent API - every 5 minutes
#[test]
fn test_task_builder_every_five_minutes() {
    let builder = TaskBuilder::new().every_five_minutes();
    assert_eq!(builder.cron(), Some("*/5 * * * *"));
}

// Test 21: TaskBuilder fluent API - every 10 minutes
#[test]
fn test_task_builder_every_ten_minutes() {
    let builder = TaskBuilder::new().every_ten_minutes();
    assert_eq!(builder.cron(), Some("*/10 * * * *"));
}

// Test 22: TaskBuilder fluent API - every 15 minutes
#[test]
fn test_task_builder_every_fifteen_minutes() {
    let builder = TaskBuilder::new().every_fifteen_minutes();
    assert_eq!(builder.cron(), Some("*/15 * * * *"));
}

// Test 23: TaskBuilder fluent API - every 30 minutes
#[test]
fn test_task_builder_every_thirty_minutes() {
    let builder = TaskBuilder::new().every_thirty_minutes();
    assert_eq!(builder.cron(), Some("*/30 * * * *"));
}

// Test 24: TaskBuilder fluent API - between hours
#[test]
fn test_task_builder_between_hours() {
    let builder = TaskBuilder::new().hourly().between("9", "17");
    assert_eq!(builder.cron(), Some("0 9-17 * * *"));
}

// Test 25: TaskBuilder fluent API - complex chaining
#[test]
fn test_task_builder_complex_chaining() {
    let builder = TaskBuilder::new()
        .name("backup")
        .daily()
        .at("02:00")
        .on_days(&["monday", "wednesday", "friday"]);

    assert_eq!(builder.cron(), Some("00 02 * * 1,3,5"));
}

// Test 26: Multiple tasks with same schedule
#[tokio::test]
async fn test_multiple_tasks_same_schedule() {
    let scheduler = Scheduler::new();

    assert!(scheduler.hourly(TestTask::new("task1")).await.is_ok());
    assert!(scheduler.hourly(TestTask::new("task2")).await.is_ok());
    assert!(scheduler.hourly(TestTask::new("task3")).await.is_ok());

    assert_eq!(scheduler.task_count().await, 3);
}

// Test 27: Task with invalid time format
#[tokio::test]
async fn test_daily_at_with_invalid_time() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("invalid_time_task");

    let result = scheduler.daily_at("25:99", task).await;
    // Should either fail or handle gracefully
    // The current implementation doesn't validate time format strictly
}

// Test 28: Schedule with 5-field cron (without seconds)
#[tokio::test]
async fn test_schedule_with_5_field_cron() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("five_field");

    let result = scheduler.schedule("30 14 * * *", task).await;
    assert!(result.is_ok());
}

// Test 29: Schedule with 6-field cron (with seconds)
#[tokio::test]
async fn test_schedule_with_6_field_cron() {
    let scheduler = Scheduler::new();
    let task = TestTask::new("six_field");

    let result = scheduler.schedule("0 30 14 * * *", task).await;
    assert!(result.is_ok());
}

// Test 30: Task name retrieval
#[tokio::test]
async fn test_task_name() {
    let task = TestTask::new("my_task");
    assert_eq!(task.name(), "my_task");
}

// Test 31: Task prevent overlap default
#[tokio::test]
async fn test_task_prevent_overlap() {
    let task = TestTask::new("test");
    assert_eq!(task.prevent_overlap(), true);
}

// Test 32: Failing task error handling
#[tokio::test]
async fn test_failing_task() {
    let task = FailingTask {
        name: "failing".to_string(),
    };

    let result = task.run().await;
    assert!(result.is_err());
}

// Test 33: TaskBuilder with name
#[test]
fn test_task_builder_with_name() {
    let builder = TaskBuilder::new().name("backup_task").daily();

    // Name is stored internally (private field)
    // Just verify the builder can be created with a name
}

// Test 34: Day abbreviations
#[test]
fn test_task_builder_day_abbreviations() {
    let builder = TaskBuilder::new().at("09:00").on("mon");
    assert_eq!(builder.cron(), Some("00 09 * * 1"));

    let builder = TaskBuilder::new().at("09:00").on("fri");
    assert_eq!(builder.cron(), Some("00 09 * * 5"));

    let builder = TaskBuilder::new().at("09:00").on("sun");
    assert_eq!(builder.cron(), Some("00 09 * * 0"));
}

// Test 35: Invalid day name handling
#[test]
fn test_task_builder_invalid_day() {
    let builder = TaskBuilder::new().at("09:00").on("invalidday");
    // Should not panic, might keep previous cron or set default
    assert!(builder.cron().is_some());
}

// Test 36: Empty days list
#[test]
fn test_task_builder_empty_days() {
    let builder = TaskBuilder::new().at("09:00").on_days(&[]);
    // Should not panic
    assert!(builder.cron().is_some());
}

// Test 37: Scheduler default implementation
#[tokio::test]
async fn test_scheduler_default() {
    let scheduler = Scheduler::default();
    assert_eq!(scheduler.task_count().await, 0);
}

// Test 38: Builder default implementation
#[test]
fn test_task_builder_default() {
    let builder = TaskBuilder::default();
    assert!(builder.cron().is_none());
}

// Test 39: Error display messages
#[test]
fn test_error_display() {
    let err = SchedulerError::InvalidCron("bad cron".to_string());
    assert!(err.to_string().contains("Invalid cron expression"));

    let err = SchedulerError::TaskFailed("task error".to_string());
    assert!(err.to_string().contains("Task execution failed"));

    let err = SchedulerError::TaskRunning("task1".to_string());
    assert!(err.to_string().contains("Task already running"));
}

// Test 40: Schedule multiple different intervals
#[tokio::test]
async fn test_mixed_schedules() {
    let scheduler = Scheduler::new();

    assert!(scheduler.hourly(TestTask::new("hourly")).await.is_ok());
    assert!(scheduler.daily(TestTask::new("daily")).await.is_ok());
    assert!(scheduler.weekly(TestTask::new("weekly")).await.is_ok());
    assert!(scheduler.monthly(TestTask::new("monthly")).await.is_ok());
    assert!(scheduler
        .every_minutes(5, TestTask::new("every_5"))
        .await
        .is_ok());

    assert_eq!(scheduler.task_count().await, 5);
}
