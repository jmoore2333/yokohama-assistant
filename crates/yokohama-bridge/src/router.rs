//! Routing: three requests are answered, everything else is a 404.
//!
//! - `GET  /health`            — "the relay is up, and here is where it writes"
//! - `POST /capture/<table>`   — append one item to that table's NDJSON file
//! - `OPTIONS /capture/<table>`— the CORS preflight Chrome sends before the POST
//!
//! The CORS rule is the security boundary that matters in a browser: the relay
//! grants a cross-origin read **only** to a `chrome-extension://` origin, and
//! refuses a POST outright if it carries any other `Origin`. A request with NO
//! `Origin` header is not a browser cross-origin request at all — that is
//! `curl` on the owner's own machine, which the setup sheet uses to verify the
//! relay, so it is allowed through.

use crate::capture::{self, CaptureError};
use crate::http::{Request, Response};
use crate::server::Relay;

/// Is this an origin we hand a CORS grant to?
///
/// Pinned at the SCHEME, not at a specific extension id, and that is a
/// deliberate limit rather than laziness: an extension loaded unpacked gets
/// its id from the load path, so it changes when the owner moves the folder or
/// loads it on a second machine, and an id-pinned relay would fail with a
/// CORS error that looks like a bug. The real containment is elsewhere and is
/// stronger — the listener is loopback-only, so the only pages that can reach
/// it at all are ones already running on this machine.
fn is_extension_origin(origin: &str) -> bool {
    match origin.strip_prefix("chrome-extension://") {
        // A non-empty host with no path: `chrome-extension://<id>` exactly.
        //
        // `is_ascii_graphic` excludes space, CR, LF, NUL and DEL, and it is
        // load-bearing rather than tidy: this value is REFLECTED into an
        // `Access-Control-Allow-Origin` response header, so a CR/LF inside it
        // would let the caller write headers of its own.
        Some(rest) => {
            !rest.is_empty() && !rest.contains('/') && rest.chars().all(|c| c.is_ascii_graphic())
        }
        None => false,
    }
}

/// Is this `Host` header one of ours?
///
/// The DNS-rebinding guard. An attacker's page at `evil.example`, whose DNS
/// answers `127.0.0.1`, reaches this listener with the browser making the
/// request — the loopback bind does not stop it, because the browser really is
/// on loopback. What gives it away is the `Host` header, which carries the name
/// the browser dialled rather than the address it reached.
///
/// A name is accepted only if it is literally `localhost`; anything else must
/// parse as a loopback IP, which no domain name does. An ABSENT `Host` is
/// allowed: a browser always sends one, so its absence means a local
/// non-browser client — the same posture as an absent `Origin`, and what the
/// setup sheet's `curl` verification relies on.
fn host_is_local(host: &str) -> bool {
    // Strip the port. An IPv6 literal is bracketed (`[::1]:8787`), so its
    // colons must not be mistaken for the port separator.
    let bare = match host.strip_prefix('[') {
        Some(rest) => match rest.split_once(']') {
            Some((inner, _)) => inner,
            None => return false,
        },
        None => host.split(':').next().unwrap_or(host),
    };
    bare.eq_ignore_ascii_case("localhost")
        || bare.parse::<std::net::IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false)
}

/// Attach the CORS grant when, and only when, the caller earned it.
fn with_cors(response: Response, origin: Option<&str>) -> Response {
    match origin {
        Some(o) if is_extension_origin(o) => response
            .header("Access-Control-Allow-Origin", o)
            // `Vary` because the grant depends on the request's Origin; without
            // it an intermediate cache could hand one origin's grant to another.
            .header("Vary", "Origin"),
        _ => response,
    }
}

/// Answer one request.
pub fn route(request: &Request, relay: &Relay) -> Response {
    let origin = request.origin.as_deref();

    // Before ANY route, including `/health` — which would otherwise confirm to
    // a rebound page that the relay exists and tell it where captures are kept.
    if let Some(host) = request.host.as_deref() {
        if !host_is_local(host) {
            return Response::error(
                403,
                "Forbidden",
                "this relay answers requests addressed to localhost only",
            );
        }
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => with_cors(health(relay), origin),

        ("OPTIONS", path) if path.starts_with("/capture/") => with_cors(
            Response::new(204, "No Content")
                .header("Access-Control-Allow-Methods", "POST, OPTIONS")
                .header("Access-Control-Allow-Headers", "Content-Type")
                // Ten minutes, so a follow session that posts every few seconds
                // pays for one preflight rather than hundreds.
                .header("Access-Control-Max-Age", "600"),
            origin,
        ),

        ("POST", path) if path.starts_with("/capture/") => {
            // Checked BEFORE the body is looked at, so a rejected origin never
            // gets a parse error that tells it anything about the format.
            if let Some(o) = origin {
                if !is_extension_origin(o) {
                    return Response::error(
                        403,
                        "Forbidden",
                        "this relay accepts captures from the bridge extension only",
                    );
                }
            }
            // No percent-decoding, on purpose: decoding is what would turn a
            // `%2F` back into the path separator the id check exists to exclude.
            let table_id = &path["/capture/".len()..];
            with_cors(capture_into(relay, table_id, &request.body), origin)
        }

        _ => Response::error(404, "Not Found", "no such route (try GET /health)"),
    }
}

fn health(relay: &Relay) -> Response {
    let body = serde_json::json!({
        "service": "yokohama-bridge-relay",
        "version": env!("CARGO_PKG_VERSION"),
        "dir": relay.dir.display().to_string(),
        "kinds": capture::KINDS,
    });
    Response::json(200, "OK", body.to_string())
}

fn capture_into(relay: &Relay, table_id: &str, body: &[u8]) -> Response {
    // Validate the id first: an id we would not write is not worth parsing a
    // 30 KB body for.
    if let Err(e) = capture::safe_table_id(table_id) {
        return Response::error(400, "Bad Request", &e.to_string());
    }
    let line = match capture::line_for(body, capture::now_ms()) {
        Ok(line) => line,
        Err(e) => return Response::error(400, "Bad Request", &e.to_string()),
    };
    let bytes = line.len();
    match relay.append(table_id, &line) {
        Ok(file_bytes) => {
            println!("capture table={table_id} +{bytes} B (file now {file_bytes} B)");
            let body = serde_json::json!({
                "stored": true,
                "table": table_id,
                "bytes": bytes,
                "file_bytes": file_bytes,
            });
            Response::json(200, "OK", body.to_string())
        }
        Err(e @ CaptureError::UnsafeTableId(_)) => {
            Response::error(400, "Bad Request", &e.to_string())
        }
        Err(e) => {
            // A failed append is the one thing the owner must not miss: the
            // capture is gone unless the extension retries, so say so loudly on
            // the console as well as in the response the extension logs.
            eprintln!("CAPTURE FAILED table={table_id}: {e}");
            Response::error(500, "Internal Server Error", &e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, path: &str, origin: Option<&str>, body: &str) -> Request {
        Request {
            method: method.to_string(),
            path: path.to_string(),
            origin: origin.map(str::to_string),
            host: None,
            body: body.as_bytes().to_vec(),
        }
    }

    fn req_from_host(host: Option<&str>, path: &str) -> Request {
        Request {
            method: "GET".to_string(),
            path: path.to_string(),
            origin: None,
            host: host.map(str::to_string),
            body: Vec::new(),
        }
    }

    fn relay() -> (Relay, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "yokohama-bridge-router-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        (Relay::new(dir.clone()), dir)
    }

    fn header_value(response: &Response, name: &str) -> Option<String> {
        response
            .headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn only_the_extension_scheme_earns_a_cors_grant() {
        assert!(is_extension_origin("chrome-extension://abcdefghijklmnopabcdefghijklmnop"));
        for hostile in [
            "https://boardgamearena.com",
            "http://127.0.0.1:8787",
            "chrome-extension://",
            "chrome-extension://id/path",
            "moz-extension://abc",
            "null",
            "",
        ] {
            assert!(!is_extension_origin(hostile), "{hostile:?} must not earn a grant");
        }
    }

    #[test]
    fn health_reports_the_directory_it_would_write_to() {
        let (relay, dir) = relay();
        let response = route(&req("GET", "/health", None, ""), &relay);
        assert_eq!(response.status, 200);
        let body: serde_json::Value =
            serde_json::from_slice(&response.body).expect("health body is JSON");
        assert_eq!(body["service"], "yokohama-bridge-relay");
        assert_eq!(body["dir"], dir.display().to_string());
        assert!(
            !dir.exists(),
            "a health probe must not create the capture directory as a side effect"
        );
    }

    #[test]
    fn a_capture_from_the_extension_is_stored_and_granted() {
        let (relay, dir) = relay();
        let response = route(
            &req(
                "POST",
                "/capture/895608640",
                Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
                r#"{"kind":"snapshot","payload":{"ok":true}}"#,
            ),
            &relay,
        );
        assert_eq!(response.status, 200);
        assert_eq!(
            header_value(&response, "access-control-allow-origin").as_deref(),
            Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop")
        );
        assert_eq!(header_value(&response, "vary").as_deref(), Some("Origin"));
        assert!(capture::capture_path(&dir, "895608640").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_capture_with_no_origin_header_is_allowed_but_granted_nothing() {
        // This is the `curl` path the setup sheet uses to verify the relay.
        let (relay, dir) = relay();
        let response = route(
            &req("POST", "/capture/895608640", None, r#"{"kind":"packets","payload":[]}"#),
            &relay,
        );
        assert_eq!(response.status, 200);
        assert!(header_value(&response, "access-control-allow-origin").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_foreign_origin_is_refused_before_the_body_is_parsed_or_stored() {
        let (relay, dir) = relay();
        // Body is deliberately valid: the refusal must come from the origin, so
        // a passing test here cannot be explained by the body being bad.
        let response = route(
            &req(
                "POST",
                "/capture/895608640",
                Some("https://boardgamearena.com"),
                r#"{"kind":"snapshot","payload":{}}"#,
            ),
            &relay,
        );
        assert_eq!(response.status, 403);
        assert!(!dir.exists(), "nothing was written");
    }

    #[test]
    fn the_preflight_answers_204_with_the_method_and_header_the_post_needs() {
        let (relay, _dir) = relay();
        let response = route(
            &req(
                "OPTIONS",
                "/capture/895608640",
                Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
                "",
            ),
            &relay,
        );
        assert_eq!(response.status, 204);
        assert!(response.body.is_empty(), "204 carries no body");
        assert_eq!(
            header_value(&response, "access-control-allow-methods").as_deref(),
            Some("POST, OPTIONS")
        );
        assert_eq!(
            header_value(&response, "access-control-allow-headers").as_deref(),
            Some("Content-Type")
        );
    }

    #[test]
    fn an_origin_carrying_control_characters_is_refused() {
        // These would be REFLECTED into a response header if accepted, so a CR
        // or LF is header injection and a NUL is a parser surprise. The scheme
        // check alone does not exclude them.
        for hostile in [
            "chrome-extension://abc\r\nX-Injected: yes",
            "chrome-extension://abc\ndrop",
            "chrome-extension://abc\0",
            "chrome-extension://ab c",
            "chrome-extension://abc\t",
        ] {
            assert!(!is_extension_origin(hostile), "{hostile:?} must not earn a grant");
            let response = route(
                &req("POST", "/capture/895608640", Some(hostile), r#"{"kind":"snapshot","payload":{}}"#),
                &relay().0,
            );
            assert_eq!(response.status, 403, "and the request itself is refused");
            assert!(
                response
                    .headers
                    .iter()
                    .all(|(n, _)| !n.eq_ignore_ascii_case("access-control-allow-origin")),
                "nothing hostile is ever echoed back into a header"
            );
        }
    }

    #[test]
    fn a_foreign_host_header_is_refused() {
        // DNS rebinding: a page on the attacker's domain, resolved to
        // 127.0.0.1, reaches this listener with the browser's own credentials.
        // The Origin check covers the capture route; this covers every route,
        // including the /health read that would otherwise confirm the relay
        // exists and name the owner's capture directory.
        for hostile in ["evil.example", "evil.example:8787", "boardgamearena.com"] {
            for path in ["/health", "/capture/895608640"] {
                let response = route(&req_from_host(Some(hostile), path), &relay().0);
                assert_eq!(response.status, 403, "Host {hostile:?} on {path}");
            }
        }
    }

    #[test]
    fn loopback_hosts_and_an_absent_host_are_allowed() {
        // A browser always sends Host, so its ABSENCE is a local non-browser
        // client - the same posture as an absent Origin, and the one the setup
        // sheet's `curl` verification relies on.
        for ok in [
            Some("127.0.0.1"),
            Some("127.0.0.1:8787"),
            Some("localhost"),
            Some("localhost:8787"),
            Some("[::1]:8787"),
            Some("[::1]"),
            None,
        ] {
            let response = route(&req_from_host(ok, "/health"), &relay().0);
            assert_eq!(response.status, 200, "Host {ok:?} should be served");
        }
    }

    #[test]
    fn unrouted_requests_are_404() {
        let (relay, _dir) = relay();
        for (method, path) in [("GET", "/"), ("GET", "/capture/1"), ("POST", "/health"), ("PUT", "/capture/1")] {
            assert_eq!(route(&req(method, path, None, ""), &relay).status, 404, "{method} {path}");
        }
    }
}
