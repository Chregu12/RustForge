use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use rf::prelude::*;
use rf_validation::ValidatedData;

use crate::app::{http::requests::post_request, models::post::Post};

pub async fn index() -> AppResult<impl IntoResponse> {
    Ok(json(Post::all_typed().await?))
}

pub async fn store() -> AppResult<impl IntoResponse> {
    let data = post_request::validate_post(false).await?;
    Ok(json(Post::create(data).await?).status(StatusCode::CREATED))
}

pub async fn show(Path(id): Path<i64>) -> AppResult<impl IntoResponse> {
    Ok(json(Post::find(id).await?.or_404()?))
}

pub async fn update(Path(id): Path<i64>) -> AppResult<impl IntoResponse> {
    persist(id, post_request::validate_post(false).await?).await
}

pub async fn patch(Path(id): Path<i64>) -> AppResult<impl IntoResponse> {
    persist(id, post_request::validate_post(true).await?).await
}

async fn persist(id: i64, data: ValidatedData) -> AppResult<impl IntoResponse> {
    let post = DB::transaction(async {
        DB::table("posts").r#where("id", id).update(data).await?;
        Post::find(id).await
    })
    .await?;
    Ok(json(post.or_404()?))
}

pub async fn destroy(Path(id): Path<i64>) -> AppResult<StatusCode> {
    let count = DB::table("posts").r#where("id", id).delete().await?;
    (count > 0).then_some(()).or_404()?;
    Ok(StatusCode::NO_CONTENT)
}
