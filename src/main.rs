//! HTTP server binary for EasyReceipt.
//!
//! Binds TCP and serves the router built by [`easyreceiptbackend::create_router`].

use easyreceiptbackend::create_router;

#[tokio::main]
async fn main() {
    let app = create_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
