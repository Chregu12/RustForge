# rf-nova

> Laravel Nova-inspired admin panel framework for RustForge

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

A powerful, flexible admin panel framework inspired by Laravel Nova, providing beautiful CRUD interfaces, dashboards, metrics, and more for your RustForge applications.

## Features

### 🎨 **Resources**
- Map your models to beautiful admin interfaces
- Full CRUD operations with validation
- Searchable and sortable fields
- Relationship handling (BelongsTo, HasMany, HasOne)
- Custom field types (Text, Email, Password, Boolean, DateTime, Select, File, Image, etc.)
- Field visibility control per context (index, detail, create, update)

### ⚡ **Actions**
- Perform bulk operations on resources
- Standalone actions without resource selection
- Custom action fields for user input
- Destructive action warnings
- Success/error feedback

### 🔍 **Filters**
- Filter resources by custom criteria
- Select, Boolean, Date, and DateRange filters
- Preset filters (Active, Trashed)
- Custom filter components

### 🔭 **Lenses**
- Create custom query views
- Aggregate data with custom SQL
- Joins, group by, having clauses
- Preset lenses (Most Recent, Top Items)

### 📊 **Metrics**
Three types of metrics for dashboards:

- **Value Metrics**: Single numbers with comparison
- **Trend Metrics**: Line charts over time
- **Partition Metrics**: Pie/donut charts

### 🎴 **Cards**
- Custom dashboard widgets
- Help cards, table cards, progress cards
- Vue component integration
- Auto-refresh support

### 📈 **Dashboards**
- Aggregate metrics and cards
- Multiple dashboards
- Customizable layouts

### 🔐 **Authorization**
- Policy-based access control
- Resource-level permissions
- Preset policies (AllowAll, AdminOnly, ReadOnly, Owner-based)
- Per-action authorization

### 📤 **Export**
- Export to CSV or JSON
- Filtered exports
- Custom formatters

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-nova = { path = "../rf-nova" }
```

## Quick Start

```rust
use rf_nova::prelude::*;

#[tokio::main]
async fn main() {
    // Create Nova instance
    let nova = Nova::new()
        .with_path("/admin")
        .with_name("My Admin Panel")
        .with_primary_color("#4299E1")
        .register_resource::<UserResource>()
        .register_resource::<PostResource>()
        .register_dashboard(MainDashboard);

    // Add to your Axum app
    let app = Router::new()
        .merge(nova.routes());

    // Serve your app
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

## Creating Resources

```rust
use rf_nova::prelude::*;

pub struct UserResource;

impl Resource for UserResource {
    type Entity = user::Entity;
    type Model = user::Model;

    fn name() -> &'static str {
        "User"
    }

    fn group() -> Option<&'static str> {
        Some("User Management")
    }

    fn fields() -> Vec<Box<dyn Field>> {
        vec![
            Box::new(ID::new("id")),
            Box::new(
                Text::new("name")
                    .sortable()
                    .searchable()
                    .rules("required|min:3")
            ),
            Box::new(
                Email::new("email")
                    .sortable()
                    .searchable()
                    .rules("required|email")
            ),
            Box::new(Password::new("password").hide_on_index()),
            Box::new(Boolean::new("is_admin")),
            Box::new(DateTime::new("created_at").sortable()),
            Box::new(HasMany::new("posts", "Post")),
        ]
    }

    fn actions() -> Vec<Box<dyn Action>> {
        vec![
            Box::new(DeactivateUser),
            Box::new(ExportAction::csv()),
        ]
    }

    fn filters() -> Vec<Box<dyn Filter>> {
        vec![
            Box::new(
                SelectFilter::new("User Type", "is_admin")
                    .option("true", "Administrators")
                    .option("false", "Regular Users")
            ),
        ]
    }
}
```

## Field Types

### Text Fields
```rust
Text::new("name")
    .sortable()
    .searchable()
    .rules("required|min:3")
    .placeholder("Enter name")
    .help("Full name of the user")
```

### Email Field
```rust
Email::new("email")
    .sortable()
    .searchable()
    .rules("required|email")
```

### Password Field
```rust
Password::new("password")
    .rules("required|min:8")
    .hide_on_index()  // Not shown on list view
```

### Boolean Field
```rust
Boolean::new("is_active")
    .sortable()
    .labels("Active", "Inactive")
```

### DateTime Field
```rust
DateTime::new("created_at")
    .sortable()
    .format("%Y-%m-%d %H:%M:%S")
```

### Select Field
```rust
Select::new("role")
    .options(vec![
        SelectOption::new("admin", "Administrator"),
        SelectOption::new("user", "Regular User"),
    ])
    .searchable()
```

### Relationship Fields
```rust
// BelongsTo
BelongsTo::new("user", "User")
    .foreign_key("user_id")
    .display("name")
    .searchable()

// HasMany
HasMany::new("posts", "Post")
    .foreign_key("user_id")
```

### File Upload
```rust
File::new("document")
    .disk("local")
    .path("documents")
    .accept(vec!["pdf", "doc"])
    .max_size(5_000_000)  // 5MB
```

### Image Upload
```rust
Image::new("avatar")
    .preview(150, 150)
    .max_size(2_000_000)  // 2MB
```

## Creating Actions

```rust
use rf_nova::prelude::*;

pub struct DeactivateUser;

#[async_trait]
impl Action for DeactivateUser {
    fn name(&self) -> &str {
        "Deactivate Users"
    }

    fn destructive(&self) -> bool {
        true  // Shows red warning
    }

    fn fields(&self) -> Vec<ActionField> {
        vec![
            ActionField::textarea("reason")
                .label("Reason for deactivation")
                .rules("required|min:10")
                .help("Please provide a detailed reason"),
        ]
    }

    async fn handle(&self, models: Vec<Value>, fields: ActionFields) -> ActionResult {
        let reason = fields.get_string("reason").unwrap();

        for model in models {
            // Update your database here
            // User::find(id).update(active: false).await?;
        }

        ActionResponse::success(format!("Deactivated {} user(s)", models.len()))
    }
}
```

## Creating Filters

```rust
use rf_nova::prelude::*;

// Select filter
let user_type_filter = SelectFilter::new("User Type", "type")
    .option("admin", "Administrators")
    .option("user", "Regular Users")
    .option("guest", "Guests");

// Boolean filter
let active_filter = BooleanFilter::new("Active", "active")
    .labels("Active Users", "Inactive Users");

// Date range filter
let date_filter = DateRangeFilter::new("Created", "created_at");
```

## Creating Lenses

```rust
use rf_nova::prelude::*;

pub struct MostActiveUsers;

impl Lens for MostActiveUsers {
    fn name(&self) -> &str {
        "Most Active Users"
    }

    fn query(&self) -> LensQuery {
        LensQuery::new()
            .select(vec![
                "users.*".to_string(),
                "COUNT(posts.id) as post_count".to_string(),
            ])
            .join(LensJoin::left("posts", "posts.user_id", "users.id"))
            .group_by(vec!["users.id".to_string()])
            .having(LensHaving::new("COUNT(posts.id)", ">", json!(5)))
            .order_by("post_count".to_string(), OrderDirection::Desc)
            .limit(25)
    }
}
```

## Creating Metrics

### Value Metric
```rust
pub struct TotalUsers;

#[async_trait]
impl ValueMetric for TotalUsers {
    fn name(&self) -> &str {
        "Total Users"
    }

    async fn calculate(&self) -> Result<MetricValue, MetricError> {
        let count = User::count().await? as f64;
        let previous = User::where_date("created_at", "<", last_month).count().await? as f64;

        Ok(MetricValue::new(count)
            .prefix("👥")
            .suffix("users")
            .previous(previous))
    }
}
```

### Trend Metric
```rust
pub struct NewUsersTrend;

#[async_trait]
impl TrendMetric for NewUsersTrend {
    fn name(&self) -> &str {
        "New Users"
    }

    async fn calculate(&self, range: DateRange) -> Result<TrendData, MetricError> {
        TrendData::by_days(range, |date| async move {
            let count = User::where_date("created_at", date)
                .count()
                .await? as f64;
            Ok(count)
        }).await
    }
}
```

### Partition Metric
```rust
pub struct UsersByType;

#[async_trait]
impl PartitionMetric for UsersByType {
    fn name(&self) -> &str {
        "Users by Type"
    }

    async fn calculate(&self) -> Result<PartitionData, MetricError> {
        let admin_count = User::where_eq("is_admin", true).count().await? as f64;
        let user_count = User::where_eq("is_admin", false).count().await? as f64;

        Ok(PartitionData::new()
            .add("Administrators", admin_count, Colors::BLUE)
            .add("Regular Users", user_count, Colors::GREEN))
    }
}
```

## Creating Cards

```rust
// Help card
let welcome = HelpCard::new("Welcome")
    .title("Welcome to the Admin Panel!")
    .content("Use the sidebar to navigate between resources.")
    .width(MetricWidth::Full);

// Table card
let recent_activity = TableCard::new("Recent Activity")
    .columns(vec![
        TableColumn::new("user", "User").sortable(),
        TableColumn::new("action", "Action"),
        TableColumn::new("time", "Time").sortable(),
    ])
    .rows(recent_rows)
    .width(MetricWidth::Full);

// Progress card
let monthly_goal = ProgressCard::new("Monthly Goal", 7500.0, 10000.0)
    .label("Revenue Goal")
    .width(MetricWidth::OneHalf);
```

## Creating Dashboards

```rust
pub struct MainDashboard;

impl Dashboard for MainDashboard {
    fn name(&self) -> &str {
        "Main Dashboard"
    }

    fn value_metrics(&self) -> Vec<Box<dyn ValueMetric>> {
        vec![
            Box::new(TotalUsers),
            Box::new(TotalRevenue),
        ]
    }

    fn trend_metrics(&self) -> Vec<Box<dyn TrendMetric>> {
        vec![
            Box::new(NewUsersTrend),
            Box::new(RevenueTrend),
        ]
    }

    fn partition_metrics(&self) -> Vec<Box<dyn PartitionMetric>> {
        vec![
            Box::new(UsersByType),
            Box::new(OrdersByStatus),
        ]
    }

    fn cards(&self) -> Vec<Box<dyn Card>> {
        vec![
            Box::new(WelcomeCard),
            Box::new(RecentActivity),
        ]
    }
}
```

## Authorization

### Using Preset Policies

```rust
impl Resource for UserResource {
    // ... other methods

    async fn authorize_view_any(user: Option<&Value>) -> bool {
        AdminOnlyPolicy::view_any(user)
    }

    async fn authorize_create(user: Option<&Value>) -> bool {
        AdminOnlyPolicy::create(user)
    }

    async fn authorize_update(user: Option<&Value>, model: &Self::Model) -> bool {
        // Admin or owner can update
        if AdminOnlyPolicy::update(user, &serde_json::to_value(model).unwrap()) {
            return true;
        }
        OwnerPolicy::new().update(user, &serde_json::to_value(model).unwrap())
    }
}
```

### Available Policies

- `AllowAllPolicy` - Permits all actions
- `AdminOnlyPolicy` - Only admins can access
- `ReadOnlyPolicy` - Only view operations allowed
- `OwnerPolicy` - Users can only edit their own records

## API Routes

Nova automatically creates these routes:

```
GET    /admin/api/resources              - List all resources
GET    /admin/api/resources/:resource    - Resource index (paginated)
GET    /admin/api/resources/:resource/:id - Show resource
POST   /admin/api/resources/:resource    - Create resource
PUT    /admin/api/resources/:resource/:id - Update resource
DELETE /admin/api/resources/:resource/:id - Delete resource

POST   /admin/api/resources/:resource/actions/:action - Run action
GET    /admin/api/resources/:resource/filters - Get filters
GET    /admin/api/resources/:resource/lenses/:lens - Get lens data
GET    /admin/api/resources/:resource/export - Export resources

GET    /admin/api/dashboards             - List dashboards
GET    /admin/api/dashboards/:dashboard  - Get dashboard data

GET    /admin/api/metrics/value/:metric      - Get value metric
GET    /admin/api/metrics/trend/:metric      - Get trend metric
GET    /admin/api/metrics/partition/:metric  - Get partition metric

GET    /admin/api/search                 - Global search
GET    /admin/api/config                 - Get Nova config
```

## Configuration

```rust
let nova = Nova::new()
    .with_path("/admin")              // Base path for admin panel
    .with_name("My Admin")            // Application name
    .with_logo("/logo.png")           // Logo URL
    .with_primary_color("#4299E1")    // Primary color (hex)
    .with_theme("dark")               // Theme: light, dark, auto
    .with_db(database_connection);    // Database connection
```

## Examples

See the `examples/` directory for complete examples:

- `basic_admin.rs` - Complete admin panel with all features

Run an example:
```bash
cargo run --example basic_admin
```

## Testing

Run tests:
```bash
cargo test
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please see the main RustForge repository for contribution guidelines.
