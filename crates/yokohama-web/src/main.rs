//! The server binary: bind a port, serve the API.
//!
//! Deliberately thin, exactly as `yokohama-cli`'s `main.rs` is — everything
//! testable lives in the library half, which route tests drive without a socket.

use std::sync::Arc;
use std::time::Duration;

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
    // `yokohama-web segment <address>…` prints the archive segment of each
    // address — the current one and the legacy one — for the one-time rename
    // security S1c's deploy performs (`deploy/RUNBOOK.md`, `worker/RUNBOOK.md`).
    // Three columns, tab-separated, one address per line; a value the grammar
    // refuses is a line that says so, and the exit code is 1 if any did.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("segment") {
        let mut failed = false;
        for raw in &args[2..] {
            match yokohama_web::identity::User::parse(raw) {
                Ok(user) => println!("{raw}\t{}\t{}", yokohama_web::archive::legacy_caller_segment(&user), yokohama_web::archive::caller_segment(&user)),
                Err(e) => { failed = true; eprintln!("{raw}\t{e}"); }
            }
        }
        std::process::exit(if failed { 1 } else { 0 });
    }

    let port: u16 = std::env::var("YOKOHAMA_WEB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    // **The default stays loopback, and that is the safe half of this knob.**
    //
    // A bind address is a real decision and not a convenience, and M6.1's
    // identity seam narrowed the reason without removing it. Every
    // `/api/games` route now refuses a request carrying no identity — so this
    // is no longer "no authentication of any kind" — but neither mode makes the
    // port safe to expose:
    //
    // - Under `open` the header is whatever the caller sends. It
    //   authenticates the DOOR, not the seat: two callers sending two different
    //   emails ARE two different people to this server, seated and scoped
    //   separately — it simply cannot verify that either is who they say, so
    //   anyone past the door can be anyone.
    // - Under `cf-access` and `worker` the trusted header is one only that
    //   deployment's own edge can set — which is worth exactly nothing if the
    //   port is reachable without going through the edge, since anyone who can
    //   connect directly can send the same header and be anyone.
    //
    // A container needs `0.0.0.0` to be reachable at all, and it is expected to
    // sit behind that identity proxy. Making the host explicit means the choice
    // is made by whoever deploys it rather than by this line.
    let host = std::env::var("YOKOHAMA_WEB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    // **No default identity source (R-1.3).** Which header names the caller is
    // a deployment decision — `open` trusts anything past the door, and the
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

    // **A client built with a Turnstile site key is the open door's** (M6.5c2
    // cycle F's fix wave; R-FP4): it reads the key as a door with no identity
    // provider in front, so behind `cf-access` or `worker` it would upload
    // nobody's finished games. The Worker's deploy guard holds the pairing
    // (check 12); this is the self-host's half, read off the stamp in the root
    // this process serves (`assets::check_build_stamp`). A `dist` with no
    // stamp, built before it, starts.
    if let Err(e) = yokohama_web::assets::check_build_stamp(&yokohama_web::assets::dist_dir(), identity) {
        eprintln!("{e}");
        std::process::exit(1);
    }

    // **Every chat variable HAS a default (R-6.4.6, R-6.4.8)**, which is the
    // one way this differs from the identity mode above: a missing switch is
    // not a missing door. A BAD value still stops the process, and names the
    // variable — a server that started with an unreadable retention number
    // silently meaning 500 would be a server whose operator believes a number
    // that is not in force.
    let chat = match yokohama_web::chat::ChatConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // **The multiplayer switch** (M6.5b2, R-6.5b2.4), read here beside the
    // identity door and the chat facts. It reads like the chat variables
    // rather than like the identity mode: absent is `on`, so every
    // deployment that ran before this cycle goes on running unchanged, and a
    // bad value stops the process naming the variable. Off, the game, chat and
    // stream routes are not served at all — not gated, not hidden — so a
    // single-player deployment has nothing to probe and no Access application
    // to forget.
    let multiplayer = match yokohama_web::Multiplayer::from_env() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // **The sweep's two TTLs** (M6.5c2 cycle F, R-F9): the Worker's names and
    // meanings, unset meaning no sweep, a bad value a refusal naming the
    // variable. Read as the archive directory is (`var_os`), so a value that
    // is not UTF-8 is refused by name rather than read as unset (M6.5c2 cycle
    // D, row 130).
    let ttls = match yokohama_web::sweep::Ttls::from_lookup(|k| {
        std::env::var_os(k).map(|v| v.to_string_lossy().into_owned())
    }) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // The archive directory is the third variable read here (M6.5a, R-6.5a.9):
    // unset is no archive, a directory that cannot be created is a refusal to
    // start that names the variable — and so is a table TTL without one.
    let state = match AppState::new(identity, chat, multiplayer, ttls) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    // **No header set** (security S2a, decision D-1): a dist without `headers.json` — a client build older than
    // 0.6.2, or none — still serves, and says so once here, where the layer's own prints do not go.
    if state.headers.is_empty() {
        eprintln!(
            "{} has no {} — no security header is served; the client build is older than 0.6.2 or absent",
            state.dist.display(),
            yokohama_web::assets::HEADERS_FILE
        );
    }
    // Hourly, the first pass a minute after start (spec §2.4).
    if state.ttls.table.is_some() {
        tokio::spawn(yokohama_web::sweep::run(
            state.clone(),
            state.ttls,
            Duration::from_secs(3600),
            Duration::from_secs(60),
        ));
    }
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
