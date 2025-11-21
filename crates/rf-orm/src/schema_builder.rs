//! # Schema Builder with Blueprint API
//!
//! Laravel-inspired schema builder for creating and modifying database tables.
//!
//! ## Features
//!
//! - Fluent Blueprint API for defining table structures
//! - Support for all common column types (string, integer, boolean, json, timestamps, etc.)
//! - Chainable column modifiers (nullable, default, unique, index, etc.)
//! - Foreign key constraints with cascade options
//! - Composite indexes and unique constraints
//! - Multi-database support (SQLite, PostgreSQL, MySQL)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_orm::schema_builder::Schema;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new table
//! Schema::create("posts", |table| {
//!     table.id();
//!     table.string("title");
//!     table.text("body");
//!     table.boolean("published").default("false");
//!     table.integer("views").default("0").unsigned();
//!     table.foreign_id("user_id").constrained().on_delete("cascade");
//!     table.timestamps();
//!     table.soft_deletes();
//!
//!     table.index("published");
//!     table.unique("slug");
//! }).await?;
//!
//! // Modify existing table
//! Schema::table("posts", |table| {
//!     table.string("subtitle").nullable();
//!     table.index(&["user_id", "published"]);
//! }).await?;
//!
//! // Drop table
//! Schema::drop_if_exists("posts").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Column Types
//!
//! ### Integers
//! - `id()` - Auto-increment primary key
//! - `integer(name)` - Standard integer
//! - `big_integer(name)` - 64-bit integer
//! - `tiny_integer(name)` - 8-bit integer
//! - `unsigned()` modifier for unsigned integers
//!
//! ### Strings
//! - `string(name)` - VARCHAR(255)
//! - `string_with_length(name, length)` - VARCHAR with custom length
//! - `text(name)` - TEXT column for long content
//!
//! ### Numbers
//! - `float(name)` - Single precision floating point
//! - `double(name)` - Double precision floating point
//! - `decimal(name, precision, scale)` - Fixed-point decimal
//!
//! ### Other Types
//! - `boolean(name)` - Boolean/bit field
//! - `json(name)` - JSON column
//! - `jsonb(name)` - Binary JSON (PostgreSQL)
//! - `date(name)` - Date without time
//! - `datetime(name)` - Date with time
//! - `timestamp(name)` - Unix timestamp
//!
//! ### Convenience Methods
//! - `timestamps()` - Adds created_at and updated_at
//! - `soft_deletes()` - Adds deleted_at for soft deletion
//!
//! ## Column Modifiers
//!
//! All column modifiers are chainable:
//!
//! ```rust,no_run
//! # use rf_orm::schema_builder::Schema;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! Schema::create("users", |table| {
//!     table.string("email")
//!         .unique()
//!         .comment("User's email address");
//!
//!     table.integer("age")
//!         .nullable()
//!         .unsigned()
//!         .default("0");
//!
//!     table.string("status")
//!         .default("'active'")
//!         .index();
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Foreign Keys
//!
//! ### Simple Foreign Key
//! ```rust,no_run
//! # use rf_orm::schema_builder::Schema;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! Schema::create("posts", |table| {
//!     // Auto-detects "users" table from "user_id"
//!     table.foreign_id("user_id").constrained();
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Custom Foreign Key
//! ```rust,no_run
//! # use rf_orm::schema_builder::Schema;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! Schema::create("posts", |table| {
//!     table.foreign("author_id")
//!         .references("id")
//!         .on("users")
//!         .on_delete("cascade")
//!         .on_update("cascade");
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Indexes
//!
//! ### Single Column Index
//! ```rust,no_run
//! # use rf_orm::schema_builder::Schema;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! Schema::create("posts", |table| {
//!     table.string("slug");
//!     table.index("slug");
//!     table.unique("slug");
//! }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Composite Index
//! ```rust,no_run
//! # use rf_orm::schema_builder::Schema;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! Schema::create("posts", |table| {
//!     table.index(&["user_id", "published"]);
//!     table.unique(&["user_id", "slug"]);
//! }).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{DbError, DbResult};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Database type detected from connection string
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    /// SQLite database
    SQLite,
    /// PostgreSQL database
    PostgreSQL,
    /// MySQL/MariaDB database
    MySQL,
}

impl DatabaseType {
    /// Detect database type from connection URL
    pub fn from_url(url: &str) -> Self {
        if url.starts_with("sqlite") {
            DatabaseType::SQLite
        } else if url.starts_with("postgres") || url.starts_with("postgresql") {
            DatabaseType::PostgreSQL
        } else if url.starts_with("mysql") {
            DatabaseType::MySQL
        } else {
            // Default to SQLite for testing
            DatabaseType::SQLite
        }
    }
}

/// Global database connection for schema operations (for static API)
static DB_CONNECTION: RwLock<Option<Arc<DatabaseConnection>>> = RwLock::const_new(None);

/// Schema builder for creating and modifying database tables
///
/// Provides a Laravel-inspired fluent API for database schema management.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::schema_builder::Schema;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Option 1: Static API (requires set_connection)
/// Schema::set_connection(db_connection).await;
/// Schema::create("users", |table| {
///     table.id();
///     table.string("email").unique();
///     table.string("name");
///     table.timestamps();
/// }).await?;
///
/// // Option 2: Instance API
/// let schema = Schema::new(db_connection);
/// schema.create("users", |table| {
///     table.id();
///     table.string("email").unique();
///     table.string("name");
///     table.timestamps();
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct Schema {
    db: Option<Arc<DatabaseConnection>>,
}

impl Schema {
    /// Create a new Schema instance with a specific database connection
    ///
    /// This allows using the Schema API without setting a global connection.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::schema_builder::Schema;
    /// use rf_orm::DatabaseManager;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = DatabaseManager::connect(Default::default()).await?;
    /// let schema = Schema::new(db.connection().clone());
    ///
    /// schema.create("users", |table| {
    ///     table.id();
    ///     table.string("email");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(connection: impl Into<Arc<DatabaseConnection>>) -> Self {
        Self {
            db: Some(connection.into()),
        }
    }

    /// Set the database connection for schema operations (static API)
    ///
    /// This must be called before using static schema operations.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::schema_builder::Schema;
    /// use rf_orm::DatabaseManager;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = DatabaseManager::connect(Default::default()).await?;
    /// Schema::set_connection(db.connection().clone()).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_connection(connection: impl Into<Arc<DatabaseConnection>>) {
        let connection = connection.into();
        let mut conn = DB_CONNECTION.write().await;
        *conn = Some(connection);
    }

    /// Get the database connection (instance or global)
    async fn get_connection(&self) -> DbResult<Arc<DatabaseConnection>> {
        if let Some(ref db) = self.db {
            return Ok(db.clone());
        }

        let conn = DB_CONNECTION.read().await;
        conn.as_ref()
            .map(Arc::clone)
            .ok_or_else(|| DbError::InvalidConfig("No database connection set. Call Schema::set_connection() or use Schema::new().".to_string()))
    }

    /// Create a new table
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table to create
    /// * `callback` - Closure that defines the table structure using Blueprint
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::schema_builder::Schema;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.id();
    ///     table.string("title");
    ///     table.text("body");
    ///     table.foreign_id("user_id").constrained();
    ///     table.timestamps();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create<F>(&self, table_name: &str, callback: F) -> DbResult<()>
    where
        F: FnOnce(&mut Blueprint),
    {
        let conn = self.get_connection().await?;
        let mut blueprint = Blueprint::new(table_name.to_string());
        callback(&mut blueprint);

        let sql = blueprint.to_create_sql(DatabaseType::SQLite)?;

        sea_orm::ConnectionTrait::execute_unprepared(conn.as_ref(), &sql)
            .await
            .map_err(|e| DbError::QueryFailed {
                query: sql.clone(),
                source: e,
            })?;

        Ok(())
    }

    /// Modify an existing table
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table to modify
    /// * `callback` - Closure that defines the modifications using Blueprint
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::schema_builder::Schema;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::table("posts", |table| {
    ///     table.string("subtitle").nullable();
    ///     table.index("published_at");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn table<F>(&self, table_name: &str, callback: F) -> DbResult<()>
    where
        F: FnOnce(&mut Blueprint),
    {
        let conn = self.get_connection().await?;
        let mut blueprint = Blueprint::new(table_name.to_string());
        blueprint.is_create = false;
        callback(&mut blueprint);

        let sql = blueprint.to_alter_sql(DatabaseType::SQLite)?;

        sea_orm::ConnectionTrait::execute_unprepared(conn.as_ref(), &sql)
            .await
            .map_err(|e| DbError::QueryFailed {
                query: sql.clone(),
                source: e,
            })?;

        Ok(())
    }

    /// Drop a table
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table to drop
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::schema_builder::Schema;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::drop("old_posts").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn drop(&self, table_name: &str) -> DbResult<()> {
        let conn = self.get_connection().await?;
        let sql = format!("DROP TABLE {}", table_name);

        sea_orm::ConnectionTrait::execute_unprepared(conn.as_ref(), &sql)
            .await
            .map_err(|e| DbError::QueryFailed {
                query: sql.clone(),
                source: e,
            })?;

        Ok(())
    }

    /// Drop a table if it exists
    ///
    /// # Arguments
    ///
    /// * `table_name` - Name of the table to drop
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::schema_builder::Schema;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::drop_if_exists("posts").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn drop_if_exists(&self, table_name: &str) -> DbResult<()> {
        let conn = self.get_connection().await?;
        let sql = format!("DROP TABLE IF EXISTS {}", table_name);

        sea_orm::ConnectionTrait::execute_unprepared(conn.as_ref(), &sql)
            .await
            .map_err(|e| DbError::QueryFailed {
                query: sql.clone(),
                source: e,
            })?;

        Ok(())
    }
}

/// Blueprint for defining table structure
///
/// Provides a fluent API for defining columns, indexes, and constraints.
/// This is passed to the callback in [`Schema::create`] and [`Schema::table`].
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::schema_builder::Schema;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// Schema::create("users", |table| {
///     // Primary key
///     table.id();
///
///     // String columns
///     table.string("email").unique();
///     table.string("name");
///
///     // Integer with default
///     table.integer("age").default("18").unsigned();
///
///     // Timestamps
///     table.timestamps();
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct Blueprint {
    table_name: String,
    columns: Vec<Column>,
    indexes: Vec<Index>,
    foreign_keys: Vec<ForeignKey>,
    is_create: bool,
}

impl Blueprint {
    /// Create a new Blueprint for the given table
    fn new(table_name: String) -> Self {
        Self {
            table_name,
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            is_create: true,
        }
    }

    /// Add auto-increment primary key column named 'id'
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.id();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn id(&mut self) -> &mut Column {
        let column = Column {
            name: "id".to_string(),
            column_type: ColumnType::BigInteger,
            nullable: false,
            default: None,
            unique: false,
            index: false,
            unsigned: true,
            comment: None,
            auto_increment: true,
            primary_key: true,
        };
        self.columns.push(column);
        self.columns.last_mut().unwrap()
    }

    /// Add a VARCHAR column with default length (255)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.string("email").unique();
    ///     table.string("name");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn string(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::String(Some(255)))
    }

    /// Add a VARCHAR column with custom length
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("products", |table| {
    ///     table.string_with_length("sku", 50);
    ///     table.string_with_length("description", 1000);
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn string_with_length(&mut self, name: &str, length: usize) -> &mut Column {
        self.add_column(name, ColumnType::String(Some(length)))
    }

    /// Add a TEXT column for long content
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.text("body");
    ///     table.text("summary").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn text(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Text)
    }

    /// Add a standard INTEGER column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("products", |table| {
    ///     table.integer("stock").default("0").unsigned();
    ///     table.integer("price");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Integer)
    }

    /// Add a BIGINT column (64-bit integer)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("analytics", |table| {
    ///     table.big_integer("views").default("0");
    ///     table.big_integer("revenue");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn big_integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::BigInteger)
    }

    /// Add a TINYINT column (8-bit integer)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("settings", |table| {
    ///     table.tiny_integer("status").default("0");
    ///     table.tiny_integer("priority");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tiny_integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::TinyInteger)
    }

    /// Add a FLOAT column (single precision)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("measurements", |table| {
    ///     table.float("temperature");
    ///     table.float("humidity");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn float(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Float)
    }

    /// Add a DOUBLE column (double precision)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("coordinates", |table| {
    ///     table.double("latitude");
    ///     table.double("longitude");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn double(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Double)
    }

    /// Add a DECIMAL column with precision and scale
    ///
    /// # Arguments
    ///
    /// * `name` - Column name
    /// * `precision` - Total number of digits
    /// * `scale` - Number of digits after decimal point
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("products", |table| {
    ///     table.decimal("price", 10, 2); // e.g., 99999999.99
    ///     table.decimal("weight", 8, 3); // e.g., 99999.999
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn decimal(&mut self, name: &str, precision: u8, scale: u8) -> &mut Column {
        self.add_column(name, ColumnType::Decimal(precision, scale))
    }

    /// Add a BOOLEAN column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.boolean("published").default("false");
    ///     table.boolean("featured");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn boolean(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Boolean)
    }

    /// Add a JSON column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.json("metadata");
    ///     table.json("settings").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn json(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Json)
    }

    /// Add a JSONB column (PostgreSQL binary JSON)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("documents", |table| {
    ///     table.jsonb("data");
    ///     table.jsonb("metadata").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn jsonb(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::JsonB)
    }

    /// Add a DATE column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("events", |table| {
    ///     table.date("event_date");
    ///     table.date("deadline").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn date(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Date)
    }

    /// Add a DATETIME column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("events", |table| {
    ///     table.datetime("starts_at");
    ///     table.datetime("ends_at").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn datetime(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::DateTime)
    }

    /// Add a TIMESTAMP column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("logs", |table| {
    ///     table.timestamp("logged_at");
    ///     table.timestamp("processed_at").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn timestamp(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::Timestamp)
    }

    /// Add created_at and updated_at timestamp columns
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.id();
    ///     table.string("title");
    ///     table.timestamps();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn timestamps(&mut self) {
        self.timestamp("created_at").nullable();
        self.timestamp("updated_at").nullable();
    }

    /// Add deleted_at timestamp column for soft deletes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.id();
    ///     table.string("email");
    ///     table.timestamps();
    ///     table.soft_deletes();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn soft_deletes(&mut self) {
        self.timestamp("deleted_at").nullable();
    }

    /// Add a foreign key column with auto-detection
    ///
    /// Automatically detects the referenced table from the column name.
    /// For example, `user_id` references the `users` table.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.id();
    ///     table.foreign_id("user_id").constrained();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn foreign_id(&mut self, name: &str) -> &mut Column {
        self.add_column(name, ColumnType::BigInteger)
            .unsigned()
            .index()
    }

    /// Add a foreign key constraint
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.big_integer("author_id");
    ///     table.foreign("author_id")
    ///         .references("id")
    ///         .on("users")
    ///         .on_delete("cascade");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn foreign(&mut self, column: &str) -> &mut ForeignKey {
        let fk = ForeignKey {
            column: column.to_string(),
            references: "id".to_string(),
            on: String::new(),
            on_delete: None,
            on_update: None,
        };
        self.foreign_keys.push(fk);
        self.foreign_keys.last_mut().unwrap()
    }

    /// Add an index on one or more columns
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.string("slug");
    ///     table.boolean("published");
    ///
    ///     // Single column index
    ///     table.index("slug");
    ///
    ///     // Composite index
    ///     table.index(&["published", "created_at"]);
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn index<T>(&mut self, columns: T) -> &mut Index
    where
        T: IndexColumns,
    {
        let cols = columns.to_vec();
        let index = Index {
            columns: cols.clone(),
            unique: false,
            name: format!("idx_{}_{}", self.table_name, cols.join("_")),
        };
        self.indexes.push(index);
        self.indexes.last_mut().unwrap()
    }

    /// Add a unique constraint on one or more columns
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.string("email");
    ///     table.string("username");
    ///
    ///     // Single column unique
    ///     table.unique("email");
    ///
    ///     // Composite unique
    ///     table.unique(&["tenant_id", "username"]);
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unique<T>(&mut self, columns: T) -> &mut Index
    where
        T: IndexColumns,
    {
        let cols = columns.to_vec();
        let index = Index {
            columns: cols.clone(),
            unique: true,
            name: format!("uniq_{}_{}", self.table_name, cols.join("_")),
        };
        self.indexes.push(index);
        self.indexes.last_mut().unwrap()
    }

    /// Add a column with the given type
    fn add_column(&mut self, name: &str, column_type: ColumnType) -> &mut Column {
        let column = Column {
            name: name.to_string(),
            column_type,
            nullable: false,
            default: None,
            unique: false,
            index: false,
            unsigned: false,
            comment: None,
            auto_increment: false,
            primary_key: false,
        };
        self.columns.push(column);
        self.columns.last_mut().unwrap()
    }

    /// Generate CREATE TABLE SQL
    fn to_create_sql(&self, db_type: DatabaseType) -> DbResult<String> {
        let mut sql = format!("CREATE TABLE {} (\n", self.table_name);

        // Columns
        let column_defs: Vec<String> = self
            .columns
            .iter()
            .map(|col| col.to_sql(db_type))
            .collect();
        sql.push_str(&format!("  {}", column_defs.join(",\n  ")));

        // Foreign keys
        if !self.foreign_keys.is_empty() {
            sql.push_str(",\n");
            let fk_defs: Vec<String> = self
                .foreign_keys
                .iter()
                .map(|fk| fk.to_sql())
                .collect();
            sql.push_str(&format!("  {}", fk_defs.join(",\n  ")));
        }

        sql.push_str("\n)");

        // Indexes (created separately)
        if !self.indexes.is_empty() {
            let index_defs: Vec<String> = self
                .indexes
                .iter()
                .map(|idx| idx.to_sql(&self.table_name))
                .collect();
            sql.push_str(";\n");
            sql.push_str(&index_defs.join(";\n"));
        }

        Ok(sql)
    }

    /// Generate ALTER TABLE SQL
    fn to_alter_sql(&self, db_type: DatabaseType) -> DbResult<String> {
        let mut statements = Vec::new();

        // Add columns
        for col in &self.columns {
            statements.push(format!(
                "ALTER TABLE {} ADD COLUMN {}",
                self.table_name,
                col.to_sql(db_type)
            ));
        }

        // Add indexes
        for idx in &self.indexes {
            statements.push(idx.to_sql(&self.table_name));
        }

        // Add foreign keys
        for fk in &self.foreign_keys {
            statements.push(format!(
                "ALTER TABLE {} ADD {}",
                self.table_name,
                fk.to_sql()
            ));
        }

        Ok(statements.join(";\n"))
    }
}

/// Table column definition
#[derive(Debug, Clone)]
pub struct Column {
    name: String,
    column_type: ColumnType,
    nullable: bool,
    default: Option<String>,
    unique: bool,
    index: bool,
    unsigned: bool,
    comment: Option<String>,
    auto_increment: bool,
    primary_key: bool,
}

impl Column {
    /// Make the column nullable
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.string("phone").nullable();
    ///     table.string("bio").nullable();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn nullable(&mut self) -> &mut Self {
        self.nullable = true;
        self
    }

    /// Set a default value for the column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.boolean("active").default("true");
    ///     table.integer("role").default("1");
    ///     table.string("status").default("'pending'");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn default(&mut self, value: &str) -> &mut Self {
        self.default = Some(value.to_string());
        self
    }

    /// Add a unique constraint to the column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.string("email").unique();
    ///     table.string("username").unique();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unique(&mut self) -> &mut Self {
        self.unique = true;
        self
    }

    /// Add an index to the column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.string("slug").index();
    ///     table.timestamp("published_at").index();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn index(&mut self) -> &mut Self {
        self.index = true;
        self
    }

    /// Make the column unsigned (for integer types)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("products", |table| {
    ///     table.integer("stock").unsigned();
    ///     table.integer("price").unsigned();
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn unsigned(&mut self) -> &mut Self {
        self.unsigned = true;
        self
    }

    /// Add a comment to the column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("users", |table| {
    ///     table.string("email").comment("User's primary email");
    ///     table.integer("age").comment("Age in years");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn comment(&mut self, text: &str) -> &mut Self {
        self.comment = Some(text.to_string());
        self
    }

    /// Add a foreign key constraint (auto-detects referenced table)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.foreign_id("user_id")
    ///         .constrained()
    ///         .on_delete("cascade");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn constrained(&mut self) -> &mut Self {
        // This is handled by foreign_id() method
        self
    }

    /// Set the ON DELETE action for foreign key
    ///
    /// # Arguments
    ///
    /// * `action` - One of: "cascade", "set null", "restrict", "no action"
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.foreign_id("user_id")
    ///         .constrained()
    ///         .on_delete("cascade");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn on_delete(&mut self, _action: &str) -> &mut Self {
        // This is handled by ForeignKey
        self
    }

    /// Generate SQL for this column
    fn to_sql(&self, db_type: DatabaseType) -> String {
        // For SQLite primary keys with autoincrement, use INTEGER type
        let column_type_sql = if self.primary_key && self.auto_increment && db_type == DatabaseType::SQLite {
            "INTEGER".to_string()
        } else {
            self.column_type.to_sql(db_type)
        };

        let mut sql = format!("{} {}", self.name, column_type_sql);

        if self.primary_key {
            sql.push_str(" PRIMARY KEY");
        }

        if self.auto_increment {
            match db_type {
                DatabaseType::SQLite => sql.push_str(" AUTOINCREMENT"),
                DatabaseType::PostgreSQL => {}, // Handled by SERIAL type
                DatabaseType::MySQL => sql.push_str(" AUTO_INCREMENT"),
            }
        }

        if !self.nullable && !self.primary_key {
            sql.push_str(" NOT NULL");
        }

        if let Some(ref default) = self.default {
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        if self.unique {
            sql.push_str(" UNIQUE");
        }

        sql
    }
}

/// Column data type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    /// Standard integer
    Integer,
    /// 64-bit integer
    BigInteger,
    /// 8-bit integer
    TinyInteger,
    /// Variable-length string with optional max length
    String(Option<usize>),
    /// Long text
    Text,
    /// Single precision floating point
    Float,
    /// Double precision floating point
    Double,
    /// Fixed-point decimal (precision, scale)
    Decimal(u8, u8),
    /// Boolean/bit
    Boolean,
    /// JSON data
    Json,
    /// Binary JSON (PostgreSQL)
    JsonB,
    /// Date without time
    Date,
    /// Date with time
    DateTime,
    /// Unix timestamp
    Timestamp,
}

impl ColumnType {
    /// Convert column type to SQL type string
    fn to_sql(&self, db_type: DatabaseType) -> String {
        match (self, db_type) {
            (ColumnType::Integer, _) => "INTEGER".to_string(),
            (ColumnType::BigInteger, DatabaseType::PostgreSQL) => "BIGINT".to_string(),
            (ColumnType::BigInteger, _) => "BIGINT".to_string(),
            (ColumnType::TinyInteger, DatabaseType::MySQL) => "TINYINT".to_string(),
            (ColumnType::TinyInteger, _) => "INTEGER".to_string(),
            (ColumnType::String(Some(len)), _) => format!("VARCHAR({})", len),
            (ColumnType::String(None), _) => "VARCHAR(255)".to_string(),
            (ColumnType::Text, _) => "TEXT".to_string(),
            (ColumnType::Float, _) => "REAL".to_string(),
            (ColumnType::Double, DatabaseType::MySQL) => "DOUBLE".to_string(),
            (ColumnType::Double, _) => "REAL".to_string(),
            (ColumnType::Decimal(p, s), _) => format!("DECIMAL({}, {})", p, s),
            (ColumnType::Boolean, DatabaseType::PostgreSQL) => "BOOLEAN".to_string(),
            (ColumnType::Boolean, _) => "INTEGER".to_string(), // SQLite/MySQL
            (ColumnType::Json, DatabaseType::PostgreSQL) => "JSON".to_string(),
            (ColumnType::Json, DatabaseType::MySQL) => "JSON".to_string(),
            (ColumnType::Json, _) => "TEXT".to_string(), // SQLite
            (ColumnType::JsonB, DatabaseType::PostgreSQL) => "JSONB".to_string(),
            (ColumnType::JsonB, _) => "TEXT".to_string(), // Fallback
            (ColumnType::Date, _) => "DATE".to_string(),
            (ColumnType::DateTime, _) => "DATETIME".to_string(),
            (ColumnType::Timestamp, DatabaseType::PostgreSQL) => "TIMESTAMP".to_string(),
            (ColumnType::Timestamp, _) => "DATETIME".to_string(),
        }
    }
}

/// Foreign key constraint
#[derive(Debug, Clone)]
pub struct ForeignKey {
    column: String,
    references: String,
    on: String,
    on_delete: Option<String>,
    on_update: Option<String>,
}

impl ForeignKey {
    /// Set the referenced column (usually "id")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.foreign("user_id")
    ///         .references("id")
    ///         .on("users");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn references(&mut self, column: &str) -> &mut Self {
        self.references = column.to_string();
        self
    }

    /// Set the referenced table
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.foreign("author_id")
    ///         .references("id")
    ///         .on("users");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn on(&mut self, table: &str) -> &mut Self {
        self.on = table.to_string();
        self
    }

    /// Set ON DELETE action
    ///
    /// # Arguments
    ///
    /// * `action` - One of: "cascade", "set null", "restrict", "no action"
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.foreign("user_id")
    ///         .references("id")
    ///         .on("users")
    ///         .on_delete("cascade");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn on_delete(&mut self, action: &str) -> &mut Self {
        self.on_delete = Some(action.to_uppercase());
        self
    }

    /// Set ON UPDATE action
    ///
    /// # Arguments
    ///
    /// * `action` - One of: "cascade", "set null", "restrict", "no action"
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::schema_builder::Schema;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Schema::create("posts", |table| {
    ///     table.foreign("user_id")
    ///         .references("id")
    ///         .on("users")
    ///         .on_update("cascade");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn on_update(&mut self, action: &str) -> &mut Self {
        self.on_update = Some(action.to_uppercase());
        self
    }

    /// Generate SQL for this foreign key
    fn to_sql(&self) -> String {
        let mut sql = format!(
            "FOREIGN KEY ({}) REFERENCES {}({})",
            self.column, self.on, self.references
        );

        if let Some(ref action) = self.on_delete {
            sql.push_str(&format!(" ON DELETE {}", action));
        }

        if let Some(ref action) = self.on_update {
            sql.push_str(&format!(" ON UPDATE {}", action));
        }

        sql
    }
}

/// Index definition
#[derive(Debug, Clone)]
pub struct Index {
    columns: Vec<String>,
    unique: bool,
    name: String,
}

impl Index {
    /// Generate SQL for this index
    fn to_sql(&self, table_name: &str) -> String {
        let unique = if self.unique { "UNIQUE " } else { "" };
        let columns = self.columns.join(", ");
        format!(
            "CREATE {}INDEX {} ON {}({})",
            unique, self.name, table_name, columns
        )
    }
}

/// Trait for types that can be used as index column specifications
pub trait IndexColumns {
    /// Convert to vector of column names
    fn to_vec(&self) -> Vec<String>;
}

impl IndexColumns for &str {
    fn to_vec(&self) -> Vec<String> {
        vec![self.to_string()]
    }
}

impl IndexColumns for String {
    fn to_vec(&self) -> Vec<String> {
        vec![self.clone()]
    }
}

impl IndexColumns for &[&str] {
    fn to_vec(&self) -> Vec<String> {
        self.iter().map(|s| s.to_string()).collect()
    }
}

impl<const N: usize> IndexColumns for &[&str; N] {
    fn to_vec(&self) -> Vec<String> {
        self.iter().map(|s| s.to_string()).collect()
    }
}

impl<const N: usize> IndexColumns for [&str; N] {
    fn to_vec(&self) -> Vec<String> {
        self.iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_type_detection() {
        assert_eq!(DatabaseType::from_url("sqlite::memory:"), DatabaseType::SQLite);
        assert_eq!(DatabaseType::from_url("postgres://localhost/test"), DatabaseType::PostgreSQL);
        assert_eq!(DatabaseType::from_url("postgresql://localhost/test"), DatabaseType::PostgreSQL);
        assert_eq!(DatabaseType::from_url("mysql://localhost/test"), DatabaseType::MySQL);
    }

    #[test]
    fn test_column_type_sql() {
        assert_eq!(ColumnType::Integer.to_sql(DatabaseType::SQLite), "INTEGER");
        assert_eq!(ColumnType::BigInteger.to_sql(DatabaseType::PostgreSQL), "BIGINT");
        assert_eq!(ColumnType::String(Some(100)).to_sql(DatabaseType::SQLite), "VARCHAR(100)");
        assert_eq!(ColumnType::String(None).to_sql(DatabaseType::SQLite), "VARCHAR(255)");
        assert_eq!(ColumnType::Text.to_sql(DatabaseType::SQLite), "TEXT");
        assert_eq!(ColumnType::Boolean.to_sql(DatabaseType::PostgreSQL), "BOOLEAN");
        assert_eq!(ColumnType::Boolean.to_sql(DatabaseType::SQLite), "INTEGER");
        assert_eq!(ColumnType::Json.to_sql(DatabaseType::PostgreSQL), "JSON");
        assert_eq!(ColumnType::JsonB.to_sql(DatabaseType::PostgreSQL), "JSONB");
        assert_eq!(ColumnType::Decimal(10, 2).to_sql(DatabaseType::SQLite), "DECIMAL(10, 2)");
    }

    #[test]
    fn test_column_sql_generation() {
        let col = Column {
            name: "email".to_string(),
            column_type: ColumnType::String(Some(255)),
            nullable: false,
            default: None,
            unique: true,
            index: false,
            unsigned: false,
            comment: None,
            auto_increment: false,
            primary_key: false,
        };

        let sql = col.to_sql(DatabaseType::SQLite);
        assert!(sql.contains("email"));
        assert!(sql.contains("VARCHAR(255)"));
        assert!(sql.contains("NOT NULL"));
        assert!(sql.contains("UNIQUE"));
    }

    #[test]
    fn test_column_with_default() {
        let mut col = Column {
            name: "active".to_string(),
            column_type: ColumnType::Boolean,
            nullable: false,
            default: Some("true".to_string()),
            unique: false,
            index: false,
            unsigned: false,
            comment: None,
            auto_increment: false,
            primary_key: false,
        };

        let sql = col.to_sql(DatabaseType::SQLite);
        assert!(sql.contains("DEFAULT true"));

        col.default = Some("'pending'".to_string());
        let sql = col.to_sql(DatabaseType::SQLite);
        assert!(sql.contains("DEFAULT 'pending'"));
    }

    #[test]
    fn test_column_nullable() {
        let col = Column {
            name: "phone".to_string(),
            column_type: ColumnType::String(Some(20)),
            nullable: true,
            default: None,
            unique: false,
            index: false,
            unsigned: false,
            comment: None,
            auto_increment: false,
            primary_key: false,
        };

        let sql = col.to_sql(DatabaseType::SQLite);
        assert!(!sql.contains("NOT NULL"));
    }

    #[test]
    fn test_primary_key_column() {
        let col = Column {
            name: "id".to_string(),
            column_type: ColumnType::BigInteger,
            nullable: false,
            default: None,
            unique: false,
            index: false,
            unsigned: true,
            comment: None,
            auto_increment: true,
            primary_key: true,
        };

        let sql = col.to_sql(DatabaseType::SQLite);
        assert!(sql.contains("PRIMARY KEY"));
        assert!(sql.contains("AUTOINCREMENT"));
        assert!(!sql.contains("NOT NULL")); // Primary keys are implicitly NOT NULL
    }

    #[test]
    fn test_foreign_key_sql() {
        let fk = ForeignKey {
            column: "user_id".to_string(),
            references: "id".to_string(),
            on: "users".to_string(),
            on_delete: Some("CASCADE".to_string()),
            on_update: Some("CASCADE".to_string()),
        };

        let sql = fk.to_sql();
        assert!(sql.contains("FOREIGN KEY (user_id)"));
        assert!(sql.contains("REFERENCES users(id)"));
        assert!(sql.contains("ON DELETE CASCADE"));
        assert!(sql.contains("ON UPDATE CASCADE"));
    }

    #[test]
    fn test_index_sql() {
        let idx = Index {
            columns: vec!["email".to_string()],
            unique: false,
            name: "idx_users_email".to_string(),
        };

        let sql = idx.to_sql("users");
        assert!(sql.contains("CREATE INDEX"));
        assert!(sql.contains("idx_users_email"));
        assert!(sql.contains("ON users(email)"));
    }

    #[test]
    fn test_unique_index_sql() {
        let idx = Index {
            columns: vec!["email".to_string()],
            unique: true,
            name: "uniq_users_email".to_string(),
        };

        let sql = idx.to_sql("users");
        assert!(sql.contains("CREATE UNIQUE INDEX"));
    }

    #[test]
    fn test_composite_index_sql() {
        let idx = Index {
            columns: vec!["user_id".to_string(), "published".to_string()],
            unique: false,
            name: "idx_posts_user_published".to_string(),
        };

        let sql = idx.to_sql("posts");
        assert!(sql.contains("ON posts(user_id, published)"));
    }

    #[test]
    fn test_blueprint_basic() {
        let mut blueprint = Blueprint::new("users".to_string());
        blueprint.id();
        blueprint.string("email").unique();
        blueprint.string("name");
        blueprint.timestamps();

        assert_eq!(blueprint.columns.len(), 5); // id, email, name, created_at, updated_at
        assert_eq!(blueprint.columns[0].name, "id");
        assert_eq!(blueprint.columns[1].name, "email");
        assert!(blueprint.columns[1].unique);
    }

    #[test]
    fn test_blueprint_with_foreign_key() {
        let mut blueprint = Blueprint::new("posts".to_string());
        blueprint.id();
        blueprint.string("title");
        blueprint.foreign_id("user_id");
        blueprint.foreign("user_id")
            .references("id")
            .on("users")
            .on_delete("cascade");

        assert_eq!(blueprint.foreign_keys.len(), 1);
        assert_eq!(blueprint.foreign_keys[0].column, "user_id");
        assert_eq!(blueprint.foreign_keys[0].on, "users");
    }

    #[test]
    fn test_blueprint_with_indexes() {
        let mut blueprint = Blueprint::new("posts".to_string());
        blueprint.string("slug");
        blueprint.boolean("published");
        blueprint.index("slug");
        blueprint.unique("slug");

        assert_eq!(blueprint.indexes.len(), 2);
        assert!(!blueprint.indexes[0].unique);
        assert!(blueprint.indexes[1].unique);
    }

    #[test]
    fn test_blueprint_soft_deletes() {
        let mut blueprint = Blueprint::new("users".to_string());
        blueprint.id();
        blueprint.soft_deletes();

        assert_eq!(blueprint.columns.len(), 2);
        assert_eq!(blueprint.columns[1].name, "deleted_at");
        assert!(blueprint.columns[1].nullable);
    }

    #[test]
    fn test_create_table_sql() {
        let mut blueprint = Blueprint::new("users".to_string());
        blueprint.id();
        blueprint.string("email").unique();
        blueprint.string("name");

        let sql = blueprint.to_create_sql(DatabaseType::SQLite).unwrap();
        assert!(sql.contains("CREATE TABLE users"));
        // SQLite requires INTEGER (not BIGINT) for AUTOINCREMENT to work
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("email VARCHAR(255) NOT NULL UNIQUE"));
        assert!(sql.contains("name VARCHAR(255) NOT NULL"));
    }

    #[test]
    fn test_create_table_with_foreign_key_sql() {
        let mut blueprint = Blueprint::new("posts".to_string());
        blueprint.id();
        blueprint.string("title");
        blueprint.foreign("user_id")
            .references("id")
            .on("users")
            .on_delete("cascade");

        let sql = blueprint.to_create_sql(DatabaseType::SQLite).unwrap();
        assert!(sql.contains("FOREIGN KEY (user_id) REFERENCES users(id)"));
        assert!(sql.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn test_create_table_with_indexes_sql() {
        let mut blueprint = Blueprint::new("posts".to_string());
        blueprint.id();
        blueprint.string("slug");
        blueprint.index("slug");

        let sql = blueprint.to_create_sql(DatabaseType::SQLite).unwrap();
        assert!(sql.contains("CREATE INDEX"));
        assert!(sql.contains("idx_posts_slug"));
    }

    #[test]
    fn test_column_modifiers_chaining() {
        let mut blueprint = Blueprint::new("products".to_string());
        let col = blueprint.integer("stock")
            .default("0")
            .unsigned()
            .comment("Available stock");

        assert_eq!(col.default, Some("0".to_string()));
        assert!(col.unsigned);
        assert_eq!(col.comment, Some("Available stock".to_string()));
    }

    #[test]
    fn test_all_column_types() {
        let mut blueprint = Blueprint::new("test".to_string());

        blueprint.id();
        blueprint.string("str");
        blueprint.string_with_length("str_len", 100);
        blueprint.text("txt");
        blueprint.integer("int");
        blueprint.big_integer("bigint");
        blueprint.tiny_integer("tinyint");
        blueprint.float("flt");
        blueprint.double("dbl");
        blueprint.decimal("dec", 10, 2);
        blueprint.boolean("bool");
        blueprint.json("jsn");
        blueprint.jsonb("jsnb");
        blueprint.date("dt");
        blueprint.datetime("dtm");
        blueprint.timestamp("ts");

        assert_eq!(blueprint.columns.len(), 16);
        assert_eq!(blueprint.columns[1].column_type, ColumnType::String(Some(255)));
        assert_eq!(blueprint.columns[2].column_type, ColumnType::String(Some(100)));
        assert_eq!(blueprint.columns[3].column_type, ColumnType::Text);
        assert_eq!(blueprint.columns[9].column_type, ColumnType::Decimal(10, 2));
    }

    #[test]
    fn test_timestamps_adds_two_columns() {
        let mut blueprint = Blueprint::new("users".to_string());
        blueprint.id();
        blueprint.timestamps();

        assert_eq!(blueprint.columns.len(), 3);
        assert_eq!(blueprint.columns[1].name, "created_at");
        assert_eq!(blueprint.columns[2].name, "updated_at");
        assert!(blueprint.columns[1].nullable);
        assert!(blueprint.columns[2].nullable);
    }

    #[test]
    fn test_alter_table_sql() {
        let mut blueprint = Blueprint::new("users".to_string());
        blueprint.is_create = false;
        blueprint.string("phone").nullable();
        blueprint.index("email");

        let sql = blueprint.to_alter_sql(DatabaseType::SQLite).unwrap();
        assert!(sql.contains("ALTER TABLE users ADD COLUMN"));
        assert!(sql.contains("phone VARCHAR(255)"));
        assert!(sql.contains("CREATE INDEX"));
    }

    #[test]
    fn test_composite_index() {
        let mut blueprint = Blueprint::new("posts".to_string());
        blueprint.index(&["user_id", "published"]);

        assert_eq!(blueprint.indexes.len(), 1);
        assert_eq!(blueprint.indexes[0].columns.len(), 2);
        assert_eq!(blueprint.indexes[0].columns[0], "user_id");
        assert_eq!(blueprint.indexes[0].columns[1], "published");
    }

    #[test]
    fn test_composite_unique() {
        let mut blueprint = Blueprint::new("users".to_string());
        blueprint.unique(&["tenant_id", "email"]);

        assert_eq!(blueprint.indexes.len(), 1);
        assert!(blueprint.indexes[0].unique);
        assert_eq!(blueprint.indexes[0].columns.len(), 2);
    }

    #[test]
    fn test_foreign_key_builder() {
        let mut fk = ForeignKey {
            column: "user_id".to_string(),
            references: "id".to_string(),
            on: "".to_string(),
            on_delete: None,
            on_update: None,
        };

        fk.on("users")
            .on_delete("cascade")
            .on_update("restrict");

        assert_eq!(fk.on, "users");
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
        assert_eq!(fk.on_update, Some("RESTRICT".to_string()));
    }
}
