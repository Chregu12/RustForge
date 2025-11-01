use crate::app::http::middleware::check_maintenance::check_maintenance;
use axum::Router;
use foundry_api::{app_router, AppRouter, HttpServer};

pub fn build(server: HttpServer) -> Router {
    let server = server.merge_router(app_routes());
    let server = server.with_middleware(check_maintenance);
    server.into_router()
}

fn app_routes() -> AppRouter {
    app_router()
    // .merge(crate::app::http::routes::account::routes())
}
