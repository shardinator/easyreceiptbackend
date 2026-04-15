//! Integration tests: full HTTP stack for `POST /api/hash`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn extract_json_string_field(body: &[u8], field: &str) -> Option<String> {
    let s = std::str::from_utf8(body).ok()?;
    let needle = format!("\"{field}\":\"");
    let start = s.find(&needle)? + needle.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[tokio::test]
async fn post_hash_returns_known_digest_for_abc() {
    let app = easyreceiptbackend::create_router();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/hash")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":"abc"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        extract_json_string_field(&body, "hash").as_deref().unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[tokio::test]
async fn post_hash_empty_string() {
    let app = easyreceiptbackend::create_router();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/hash")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"text":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(
        extract_json_string_field(&body, "hash").as_deref().unwrap(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}
