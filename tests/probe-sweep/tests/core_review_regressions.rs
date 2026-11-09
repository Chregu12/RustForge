use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    response::IntoResponse,
};
use rf::prelude::*;
use tower::ServiceExt;

Model!(ReviewPost {
    title: String,
    body: String
});

async fn database() -> DatabaseManager {
    let db = DatabaseManager::connect(DatabaseConfig {
        max_connections: 1,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .unwrap();
    db.scope(DB::statement_async(
        "CREATE TABLE review_posts (id INTEGER PRIMARY KEY, title TEXT, body TEXT)",
    ))
    .await
    .unwrap();
    db
}

#[tokio::test]
async fn typed_reads_report_bad_rows_instead_of_dropping_them() {
    let db = database().await;
    db.scope(async {
        create!(ReviewPost, title = "valid", body = "row").unwrap();
        DB::insert_async(
            "INSERT INTO review_posts (title, body) VALUES (NULL, 'bad')",
            &[],
        )
        .await
        .unwrap();
        assert_eq!(ReviewPost::all().await.unwrap().len(), 2);
        assert!(ReviewPost::all_typed()
            .await
            .unwrap_err()
            .contains("decode ReviewPost"));
        assert!(ReviewPost::paginate(10, 1)
            .await
            .unwrap_err()
            .contains("decode ReviewPost"));
    })
    .await;
}

#[tokio::test]
async fn scoped_facades_and_injected_services_share_a_pool_and_rollback() {
    let db = database().await;
    db.scope(async {
        let result: Result<(), String> = DB::transaction(async {
            create!(ReviewPost, title = "rollback", body = "test")?;
            assert_eq!(ReviewPost::all().await?.len(), 1);
            Err("abort".into())
        })
        .await;
        assert_eq!(result.unwrap_err(), "abort");
        assert!(ReviewPost::all().await.unwrap().is_empty());
        DB::transaction(async {
            create!(ReviewPost, title = "commit", body = "test")?;
            let nested: Result<(), String> = DB::transaction(async {
                create!(ReviewPost, title = "savepoint", body = "test")?;
                Err("rollback savepoint".into())
            })
            .await;
            assert!(nested.is_err());
            Ok(())
        })
        .await
        .unwrap();
    })
    .await;
    // A separate typed scope sees the same committed row, through the same pool.
    let rows = db
        .scope(DB::select_async("SELECT * FROM review_posts", &[]))
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["title"], "commit");
}

async fn validated() -> AppResult<impl IntoResponse> {
    let data = validate! { enabled: bool }?;
    assert!(
        data.get("admin").is_none(),
        "unvalidated field leaked into ValidatedData"
    );
    #[derive(serde::Deserialize, serde::Serialize)]
    struct Input {
        enabled: bool,
    }
    let dto: Input = data.into_typed().map_err(|e| e.to_string())?;
    Ok(json(dto))
}

#[tokio::test]
async fn boolean_dsl_rejects_wrong_types_and_only_returns_validated_fields() {
    let app = App::new().post("/validate", validated).build();
    for (body, expected) in [
        (r#"{"enabled":true,"admin":true}"#, StatusCode::OK),
        (r#"{"enabled":false}"#, StatusCode::OK),
        (r#"{"enabled":"true"}"#, StatusCode::UNPROCESSABLE_ENTITY),
        (r#"{"enabled":1}"#, StatusCode::UNPROCESSABLE_ENTITY),
        (r#"{"enabled":null}"#, StatusCode::UNPROCESSABLE_ENTITY),
        (r#"{}"#, StatusCode::UNPROCESSABLE_ENTITY),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/validate")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), expected, "{body}");
        if expected == StatusCode::OK {
            let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(value.get("admin").is_none());
        }
    }
}

#[tokio::test]
async fn concurrent_apps_do_not_share_routes_or_databases() {
    let a = database().await;
    let b = database().await;
    let work = |db: DatabaseManager, title: &'static str| async move {
        db.scope(async {
            for _ in 0..8 {
                create!(ReviewPost, title = title, body = "isolated").unwrap();
                tokio::task::yield_now().await;
            }
            let rows = ReviewPost::all_typed().await.unwrap();
            assert_eq!(rows.len(), 8);
            assert!(rows.iter().all(|row| row.title == title));
        })
        .await;
    };
    tokio::join!(work(a, "a"), work(b, "b"));
    let a = App::new().get("/a", || async { "a" }).build();
    let b = App::new().get("/b", || async { "b" }).build();
    assert_eq!(
        a.oneshot(Request::builder().uri("/b").body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        b.oneshot(Request::builder().uri("/a").body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
}
