//! CRUD operations for Nova resources
//!
//! Provides helper functions for common database operations.

use super::resource::{PaginatedResponse, PaginationMeta, ResourceError, ResourceQuery, ResourceResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
};
use serde::Serialize;
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
    if let Some(_sort_field) = &query.sort_by {
        // Note: Sorting would need to be implemented based on the entity's columns
        // This is a placeholder for the actual implementation
    }

    // Apply filters
    if let Some(_filters) = &query.filters {
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
pub async fn show<E>(_db: &DatabaseConnection, _id: Value) -> ResourceResult<Value>
where
    E: EntityTrait,
{
    // This is a simplified placeholder
    // In production, you'd implement actual DB query using the entity
    Err(ResourceError::NotFound("Not implemented".to_string()))
}

/// Create a new resource
/// Note: This is a placeholder - in production you would implement actual DB insert
pub async fn create<E>(
    _db: &DatabaseConnection,
    _data: HashMap<String, Value>,
) -> ResourceResult<Value>
where
    E: EntityTrait,
{
    // This is a simplified placeholder
    // In production, you'd implement actual DB insert
    Ok(serde_json::json!({"id": 1}))
}

/// Update an existing resource
/// Note: This is a placeholder - in production you would implement actual DB update
pub async fn update<E>(
    _db: &DatabaseConnection,
    id: Value,
    _data: HashMap<String, Value>,
) -> ResourceResult<Value>
where
    E: EntityTrait,
{
    // This is a simplified placeholder
    // In production, you'd implement actual DB update
    Ok(serde_json::json!({"id": id}))
}

/// Delete a resource
/// Note: This is a placeholder - in production you would implement actual DB delete
pub async fn destroy<E>(_db: &DatabaseConnection, _id: Value) -> ResourceResult<()>
where
    E: EntityTrait,
{
    // This is a simplified placeholder
    // In production, you'd implement actual DB delete
    Ok(())
}

/// Bulk delete resources
pub async fn bulk_destroy<E>(_db: &DatabaseConnection, _ids: Vec<Value>) -> ResourceResult<u64>
where
    E: EntityTrait,
{
    // Note: This would need to be implemented based on the entity's primary key type
    // For now, this is a placeholder
    Ok(0)
}

/// Export resources to various formats
pub enum ExportFormat {
    Json,
    Csv,
}

/// Export resources
pub async fn export<E>(
    db: &DatabaseConnection,
    _query: ResourceQuery,
    format: ExportFormat,
) -> ResourceResult<String>
where
    E: EntityTrait,
    E::Model: Serialize + Send + Sync,
{
    // Fetch all matching records (no pagination for export)
    let select = E::find();

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
