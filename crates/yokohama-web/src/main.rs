//! The server binary: bind a port, serve the API.
//!
//! Deliberately thin, exactly as `yokohama-cli`'s `main.rs` is — everything
//! testable lives in the library half, which route tests drive without a socket.

use std::sync::Arc;

use yokohama_web::{app, AppState};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("YOKOHAMA_WEB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let state = Arc::new(AppState::new());
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind 127.0.0.1:{port}: {e}");
            std::process::exit(1);
        }
    };
    println!("yokohama-web listening on http://127.0.0.1:{port}");
    if let Err(e) = axum::serve(listener, app(state)).await {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
