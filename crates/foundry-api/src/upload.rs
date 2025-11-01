use crate::error::{HttpError, HttpResult};
use crate::http::AppState;
use crate::JsonResponse;
use axum::{
    extract::{multipart::Multipart, State},
    http::StatusCode,
};
use foundry_plugins::{AppError, AppResult};

#[derive(serde::Serialize)]
pub struct UploadResponse {
    pub path: String,
    pub url: String,
    pub size: u64,
    pub mime_type: String,
}

const DEFAULT_MIME: &str = "application/octet-stream";

pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> HttpResult<JsonResponse<UploadResponse>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| HttpError::from(parse_error("MULTIPART_ERROR", err)))?
    {
        let file_name = field.file_name().unwrap_or("file").to_string();
        let mime_type = field.content_type().unwrap_or(DEFAULT_MIME).to_string();

        let contents = field
            .bytes()
            .await
            .map_err(|err| HttpError::from(parse_error("READ_ERROR", err)))?;

        validate_upload(&file_name, &contents, &mime_type).map_err(HttpError::from)?;

        let file_service = state.file_service();
        let path = file_service
            .store(contents.clone(), &file_name, Some("public"))
            .await
            .map_err(|err| HttpError::from(storage_error("STORE_FAILED", err)))?;

        let url = file_service
            .url(&path, Some("public"))
            .map_err(|err| HttpError::from(storage_error("URL_RESOLUTION_FAILED", err)))?;

        return Ok(JsonResponse::created(UploadResponse {
            path,
            url,
            size: contents.len() as u64,
            mime_type,
        }));
    }

    Err(HttpError::from(
        AppError::validation("file", "No file uploaded")
            .with_status(StatusCode::BAD_REQUEST.as_u16()),
    ))
}

fn validate_upload(filename: &str, contents: &[u8], mime_type: &str) -> AppResult<()> {
    const MAX_SIZE: usize = 50 * 1024 * 1024; // 50 MB
    const ALLOWED_TYPES: &[&str] = &[
        "image/jpeg",
        "image/png",
        "image/gif",
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "text/plain",
    ];

    let normalized_mime = mime_type
        .split(';')
        .next()
        .map(|value| value.trim())
        .unwrap_or(mime_type);

    if contents.len() > MAX_SIZE {
        return Err(
            AppError::validation("file", "File exceeds maximum size of 50 MB")
                .with_status(StatusCode::BAD_REQUEST.as_u16()),
        );
    }

    if !ALLOWED_TYPES.contains(&normalized_mime) {
        return Err(AppError::validation("mime_type", "File type not allowed")
            .with_status(StatusCode::BAD_REQUEST.as_u16()));
    }

    if is_malicious(contents)? {
        return Err(
            AppError::new("SUSPICIOUS_FILE", "Suspicious file content detected")
                .with_status(StatusCode::BAD_REQUEST.as_u16())
                .with_context("file", filename.to_string()),
        );
    }

    Ok(())
}

fn is_malicious(contents: &[u8]) -> AppResult<bool> {
    let content_str = String::from_utf8_lossy(contents);

    let dangerous_patterns = [
        "<?php",
        "<%=",
        "<script",
        "javascript:",
        "onerror=",
        "onload=",
    ];

    for pattern in dangerous_patterns {
        if content_str.contains(pattern) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn parse_error(code: &str, error: impl ToString) -> AppError {
    AppError::new(code, error.to_string()).with_status(StatusCode::BAD_REQUEST.as_u16())
}

fn storage_error(code: &str, error: impl ToString) -> AppError {
    AppError::new(code, "Storage operation failed")
        .with_status(StatusCode::INTERNAL_SERVER_ERROR.as_u16())
        .with_context("details", error.to_string())
}
