mod common;

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn upload_accepts_plain_text_files() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let boundary = "X-BOUNDARY";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         \r\n\
         hello world\r\n\
         --{boundary}--\r\n"
    );

    let request = Request::builder()
        .method(Method::POST)
        .uri("/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("serialize response");
    let payload: Value = serde_json::from_slice(&bytes).expect("json body");

    let data = payload["data"].as_object().expect("data envelope");
    let path = data["path"].as_str().expect("upload response path present");
    let stored = harness.public_storage_path().join(path);
    if !stored.exists() {
        let entries = list_files(harness.public_storage_path());
        panic!(
            "expected stored file at {} but directory contained: {:?}",
            stored.display(),
            entries
        );
    }

    let contents = std::fs::read_to_string(&stored).expect("read stored file");
    assert_eq!(contents, "hello world");
    let reported_mime = data["mime_type"].as_str().expect("mime type returned");
    assert!(
        reported_mime.starts_with("text/plain"),
        "expected text/plain MIME, got {reported_mime}"
    );
}

#[tokio::test]
async fn upload_rejects_unknown_mime_types() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let boundary = "X-BOUNDARY";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"script.sh\"\r\n\
         Content-Type: application/x-sh\r\n\
         \r\n\
         echo 'danger'\r\n\
         --{boundary}--\r\n"
    );

    let request = Request::builder()
        .method(Method::POST)
        .uri("/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("serialize response");
    let payload: Value = serde_json::from_slice(&bytes).expect("parse error response");
    assert_eq!(payload["code"], "VALIDATION");
    assert_eq!(payload["status"], 400);
    let errors = payload["errors"].as_array().expect("errors array");
    assert!(errors.iter().any(|entry| {
        entry["field"].as_str() == Some("mime_type")
            && entry["message"].as_str() == Some("File type not allowed")
    }));
}

fn list_files(root: std::path::PathBuf) -> Vec<String> {
    fn walk(path: &std::path::Path, acc: &mut Vec<String>, base: &std::path::Path) {
        if let Ok(read_dir) = std::fs::read_dir(path) {
            for entry in read_dir.flatten() {
                let child_path = entry.path();
                if child_path.is_dir() {
                    walk(&child_path, acc, base);
                } else if let Ok(relative) = child_path.strip_prefix(base) {
                    acc.push(relative.to_string_lossy().to_string());
                } else {
                    acc.push(child_path.to_string_lossy().to_string());
                }
            }
        }
    }

    let mut entries = Vec::new();
    walk(&root, &mut entries, &root);
    entries.sort();
    entries
}
