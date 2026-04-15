//! HTTP API: Axum routes only. Hashing stays in [`crate::Sha256Hash`].

use crate::Sha256Hash;
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
pub(crate) struct HashRequest {
    text: String,
}

#[derive(Serialize)]
pub(crate) struct HashResponse {
    hash: String,
}

async fn hash_handler(Json(payload): Json<HashRequest>) -> Json<HashResponse> {
    let hash = Sha256Hash::digest_hex(payload.text);
    Json(HashResponse { hash })
}

/// Full Axum router (JSON API + permissive CORS for browser clients).
pub fn create_router() -> Router {
    Router::new()
        .route("/", get(|| async { "OK" }))
        .route("/api/hash", post(hash_handler))
        .layer(CorsLayer::permissive())
}
