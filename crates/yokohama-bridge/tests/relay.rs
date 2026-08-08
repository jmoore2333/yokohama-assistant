//! End-to-end tests for the relay, driven over a real loopback socket.
//!
//! The unit tests inside the modules cover the pure parts (routing decisions,
//! the bind guard, the NDJSON envelope). These cover the thing those cannot:
//! that the bytes on the wire are an HTTP response a browser will accept, and
//! that a capture that crossed a socket lands in the file intact.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use yokohama_bridge::bind;
use yokohama_bridge::server::{self, Relay};

/// A running relay plus the throwaway directory it captures into.
struct Harness {
    addr: SocketAddr,
    dir: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn start() -> Harness {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let unique = format!(
        "yokohama-bridge-test-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&dir).expect("temp capture dir");

    // Port 0 = let the OS pick, so tests never collide with each other or with
    // a relay the owner happens to be running.
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral loopback port");
    let addr = listener.local_addr().expect("local addr");
    let relay = Arc::new(Relay::new(dir.clone()));
    std::thread::spawn(move || server::serve(listener, relay));

    Harness { addr, dir }
}

/// Send one raw HTTP request and read the whole response back.
fn request(addr: SocketAddr, raw: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect to relay");
    stream.write_all(raw.as_bytes()).expect("write request");
    stream.flush().expect("flush request");
    let mut out = String::new();
    stream.read_to_string(&mut out).expect("read response");
    out
}

fn post(addr: SocketAddr, path: &str, origin: Option<&str>, body: &str) -> String {
    let origin_header = origin.map(|o| format!("Origin: {o}\r\n")).unwrap_or_default();
    request(
        addr,
        &format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n{origin_header}Content-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    )
}

fn status_line(response: &str) -> &str {
    response.lines().next().unwrap_or("")
}

fn header(response: &str, name: &str) -> Option<String> {
    let needle = format!("{}:", name.to_ascii_lowercase());
    response
        .lines()
        .take_while(|line| !line.is_empty())
        .find(|line| line.to_ascii_lowercase().starts_with(&needle))
        .map(|line| line[needle.len()..].trim().to_string())
}

fn body_of(response: &str) -> &str {
    response.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("")
}

fn capture_lines(harness: &Harness, table: &str) -> Vec<serde_json::Value> {
    let path = harness.dir.join(format!("{table}.ndjson"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read capture file {}: {e}", path.display()));
    text.lines()
        .map(|line| serde_json::from_str(line).expect("each NDJSON line parses on its own"))
        .collect()
}

#[test]
fn health_answers_with_the_service_identity_and_the_capture_directory() {
    let h = start();
    let response = request(
        h.addr,
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    );

    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "health should be 200, got: {}",
        status_line(&response)
    );
    let body: serde_json::Value = serde_json::from_str(body_of(&response)).expect("health is JSON");
    assert_eq!(body["service"], "yokohama-bridge-relay");
    assert_eq!(
        body["dir"],
        serde_json::Value::String(h.dir.display().to_string()),
        "the extension's health probe is also how the owner confirms WHERE captures land"
    );
}

#[test]
fn two_posts_append_two_parseable_ndjson_lines_in_order() {
    let h = start();

    let first = post(
        h.addr,
        "/capture/895608640",
        Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
        r#"{"kind":"snapshot","payload":{"marker":"one","gamedatas":{"players":{}}}}"#,
    );
    let second = post(
        h.addr,
        "/capture/895608640",
        Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
        r#"{"kind":"packets","payload":{"marker":"two","body_text":"{\"logs\":[]}"}}"#,
    );

    assert!(status_line(&first).starts_with("HTTP/1.1 200"));
    assert!(status_line(&second).starts_with("HTTP/1.1 200"));

    let lines = capture_lines(&h, "895608640");
    assert_eq!(lines.len(), 2, "one line per POST");

    assert_eq!(lines[0]["kind"], "snapshot");
    assert_eq!(lines[0]["payload"]["marker"], "one");
    assert_eq!(lines[1]["kind"], "packets");
    assert_eq!(lines[1]["payload"]["marker"], "two");

    // The raw-truth guarantee the design leans on: a notification body that was
    // shipped as a verbatim string survives the round trip as that same string.
    assert_eq!(lines[1]["payload"]["body_text"], r#"{"logs":[]}"#);

    for line in &lines {
        let stamp = line["received_at_ms"]
            .as_u64()
            .expect("every line carries a receive stamp");
        assert!(stamp > 1_700_000_000_000, "stamp looks like epoch millis");
    }
    assert!(
        lines[0]["received_at_ms"].as_u64() <= lines[1]["received_at_ms"].as_u64(),
        "stamps are non-decreasing in file order"
    );
}

#[test]
fn each_table_gets_its_own_capture_file() {
    let h = start();
    let body = r#"{"kind":"snapshot","payload":{}}"#;
    post(h.addr, "/capture/895608640", None, body);
    post(h.addr, "/capture/111222333", None, body);

    assert_eq!(capture_lines(&h, "895608640").len(), 1);
    assert_eq!(capture_lines(&h, "111222333").len(), 1);
}

#[test]
fn a_preflight_from_a_chrome_extension_origin_is_greenlit_for_the_capture_route() {
    let h = start();
    let response = request(
        h.addr,
        "OPTIONS /capture/895608640 HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: chrome-extension://abcdefghijklmnopabcdefghijklmnop\r\nAccess-Control-Request-Method: POST\r\n\r\n",
    );

    assert!(
        status_line(&response).starts_with("HTTP/1.1 204"),
        "preflight should be 204, got: {}",
        status_line(&response)
    );
    assert_eq!(
        header(&response, "Access-Control-Allow-Origin").as_deref(),
        Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
        "the allowed origin is REFLECTED, not a wildcard"
    );
    let methods = header(&response, "Access-Control-Allow-Methods").unwrap_or_default();
    assert!(methods.contains("POST"), "methods header: {methods}");
    let headers = header(&response, "Access-Control-Allow-Headers").unwrap_or_default();
    assert!(
        headers.to_ascii_lowercase().contains("content-type"),
        "the POST sends application/json, so content-type must be allowed: {headers}"
    );
}

#[test]
fn a_successful_capture_reflects_the_extension_origin_back() {
    let h = start();
    let response = post(
        h.addr,
        "/capture/895608640",
        Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
        r#"{"kind":"snapshot","payload":{}}"#,
    );
    assert_eq!(
        header(&response, "Access-Control-Allow-Origin").as_deref(),
        Some("chrome-extension://abcdefghijklmnopabcdefghijklmnop"),
        "without this the service worker cannot read its own 200 and would retry forever"
    );
}

#[test]
fn a_post_from_a_non_extension_origin_is_refused_and_appends_nothing() {
    let h = start();
    let response = post(
        h.addr,
        "/capture/895608640",
        Some("https://evil.example"),
        r#"{"kind":"snapshot","payload":{"marker":"should not land"}}"#,
    );

    assert!(
        status_line(&response).starts_with("HTTP/1.1 403"),
        "a browser origin that is not this extension is refused outright, got: {}",
        status_line(&response)
    );
    assert!(
        header(&response, "Access-Control-Allow-Origin").is_none(),
        "and it is never handed a CORS grant"
    );
    assert!(
        !h.dir.join("895608640.ndjson").exists(),
        "a refused capture must not create or touch the capture file"
    );
}

#[test]
fn a_malformed_body_is_refused_and_appends_nothing() {
    let h = start();
    let response = post(h.addr, "/capture/895608640", None, "{not json at all");

    assert!(
        status_line(&response).starts_with("HTTP/1.1 400"),
        "got: {}",
        status_line(&response)
    );
    assert!(
        !h.dir.join("895608640.ndjson").exists(),
        "an unparseable body must never reach the file - one bad line would break every \
         downstream NDJSON reader"
    );
}

#[test]
fn a_body_without_a_known_kind_is_refused_and_appends_nothing() {
    let h = start();
    let response = post(
        h.addr,
        "/capture/895608640",
        None,
        r#"{"kind":"whatever","payload":{}}"#,
    );
    assert!(
        status_line(&response).starts_with("HTTP/1.1 400"),
        "got: {}",
        status_line(&response)
    );
    assert!(!h.dir.join("895608640.ndjson").exists());
}

#[test]
fn a_table_id_that_is_not_a_safe_filename_component_is_refused() {
    let h = start();
    for hostile in ["../../etc/passwd", "table%2Fslash", "with space", ""] {
        let response = post(
            h.addr,
            &format!("/capture/{hostile}"),
            None,
            r#"{"kind":"snapshot","payload":{}}"#,
        );
        let line = status_line(&response);
        assert!(
            line.starts_with("HTTP/1.1 400") || line.starts_with("HTTP/1.1 404"),
            "table id {hostile:?} must not be accepted, got: {line}"
        );
    }
    // Nothing escaped the capture directory, and nothing was created inside it.
    let entries: Vec<_> = std::fs::read_dir(&h.dir)
        .expect("capture dir")
        .map(|e| e.expect("dir entry").file_name())
        .collect();
    assert!(entries.is_empty(), "capture dir should be untouched: {entries:?}");
}

#[test]
fn an_unknown_route_is_a_404() {
    let h = start();
    let response = request(h.addr, "GET /whatever HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    assert!(
        status_line(&response).starts_with("HTTP/1.1 404"),
        "got: {}",
        status_line(&response)
    );
}

#[test]
fn the_relay_refuses_to_bind_anywhere_but_loopback() {
    // The whole security posture is "this listener is not reachable from the
    // network". It is enforced here rather than by remembering to pass the
    // right flag.
    for hostile in ["0.0.0.0", "::", "192.168.1.50", "8.8.8.8"] {
        assert!(
            bind::resolve(hostile, 8787).is_err(),
            "{hostile} must be refused as a bind address"
        );
    }
    for ok in ["127.0.0.1", "::1", "localhost"] {
        let addr = bind::resolve(ok, 8787).unwrap_or_else(|e| panic!("{ok} should bind: {e}"));
        assert!(addr.ip().is_loopback());
        assert_eq!(addr.port(), 8787);
    }
}
