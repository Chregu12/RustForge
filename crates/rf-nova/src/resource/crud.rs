//! CRUD operations for Nova resources
//!
//! Provides helper functions for common database operations.

use super::resource::{PaginatedResponse, PaginationMeta, ResourceError, ResourceQuery, ResourceResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IdenStatic, IntoActiveModel,
    Iterable, ModelTrait, PaginatorTrait, PrimaryKeyToColumn, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Fetch paginated list of resources
pub async fn index<E>(
    db: &DatabaseConnection,
    query: ResourceQuery,
    searchable_fields: Vec<String>,
) -> ResourceResult<PaginatedResponse<E::Model>>
where
    E: EntityTrait,
    E::Model: Serialize + Send + Sync,
{
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(15);

    let mut select = E::find();

    // Apply search if provided
    if let Some(search_term) = &query.search {
        if !search_term.is_empty() && !searchable_fields.is_empty() {
            // Build OR conditions for searchable fields
            // Note: This is a simplified version. In production, you'd want to use
            // the actual column types and build proper LIKE queries
            select = select.filter(
                sea_orm::Condition::any()
                    // We'll add conditions dynamically based on the entity's columns
            );
        }
    }

    // Apply sorting
    if let Some(sort_field) = &query.sort_by {
        // Note: Sorting would need to be implemented based on the entity's columns
        // This is a placeholder for the actual implementation
    }

    // Apply filters
    if let Some(filters) = &query.filters {
        // Apply custom filters
        // This would be implemented based on the specific filter types
    }

    // Get total count
    let total = select.clone().count(db).await?;

    // Apply pagination
    let paginator = select.paginate(db, per_page);
    let data = paginator.fetch_page(page - 1).await?;

    let meta = PaginationMeta::new(page, per_page, total);

    Ok(PaginatedResponse { data, meta })
}

/// Show a single resource by ID
/// Note: This is a placeholder - in production you would implement actual DB queries
pub async fn show<E>(db: &DatabaseConnection, id: Value) -> ResourceResult<Value>
where
    E: EntityTrait,
{
    // This is a simplified placeholder
    // In production, you'd implement actual DB query using the entity
    Err(ResourceError::NotFound("Not implemented".to_string()))
}

/// Create a new resource.
///
/// Builds an `E::ActiveModel` from the provided `data` map by resolving each
/// key to a real entity column (via `IdenStatic::as_str`), coercing the
/// incoming JSON value into a `sea_orm::Value` matching that column's type
/// (the same coercion machinery `bulk_destroy` uses for primary keys), and
/// issuing a real `INSERT`. Returns the freshly-persisted row as JSON,
/// including its real (database-assigned) primary key.
///
/// Keys that do not correspond to any entity column are ignored (Laravel-style
/// mass-assignment); type-incompatible values return an `InvalidInput` error.
pub async fn create<E>(
    db: &DatabaseConnection,
    data: HashMap<String, Value>,
) -> ResourceResult<Value>
where
    E: EntityTrait,
    E::Model: Serialize + IntoActiveModel<E::ActiveModel>,
{
    let mut active = <E::ActiveModel as ActiveModelTrait>::default();

    for (key, json_value) in &data {
        // Resolve the incoming key to a real column on the entity.
        if let Some(column) = <E::Column as Iterable>::iter().find(|c| c.as_str() == key) {
            let col_type = column.def().get_column_type().clone();
            let value = json_to_column_value(json_value, &col_type)?;
            active.set(column, value);
        }
        // Unknown keys are silently ignored (mass-assignment semantics).
    }

    // Insert and read the row back so the returned JSON reflects the real,
    // database-assigned values (auto-increment ids, defaults, ...).
    let model = E::insert(active).exec_with_returning(db).await?;
    Ok(serde_json::to_value(&model)?)
}

/// Update an existing resource
/// Note: This is a placeholder - in production you would implement actual DB update
pub async fn update<E>(
    db: &DatabaseConnection,
    id: Value,
    data: HashMap<String, Value>,
) -> ResourceResult<Value>
where
    E: EntityTrait,
{
    // This is a simplified placeholder
    // In production, you'd implement actual DB update
    Ok(serde_json::json!({"id": id}))
}

/// Delete a resource by its primary key.
///
/// Resolves the entity's primary-key column dynamically (mirroring
/// `bulk_destroy`), coerces the incoming JSON `id` into a correctly-typed
/// `sea_orm::Value`, and issues a real `DELETE`. Returns `NotFound` when no
/// row matched the given id so callers can surface an honest 404.
pub async fn destroy<E>(db: &DatabaseConnection, id: Value) -> ResourceResult<()>
where
    E: EntityTrait,
{
    let pk_column = <E::PrimaryKey as Iterable>::iter()
        .next()
        .ok_or_else(|| ResourceError::InvalidInput("Entity has no primary key column".to_string()))?
        .into_column();
    let col_type = pk_column.def().get_column_type().clone();
    let pk_value = json_to_pk_value(&id, &col_type)?;

    let result = E::delete_many()
        .filter(pk_column.eq(pk_value))
        .exec(db)
        .await?;

    if result.rows_affected == 0 {
        return Err(ResourceError::NotFound(format!(
            "No resource found with id {id}"
        )));
    }

    Ok(())
}

/// Bulk delete resources by their primary keys.
///
/// Resolves the entity's primary-key column dynamically (the same way
/// `rf-orm`'s `find(id)` does), coerces each incoming JSON id into a
/// `sea_orm::Value` matching that column's type, and issues a single
/// `DELETE ... WHERE pk IN (...)`. Returns the real number of rows removed.
pub async fn bulk_destroy<E>(db: &DatabaseConnection, ids: Vec<Value>) -> ResourceResult<u64>
where
    E: EntityTrait,
{
    if ids.is_empty() {
        return Ok(0);
    }

    // Resolve the entity's primary-key column (first PK column for composite keys).
    let pk_column = <E::PrimaryKey as Iterable>::iter()
        .next()
        .ok_or_else(|| ResourceError::InvalidInput("Entity has no primary key column".to_string()))?
        .into_column();
    let col_type = pk_column.def().get_column_type().clone();

    // Coerce the incoming JSON ids into values matching the primary-key column type.
    let mut pk_values = Vec::with_capacity(ids.len());
    for id in &ids {
        pk_values.push(json_to_pk_value(id, &col_type)?);
    }

    let result = E::delete_many()
        .filter(pk_column.is_in(pk_values))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Convert a JSON id into a `sea_orm::Value` that matches the primary-key
/// column type so the generated `IN (...)` bind parameters are correctly typed.
fn json_to_pk_value(id: &Value, col_type: &sea_orm::ColumnType) -> ResourceResult<sea_orm::Value> {
    use sea_orm::ColumnType;

    let invalid = || {
        ResourceError::InvalidInput(format!(
            "id {id} is not compatible with the primary-key column type {col_type:?}"
        ))
    };

    let value = match col_type {
        ColumnType::TinyInteger => id.as_i64().map(|n| sea_orm::Value::from(n as i8)),
        ColumnType::SmallInteger => id.as_i64().map(|n| sea_orm::Value::from(n as i16)),
        ColumnType::Integer => id.as_i64().map(|n| sea_orm::Value::from(n as i32)),
        ColumnType::BigInteger => id.as_i64().map(sea_orm::Value::from),
        ColumnType::TinyUnsigned => id.as_u64().map(|n| sea_orm::Value::from(n as u8)),
        ColumnType::SmallUnsigned => id.as_u64().map(|n| sea_orm::Value::from(n as u16)),
        ColumnType::Unsigned => id.as_u64().map(|n| sea_orm::Value::from(n as u32)),
        ColumnType::BigUnsigned => id.as_u64().map(sea_orm::Value::from),
        ColumnType::String(_) | ColumnType::Text | ColumnType::Char(_) => {
            id.as_str().map(|s| sea_orm::Value::from(s.to_string()))
        }
        // Fallback: infer from the JSON shape for column types we don't special-case.
        _ => match id {
            Value::Number(n) if n.is_i64() => n.as_i64().map(sea_orm::Value::from),
            Value::String(s) => Some(sea_orm::Value::from(s.clone())),
            _ => None,
        },
    };

    value.ok_or_else(invalid)
}

/// Convert an arbitrary JSON value into a `sea_orm::Value` matching the target
/// column's type, so `INSERT`/`UPDATE` bind parameters are correctly typed.
///
/// This generalises `json_to_pk_value` to the full range of column types a
/// create payload may touch (booleans, floats, text, and — for null — the
/// column's typed NULL variant). Type-incompatible values yield `InvalidInput`.
fn json_to_column_value(
    value: &Value,
    col_type: &sea_orm::ColumnType,
) -> ResourceResult<sea_orm::Value> {
    use sea_orm::ColumnType;

    let invalid = || {
        ResourceError::InvalidInput(format!(
            "value {value} is not compatible with column type {col_type:?}"
        ))
    };

    // A JSON null maps to a typed SQL NULL for the column.
    if value.is_null() {
        return Ok(typed_null(col_type));
    }

    let coerced = match col_type {
        ColumnType::Boolean => value.as_bool().map(sea_orm::Value::from),
        ColumnType::TinyInteger => value.as_i64().map(|n| sea_orm::Value::from(n as i8)),
        ColumnType::SmallInteger => value.as_i64().map(|n| sea_orm::Value::from(n as i16)),
        ColumnType::Integer => value.as_i64().map(|n| sea_orm::Value::from(n as i32)),
        ColumnType::BigInteger => value.as_i64().map(sea_orm::Value::from),
        ColumnType::TinyUnsigned => value.as_u64().map(|n| sea_orm::Value::from(n as u8)),
        ColumnType::SmallUnsigned => value.as_u64().map(|n| sea_orm::Value::from(n as u16)),
        ColumnType::Unsigned => value.as_u64().map(|n| sea_orm::Value::from(n as u32)),
        ColumnType::BigUnsigned => value.as_u64().map(sea_orm::Value::from),
        ColumnType::Float => value.as_f64().map(|n| sea_orm::Value::from(n as f32)),
        ColumnType::Double => value.as_f64().map(sea_orm::Value::from),
        ColumnType::String(_) | ColumnType::Text | ColumnType::Char(_) => {
            value.as_str().map(|s| sea_orm::Value::from(s.to_string()))
        }
        // JSON columns are stored as their serialized text form (no dependence
        // on sea-orm's optional `with-json` Value variant).
        ColumnType::Json | ColumnType::JsonBinary => Some(sea_orm::Value::from(value.to_string())),
        // Fallback: infer from the JSON shape for column types we don't special-case.
        _ => match value {
            Value::Bool(b) => Some(sea_orm::Value::from(*b)),
            Value::Number(n) if n.is_i64() => n.as_i64().map(sea_orm::Value::from),
            Value::Number(n) if n.is_u64() => n.as_u64().map(sea_orm::Value::from),
            Value::Number(n) => n.as_f64().map(sea_orm::Value::from),
            Value::String(s) => Some(sea_orm::Value::from(s.clone())),
            _ => None,
        },
    };

    coerced.ok_or_else(invalid)
}

/// Produce the typed NULL `sea_orm::Value` variant for a column type so an
/// explicit JSON null binds as the correct nullable parameter.
fn typed_null(col_type: &sea_orm::ColumnType) -> sea_orm::Value {
    use sea_orm::ColumnType;

    match col_type {
        ColumnType::Boolean => sea_orm::Value::Bool(None),
        ColumnType::TinyInteger => sea_orm::Value::TinyInt(None),
        ColumnType::SmallInteger => sea_orm::Value::SmallInt(None),
        ColumnType::Integer => sea_orm::Value::Int(None),
        ColumnType::BigInteger => sea_orm::Value::BigInt(None),
        ColumnType::TinyUnsigned => sea_orm::Value::TinyUnsigned(None),
        ColumnType::SmallUnsigned => sea_orm::Value::SmallUnsigned(None),
        ColumnType::Unsigned => sea_orm::Value::Unsigned(None),
        ColumnType::BigUnsigned => sea_orm::Value::BigUnsigned(None),
        ColumnType::Float => sea_orm::Value::Float(None),
        ColumnType::Double => sea_orm::Value::Double(None),
        _ => sea_orm::Value::String(None),
    }
}

/// Export resources to various formats
pub enum ExportFormat {
    Json,
    Csv,
}

/// Export resources
pub async fn export<E>(
    db: &DatabaseConnection,
    query: ResourceQuery,
    format: ExportFormat,
) -> ResourceResult<String>
where
    E: EntityTrait,
    E::Model: Serialize + Send + Sync,
{
    // Fetch all matching records (no pagination for export)
    let mut select = E::find();

    // Apply filters similar to index()
    // ... (omitted for brevity)

    let records = select.all(db).await?;

    match format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&records)
                .map_err(|e| ResourceError::InvalidInput(e.to_string()))?;
            Ok(json)
        }
        ExportFormat::Csv => {
            // Convert to CSV
            let mut wtr = csv::Writer::from_writer(vec![]);

            // Write records
            for record in records {
                let json = serde_json::to_value(&record)
                    .map_err(|e| ResourceError::InvalidInput(e.to_string()))?;

                if let Value::Object(map) = json {
                    let values: Vec<String> = map.values().map(|v| format!("{}", v)).collect();
                    wtr.write_record(&values)
                        .map_err(|e| ResourceError::InvalidInput(e.to_string()))?;
                }
            }

            let data = wtr
                .into_inner()
                .map_err(|e| ResourceError::InvalidInput(e.to_string()))?;
            let csv = String::from_utf8(data)
                .map_err(|e| ResourceError::InvalidInput(e.to_string()))?;

            Ok(csv)
        }
    }
}

// Implement conversions
impl From<serde_json::Error> for ResourceError {
    fn from(err: serde_json::Error) -> Self {
        ResourceError::InvalidInput(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, Set};

    mod widget {
        use sea_orm::entity::prelude::*;
        use serde::Serialize;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
        #[sea_orm(table_name = "widgets")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub name: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    mod thing {
        use sea_orm::entity::prelude::*;
        use serde::Serialize;

        // String primary key to exercise the text-coercion path.
        #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
        #[sea_orm(table_name = "things")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub slug: String,
            pub label: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    async fn seed_widgets(db: &DatabaseConnection) {
        db.execute_unprepared(
            "CREATE TABLE widgets (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        )
        .await
        .unwrap();

        for (id, name) in [(1, "a"), (2, "b"), (3, "c"), (4, "d")] {
            widget::ActiveModel {
                id: Set(id),
                name: Set(name.to_string()),
            }
            .insert(db)
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn bulk_destroy_removes_exactly_the_given_integer_ids() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        seed_widgets(&db).await;

        let removed = bulk_destroy::<widget::Entity>(
            &db,
            vec![serde_json::json!(2), serde_json::json!(4)],
        )
        .await
        .unwrap();

        assert_eq!(removed, 2, "should report exactly the two rows deleted");

        let remaining: Vec<i32> = widget::Entity::find()
            .all(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(remaining, vec![1, 3], "only ids 1 and 3 should survive");
    }

    #[tokio::test]
    async fn bulk_destroy_ignores_missing_ids_and_reports_real_count() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        seed_widgets(&db).await;

        // id 99 does not exist; only id 1 should be deleted.
        let removed = bulk_destroy::<widget::Entity>(
            &db,
            vec![serde_json::json!(1), serde_json::json!(99)],
        )
        .await
        .unwrap();

        assert_eq!(removed, 1, "count must reflect actually-deleted rows only");
        assert_eq!(widget::Entity::find().all(&db).await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn bulk_destroy_empty_ids_is_a_noop() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        seed_widgets(&db).await;

        let removed = bulk_destroy::<widget::Entity>(&db, vec![]).await.unwrap();
        assert_eq!(removed, 0);
        assert_eq!(widget::Entity::find().all(&db).await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn bulk_destroy_works_with_string_primary_keys() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE things (slug TEXT PRIMARY KEY, label TEXT NOT NULL)",
        )
        .await
        .unwrap();
        for (slug, label) in [("x", "X"), ("y", "Y"), ("z", "Z")] {
            thing::ActiveModel {
                slug: Set(slug.to_string()),
                label: Set(label.to_string()),
            }
            .insert(&db)
            .await
            .unwrap();
        }

        let removed = bulk_destroy::<thing::Entity>(
            &db,
            vec![serde_json::json!("x"), serde_json::json!("z")],
        )
        .await
        .unwrap();

        assert_eq!(removed, 2);
        let remaining: Vec<String> = thing::Entity::find()
            .all(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|m| m.slug)
            .collect();
        assert_eq!(remaining, vec!["y".to_string()]);
    }

    #[tokio::test]
    async fn create_persists_row_and_returns_real_id() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE widgets (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
        )
        .await
        .unwrap();

        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_json::json!("gizmo"));

        let created = create::<widget::Entity>(&db, data).await.unwrap();

        // The returned JSON must carry a real, database-assigned id (not a stub 1
        // by coincidence — assert it is a positive integer and the name round-trips).
        let id = created.get("id").and_then(|v| v.as_i64()).unwrap();
        assert!(id >= 1, "created row must expose a real primary key");
        assert_eq!(created.get("name").and_then(|v| v.as_str()), Some("gizmo"));

        // Prove it was actually persisted by reading it back from the DB.
        let fetched = widget::Entity::find_by_id(id as i32)
            .one(&db)
            .await
            .unwrap()
            .expect("row must exist in the database");
        assert_eq!(fetched.name, "gizmo");
    }

    #[tokio::test]
    async fn create_then_destroy_round_trip() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE widgets (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
        )
        .await
        .unwrap();

        let mut data = HashMap::new();
        data.insert("name".to_string(), serde_json::json!("temp"));
        let created = create::<widget::Entity>(&db, data).await.unwrap();
        let id = created.get("id").and_then(|v| v.as_i64()).unwrap();

        // Row exists...
        assert!(widget::Entity::find_by_id(id as i32)
            .one(&db)
            .await
            .unwrap()
            .is_some());

        // ...destroy removes it for real.
        destroy::<widget::Entity>(&db, serde_json::json!(id))
            .await
            .unwrap();

        assert!(widget::Entity::find_by_id(id as i32)
            .one(&db)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn destroy_missing_id_is_not_found() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        seed_widgets(&db).await;

        let err = destroy::<widget::Entity>(&db, serde_json::json!(999))
            .await
            .unwrap_err();
        assert!(matches!(err, ResourceError::NotFound(_)));

        // Nothing was deleted.
        assert_eq!(widget::Entity::find().all(&db).await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn create_works_with_string_primary_key_and_ignores_unknown_keys() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE things (slug TEXT PRIMARY KEY, label TEXT NOT NULL)",
        )
        .await
        .unwrap();

        let mut data = HashMap::new();
        data.insert("slug".to_string(), serde_json::json!("alpha"));
        data.insert("label".to_string(), serde_json::json!("Alpha"));
        // Unknown key must be ignored rather than error.
        data.insert("nonexistent".to_string(), serde_json::json!("ignored"));

        let created = create::<thing::Entity>(&db, data).await.unwrap();
        assert_eq!(created.get("slug").and_then(|v| v.as_str()), Some("alpha"));

        let fetched = thing::Entity::find_by_id("alpha".to_string())
            .one(&db)
            .await
            .unwrap()
            .expect("row must exist");
        assert_eq!(fetched.label, "Alpha");
    }
}
