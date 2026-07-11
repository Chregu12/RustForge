// Integration probe: eager_bidirectional
// Adapted from sandbox/probes/eager_bidirectional/src/main.rs
// Proves bidirectional hasMany/belongsTo relations compile + hydrate in BOTH directions.

use rf_db_facade::DB;

rf::prelude::Model!(User {
    name: String,
});

rf::prelude::Model!(Project {
    name: String,

    hasMany tasks: Task,
});

rf::prelude::Model!(Task {
    title: String,
    project_id: i64,
    assignee_id: i64,

    belongsTo project: Project,
    belongsTo assignee: User,
});

#[tokio::test]
async fn test_eager_bidirectional() {
    DB::statement("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
    DB::statement("CREATE TABLE projects (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
    DB::statement(
        "CREATE TABLE tasks (id INTEGER PRIMARY KEY, title TEXT, project_id INTEGER, assignee_id INTEGER)",
    )
    .unwrap();

    DB::statement("INSERT INTO users (id, name) VALUES (1, 'Ann'), (2, 'Bob')").unwrap();
    DB::statement("INSERT INTO projects (id, name) VALUES (10, 'Apollo'), (11, 'Zephyr')").unwrap();
    DB::statement(
        "INSERT INTO tasks (id, title, project_id, assignee_id) VALUES \
         (100, 'design', 10, 1), (101, 'build', 10, 2), (102, 'ship', 11, 1)",
    )
    .unwrap();

    // Direction 1: Project hasMany tasks -> project.tasks populated.
    let mut projects = Project::with(&["tasks"])
        .get()
        .await
        .expect("Project::with(tasks).get() should succeed");
    projects.sort_by_key(|p| p.id);
    assert_eq!(projects[0].id, Some(10));
    assert_eq!(projects[0].tasks.len(), 2, "Apollo has 2 tasks");
    assert_eq!(projects[1].tasks.len(), 1, "Zephyr has 1 task");

    // Direction 2: Task belongsTo project -> task.project populated.
    let mut tasks = Task::with(&["project", "assignee"])
        .get()
        .await
        .expect("Task::with(project, assignee).get() should succeed");
    tasks.sort_by_key(|t| t.id);
    assert_eq!(
        tasks[0].project.as_ref().map(|p| p.name.as_str()),
        Some("Apollo"),
        "task 100 project hydrated -> Apollo"
    );
    assert_eq!(
        tasks[0].assignee.as_ref().map(|u| u.name.as_str()),
        Some("Ann"),
        "task 100 assignee hydrated -> Ann"
    );
    assert_eq!(
        tasks[2].project.as_ref().map(|p| p.name.as_str()),
        Some("Zephyr"),
        "task 102 project hydrated -> Zephyr"
    );

    // Nested across the cycle: project.tasks.assignee.
    let mut deep = Project::with(&["tasks.assignee"])
        .get()
        .await
        .expect("Project::with(tasks.assignee).get() should succeed");
    deep.sort_by_key(|p| p.id);
    assert!(
        deep[0].tasks.iter().all(|t| t.assignee.is_some()),
        "every task under Apollo has its assignee hydrated"
    );
}
