use axum::response::Html;
use rf::App;

pub fn register(app: App) -> App {
    app.get("/", || async {
        Html(include_str!("../resources/views/welcome.html"))
    })
}
