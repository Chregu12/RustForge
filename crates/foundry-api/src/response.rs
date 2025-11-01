use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::Value;

use crate::error::HttpError;

#[derive(Debug, Serialize)]
pub struct ResponseEnvelope<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
}

impl<T> ResponseEnvelope<T> {
    pub fn new(data: T) -> Self {
        Self { data, meta: None }
    }

    pub fn with_meta(mut self, meta: Value) -> Self {
        self.meta = Some(meta);
        self
    }
}

pub struct JsonResponse<T> {
    status: StatusCode,
    payload: ResponseEnvelope<T>,
}

impl<T> JsonResponse<T> {
    pub fn new(status: StatusCode, data: T) -> Self {
        Self {
            status,
            payload: ResponseEnvelope::new(data),
        }
    }

    pub fn ok(data: T) -> Self {
        Self::new(StatusCode::OK, data)
    }

    pub fn created(data: T) -> Self {
        Self::new(StatusCode::CREATED, data)
    }

    pub fn with_meta(mut self, meta: Value) -> Self {
        self.payload = self.payload.with_meta(meta);
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn into_inner(self) -> ResponseEnvelope<T> {
        self.payload
    }
}

impl<T: Serialize> IntoResponse for JsonResponse<T> {
    fn into_response(self) -> Response {
        (self.status, Json(self.payload)).into_response()
    }
}

pub type ApiResult<T> = Result<JsonResponse<T>, HttpError>;
