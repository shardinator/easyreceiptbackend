//! HTTP API: Axum routes only. Hashing stays in [`crate::Sha256Hash`].

use crate::{EntryStore, Sha256Hash};
use axum::{
    routing::{get, post},
    extract::State,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Deserialize)]
pub(crate) struct HashRequest {
    text: String,
}

#[derive(Serialize)]
pub(crate) struct HashSaved {
    id: String,
    count: u64,
    timestamp_ms: u128,
}

#[derive(Serialize)]
pub(crate) struct HashResponseWithSave {
    hash: String,
    saved: HashSaved,
}

#[derive(Serialize)]
pub(crate) struct EntryRow {
    id: String,
    count: u64,
    timestamp_ms: u128,
    text: String,
    hash: String,
}

async fn hash_handler(
    State(store): State<Arc<EntryStore>>,
    Json(payload): Json<HashRequest>,
) -> Json<HashResponseWithSave> {
    let hash = Sha256Hash::digest_hex(&payload.text);
    let saved = store
        .append(&payload.text, &hash)
        .expect("failed to append hash entry");
    let saved = HashSaved {
        id: saved.id,
        count: saved.count,
        timestamp_ms: saved.timestamp_ms,
    };
    Json(HashResponseWithSave { hash, saved })
}

async fn entries_handler(State(store): State<Arc<EntryStore>>) -> Json<Vec<EntryRow>> {
    let rows = store.read_all().unwrap_or_default();
    let rows = rows
        .into_iter()
        .map(|r| EntryRow {
            id: r.id,
            count: r.count,
            timestamp_ms: r.timestamp_ms,
            text: r.text,
            hash: r.hash,
        })
        .collect();
    Json(rows)
}

/// Full Axum router (JSON API + permissive CORS for browser clients).
pub fn create_router() -> Router {
    let path = std::env::var("EASYRECEIPT_HASH_STORE_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            if std::env::var("PORT").is_ok() {
                "/data/hash_entries.jsonl".to_string()
            } else {
                "hash_entries.jsonl".to_string()
            }
        });
    let store = Arc::new(EntryStore::new(path).expect("init hash entry store"));

    Router::new()
        .route("/", get(|| async { "OK" }))
        .route("/api/hash", post(hash_handler))
        .route("/api/entries", get(entries_handler))
        .with_state(store)
        .layer(CorsLayer::permissive())
}
