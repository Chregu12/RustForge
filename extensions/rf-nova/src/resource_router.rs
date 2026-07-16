//! Real, generic CRUD router for a single SeaORM entity.
//!
//! [`resource_router`] builds an axum [`Router`] whose five handlers call the
//! real [`crud`] functions — they actually read and write the database.
//!
//! # Status
//!
//! This is the **minimal working CRUD path** in rf-nova:
//!
//! | Path            | Method | Description                     |
//! |-----------------|--------|---------------------------------|
//! | `/`             | GET    | Paginated list (`?page`, `?per_page`, `?search` accepted but search is no-op in the generic path — see [`crud::index`]) |
//! | `/`             | POST   | Insert row, return persisted JSON |
//! | `/{id}`         | GET    | Single row by primary key       |
//! | `/{id}`         | PUT    | Update row, return persisted JSON |
//! | `/{id}`         | DELETE | Delete row (204 No Content)     |
//!
//! The `id` segment is always forwarded as a JSON string to the crud layer,
//! which coerces it to the correct primary-key type for the entity.
//!
//! # Experimental / not yet wired
//!
//! The multi-resource type-erased dispatch in `Nova::routes()` +
//! `handlers.rs` is **experimental** — those handlers currently return
//! hardcoded placeholder data and never touch the database. Use
//! `resource_router::<YourEntity>` for actual CRUD.
//!
//! # Example
//!
//! ```ignore
//! use rf_nova::resource_router;
//! use sea_orm::Database;
//! use std::sync::Arc;
//!
//! let db = Arc::new(Database::connect("sqlite://app.db").await?);
//! let router = resource_router::<post::Entity>(db);
//! // nest into your app:
//! let app = axum::Router::new().nest("/admin/posts", router);
//! ```

use crate::resource::crud;
use crate::resource::resource::{PaginatedResponse, ResourceError, ResourceQuery};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use sea_orm::{DatabaseConnection, EntityTrait, IntoActiveModel};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Convert a URL path segment id to the most appropriate JSON value:
/// tries to parse as an i64 first (integer PKs), falls back to a String
/// (UUID / slug PKs). The `json_to_pk_value` machinery in crud.rs then
/// coerces this JSON value to the correct `sea_orm::Value` for the column type.
fn parse_path_id(id: &str) -> Value {
    if let Ok(n) = id.parse::<i64>() {
        Value::Number(n.into())
    } else {
        Value::String(id.to_owned())
    }
}

/// Build a real CRUD [`Router`] for a single concrete SeaORM entity `E`.
///
/// The returned router has state `Arc<DatabaseConnection>` already applied (it
/// returns plain `Router`, i.e. `Router<()>`), so you can `.nest()` or
/// `.merge()` it directly into any axum application.
pub fn resource_router<E>(db: Arc<DatabaseConnection>) -> Router
where
    E: EntityTrait + Send + Sync + 'static,
    E::Model: Serialize + IntoActiveModel<E::ActiveModel> + Send + Sync + 'static,
    E::ActiveModel: sea_orm::ActiveModelTrait + Send + 'static,
{
    Router::new()
        .route("/", get(index_handler::<E>).post(create_handler::<E>))
        .route("/{id}", get(show_handler::<E>).put(update_handler::<E>).delete(delete_handler::<E>))
        .with_state(db)
}

// ---------------------------------------------------------------------------
// Handlers — one per CRUD operation; all generic over E.
//
// Axum accepts monomorphized function items here: `index_handler::<E>` is a
// concrete zero-sized type once E is resolved, and axum's Handler blanket impl
// covers it.
// ---------------------------------------------------------------------------

async fn index_handler<E>(
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode>
where
    E: EntityTrait + Send + Sync,
    E::Model: Serialize + Send + Sync,
{
    let query = ResourceQuery {
        page: params.get("page").and_then(|v| v.parse().ok()),
        per_page: params.get("per_page").and_then(|v| v.parse().ok()),
        search: params.get("search").cloned(),
        sort_by: params.get("sort_by").cloned(),
        sort_order: params.get("sort_order").cloned(),
        filters: None,
    };

    // Pass empty searchable_fields: the generic crud::index does not yet
    // support dynamic column-name-to-ColumnTrait resolution, so passing an
    // empty slice avoids the WHERE FALSE bug while the search param is noted
    // but not applied.
    crud::index::<E>(&db, query, vec![])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .and_then(|page: PaginatedResponse<E::Model>| {
            serde_json::to_value(page)
                .map(Json)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        })
}

async fn show_handler<E>(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode>
where
    E: EntityTrait + Send + Sync,
    E::Model: Serialize,
{
    crud::show::<E>(&db, parse_path_id(&id))
        .await
        .map(Json)
        .map_err(|e| match e {
            ResourceError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
}

async fn create_handler<E>(
    State(db): State<Arc<DatabaseConnection>>,
    Json(data): Json<HashMap<String, Value>>,
) -> Result<Json<Value>, StatusCode>
where
    E: EntityTrait + Send + Sync,
    E::Model: Serialize + IntoActiveModel<E::ActiveModel>,
{
    crud::create::<E>(&db, data)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn update_handler<E>(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<String>,
    Json(data): Json<HashMap<String, Value>>,
) -> Result<Json<Value>, StatusCode>
where
    E: EntityTrait + Send + Sync,
    E::Model: Serialize + IntoActiveModel<E::ActiveModel>,
    E::ActiveModel: Send,
{
    crud::update::<E>(&db, parse_path_id(&id), data)
        .await
        .map(Json)
        .map_err(|e| match e {
            ResourceError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
}

async fn delete_handler<E>(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode>
where
    E: EntityTrait + Send + Sync,
{
    crud::destroy::<E>(&db, parse_path_id(&id))
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| match e {
            ResourceError::NotFound(_) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
}
