use axum::http::StatusCode;
use rf::{App, DB};
use std::time::Duration;

pub fn routes() -> App {
    crate::routes::api::register(crate::routes::web::register(App::new()))
        .get("/up", || async { StatusCode::OK })
        .get("/ready", ready)
}

async fn ready() -> StatusCode {
    let check = DB::select_async("SELECT 1 FROM posts LIMIT 0", &[]);
    match tokio::time::timeout(Duration::from_secs(2), check).await {
        Ok(Ok(_)) => StatusCode::OK,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}
