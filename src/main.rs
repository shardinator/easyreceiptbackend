//! HTTP server binary for EasyReceipt.
//!
//! Binds TCP and serves the router from `easyreceiptbackend::create_router`.
//! On Fly.io, `PORT` is set: we listen on `0.0.0.0` so the proxy can reach the process.
//! Locally, default is `127.0.0.1:3000`.

use easyreceiptbackend::create_router;
use std::env;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = create_router();

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let fly_or_container = env::var("PORT").is_ok();
    let addr = if fly_or_container {
        SocketAddr::from(([0, 0, 0, 0], port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], port))
    };

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    eprintln!("listening on http://{addr}");
    axum::serve(listener, app).await.expect("serve");
}