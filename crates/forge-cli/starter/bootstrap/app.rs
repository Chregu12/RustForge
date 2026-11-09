use rf::App;

pub fn routes() -> App {
    crate::routes::api::register(crate::routes::web::register(App::new()))
        .get("/up", || async { axum::http::StatusCode::OK })
}
