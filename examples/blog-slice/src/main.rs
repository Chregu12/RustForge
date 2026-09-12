//! The canonical, executable getting-started example. Documentation is generated
//! from the marked section below by scripts/sync-getting-started.py.

// docs:begin
use axum::{http::StatusCode, response::IntoResponse};
use rf::prelude::*;

Model!(Post: title, body);

async fn store() -> AppResult<impl IntoResponse> {
    let data = validate! { title: string.max(200), body: string }?;
    let post = Post::create(data).await?;
    Ok(json(post).status(StatusCode::CREATED))
}

async fn index() -> AppResult<impl IntoResponse> {
    Ok(json(Post::all_typed().await?))
}

async fn show() -> AppResult<impl IntoResponse> {
    let id = input::<i64>("id").ok_or_else(|| AppError::BadRequest {
        message: "Invalid post id".into(),
    })?;
    Ok(json(Post::find(id).await?.or_404()?))
}

async fn build_app() -> AppResult<axum::Router> {
    let db = DatabaseManager::connect(DatabaseConfig {
        url: "sqlite::memory:".into(),
        max_connections: 1,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .map_err(|e| e.to_string())?;
    // Demo schema. Production applications should apply versioned migrations.
    db.scope(DB::statement_async(
        "CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL)"
    )).await?;
    Ok(App::new()
        .database(db)
        .get("/posts", index)
        .get("/posts/{id}", show)
        .post("/posts", store)
        .build())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = build_app().await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
// docs:end

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    async fn call(
        app: &axum::Router,
        method: &str,
        uri: &str,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn documented_app_preserves_http_contract_and_isolation() {
        let a = build_app().await.unwrap();
        let b = build_app().await.unwrap();
        let (status, post) = call(
            &a,
            "POST",
            "/posts",
            r#"{"title":"Hello","body":"World","admin":true}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(post.get("admin").is_none());
        let id = post["id"].as_i64().unwrap();
        let (status, shown) = call(&a, "GET", &format!("/posts/{id}"), "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(shown["title"], "Hello");
        assert_eq!(
            call(&a, "GET", "/posts/999999", "").await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            call(&a, "GET", "/posts/no-id", "").await.0,
            StatusCode::BAD_REQUEST
        );
        let (status, error) = call(&a, "POST", "/posts", r#"{"title":"","body":"World"}"#).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(error.to_string().contains("title"));
        assert_eq!(call(&b, "GET", "/posts", "").await.1, serde_json::json!([]));
        assert_eq!(
            call(&a, "GET", "/posts", "")
                .await
                .1
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
