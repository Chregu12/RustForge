use crate::app::http::controllers::post_controller as posts;
use rf::App;

pub fn register(app: App) -> App {
    app.get("/api/posts", posts::index)
        .post("/api/posts", posts::store)
        .get("/api/posts/{post}", posts::show)
        .put("/api/posts/{post}", posts::update)
        .patch("/api/posts/{post}", posts::patch)
        .delete("/api/posts/{post}", posts::destroy)
}
