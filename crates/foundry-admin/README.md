# Foundry Admin

Admin Panel & Dashboard for Foundry Core - Filament/Nova-style admin interface.

## Features

- **CRUD Operations**: Automatic CRUD generation for models
- **Dashboard Widgets**: Customizable dashboard with metrics, charts, and tables
- **Model Inspector**: Automatically list and manage all models
- **User Management**: Built-in user management interface
- **Settings UI**: Configuration management
- **Activity Log**: Track user actions
- **Authentication**: Integrated auth middleware

## Quick Start

```rust
use foundry_admin::{AdminPanel, AdminConfig};

let config = AdminConfig::default()
    .with_prefix("/admin")
    .with_auth(true)
    .with_title("My Admin Panel");

let panel = AdminPanel::new(config);

// Register resources
panel.register_resource("users", Arc::new(UserResource));
```

## CLI Commands

```bash
# Generate admin resource
foundry make:admin-resource User

# Publish admin configuration
foundry admin:publish
```

## Admin Routes

- `/admin` - Dashboard
- `/admin/login` - Login page
- `/admin/resources` - Resource list
- `/admin/resources/{resource}` - CRUD interface
- `/admin/users` - User management
- `/admin/settings` - Settings
- `/admin/activity` - Activity log

## Creating Resources

```rust
use foundry_admin::{AdminResource, ResourceConfig, FieldConfig};
use async_trait::async_trait;

pub struct UserResource;

#[async_trait]
impl AdminResource for UserResource {
    fn config(&self) -> &ResourceConfig {
        // Define fields, searchable, filterable fields
    }

    async fn list(&self, query: ListQuery) -> anyhow::Result<ListResult> {
        // Implement listing logic
    }

    async fn get(&self, id: &str) -> anyhow::Result<Option<Value>> {
        // Implement get logic
    }

    // ... create, update, delete
}
```

## Dashboard Widgets

```rust
use foundry_admin::{MetricWidget, ChartWidget, TableWidget};

// Add metric widget
dashboard.add_widget(Arc::new(MetricWidget::new(
    "users",
    "Total Users",
    || Ok(MetricValue {
        value: "1,234".to_string(),
        label: "users".to_string(),
        trend: Some(Trend {
            direction: TrendDirection::Up,
            percentage: 12.5,
        }),
    }),
)));

// Add chart widget
dashboard.add_widget(Arc::new(ChartWidget::new(
    "revenue",
    "Monthly Revenue",
    ChartType::Line,
    || Ok(ChartData {
        labels: vec!["Jan".to_string(), "Feb".to_string()],
        datasets: vec![Dataset {
            label: "Revenue".to_string(),
            data: vec![1000.0, 1500.0],
            color: Some("#3b82f6".to_string()),
        }],
    }),
)));
```
