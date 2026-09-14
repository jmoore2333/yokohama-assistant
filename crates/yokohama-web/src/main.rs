//! The server binary: bind a port, serve the API.
//!
//! Deliberately thin, exactly as `yokohama-cli`'s `main.rs` is — everything
//! testable lives in the library half, which route tests drive without a socket.

use std::sync::Arc;

use yokohama_web::{app, AppState};

/// Resolve when the platform asks this process to stop.
///
/// **This exists because of what a `POST /input` does in the middle.**
/// `Repl::step` mutates the resident `Repl` and only then does `Games::persist`
/// write the record. A process killed between those two points has applied a
/// move it never stored: the client gets no response, and the move is gone.
///
/// That is not a hypothetical on the deployment shapes this project is heading
/// for. A Cloudflare Container and a Modal `@modal.web_server` both scale to
/// zero, and both do it by SIGTERM — which is a signal a process can be asked
/// to finish its work under, and which this binary previously ignored while
/// `Cargo.toml` already enabled tokio's `signal` feature, unused.
///
/// **What this buys and what it does not.** `axum::serve(...)
/// .with_graceful_shutdown(...)` stops accepting new connections and then waits
/// for the requests already in flight, so an acknowledged move is a stored
/// move. It does NOT bound how long the platform will wait before SIGKILL, and
/// it does nothing for a hard kill or a power loss — the guarantee is "a move
/// this server answered 200 for is on disk", not "no move is ever lost".
async fn shutdown_requested() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            // A process that cannot install the handler should still serve; it
            // simply loses the graceful half. Failing to start would be a worse
            // answer than starting without it.
            Err(e) => {
                eprintln!("could not listen for SIGTERM ({e}); shutdown will not be graceful");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    println!("shutdown requested — draining in-flight requests");
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("YOKOHAMA_WEB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // **The default stays loopback, and that is the safe half of this knob.**
    // This API has no authentication of any kind — `GET /api/games` lists every
    // game to any caller — so a bind address is a real decision and not a
    // convenience. A container needs `0.0.0.0` to be reachable at all, and it
    // is expected to sit behind an identity proxy that terminates before the
    // request arrives. Making the host explicit means that choice is made by
    // whoever deploys it rather than by this line.
    let host = std::env::var("YOKOHAMA_WEB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    // **No default identity source (R-1.3).** Which header names the caller is
    // a deployment decision — `claimed` trusts anything past the door, and the
    // two proxy modes trust a header only their own edge can set — so a process
    // that was not told refuses to start rather than guessing the permissive
    // one. The Worker's own fail-closed shape for its secrets.
    let identity = match yokohama_web::identity::IdentityMode::from_env() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let state = Arc::new(AppState::new(identity));
    let listener = match tokio::net::TcpListener::bind((host.as_str(), port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind {host}:{port}: {e}");
            std::process::exit(1);
        }
    };
    println!("yokohama-web listening on http://{host}:{port}");
    if let Err(e) = axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_requested())
        .await
    {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
