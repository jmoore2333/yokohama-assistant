//! The smallest slice of HTTP/1.1 the extension needs.
//!
//! Three verbs on two routes and a CORS preflight — that is the entire
//! surface, so this is a hand-rolled parser rather than a dependency. It reads
//! a request line, the headers it cares about, and a `Content-Length` body;
//! there is no chunked encoding, no keep-alive, and no request pipelining,
//! because `fetch` from a service worker needs none of them.
//!
//! Everything here is deliberately strict: a request that does not parse gets
//! a 400, never a guess.

use std::io::{BufRead, BufReader, Read};

/// Header block ceiling. A browser preflight is a few hundred bytes; anything
/// near this is not the extension.
const MAX_HEADER_BYTES: usize = 16 * 1024;

/// Body ceiling. `gamedatas` measured ~30 KB and a full notification history
/// is larger, so this has real headroom — it exists to bound a runaway, not to
/// police normal captures.
pub const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// A parsed request, reduced to the four things the router looks at.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// Path only — any `?query` is split off and discarded, since no route
    /// here takes one.
    pub path: String,
    /// The `Origin` header, if the client sent one. Absent for `curl` and for
    /// the tests; present for anything a browser sends cross-origin.
    pub origin: Option<String>,
    pub body: Vec<u8>,
}

/// Why a request could not be turned into a [`Request`].
#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    /// Malformed request line, oversized headers, or a `Content-Length` that
    /// is not a number / is over [`MAX_BODY_BYTES`].
    Malformed(&'static str),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "i/o error reading request: {e}"),
            ParseError::Malformed(why) => write!(f, "malformed request: {why}"),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::Io(e)
    }
}

/// Read one request off a stream.
pub fn read_request<R: Read>(stream: R) -> Result<Request, ParseError> {
    let mut reader = BufReader::new(stream);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(ParseError::Malformed("no method"))?.to_string();
    let target = parts.next().ok_or(ParseError::Malformed("no request target"))?;
    // A request line is exactly three tokens. Enforcing that is not pedantry:
    // without it, `POST /capture/with space HTTP/1.1` silently truncates to the
    // target `/capture/with` and a capture lands under a table id nobody asked
    // for. A whitespace-bearing target is malformed, and it is answered as such.
    let version = parts.next().ok_or(ParseError::Malformed("no HTTP version"))?;
    if !version.starts_with("HTTP/") {
        return Err(ParseError::Malformed("request line does not end in an HTTP version"));
    }
    if parts.next().is_some() {
        return Err(ParseError::Malformed("request target contains whitespace"));
    }
    let path = target.split('?').next().unwrap_or(target).to_string();

    let mut origin = None;
    let mut content_length = 0usize;
    let mut header_bytes = request_line.len();
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        header_bytes += read;
        if header_bytes > MAX_HEADER_BYTES {
            return Err(ParseError::Malformed("header block too large"));
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(ParseError::Malformed("header line without a colon"));
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "origin" => origin = Some(value.to_string()),
            "content-length" => {
                content_length =
                    value.parse().map_err(|_| ParseError::Malformed("content-length is not a number"))?;
                if content_length > MAX_BODY_BYTES {
                    return Err(ParseError::Malformed("content-length over the body ceiling"));
                }
            }
            _ => {}
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(Request { method, path, origin, body })
}

/// A response, held as data so the router can be unit-tested without a socket.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub reason: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16, reason: &'static str) -> Self {
        Response { status, reason, headers: Vec::new(), body: Vec::new() }
    }

    /// A JSON response. Every route here answers in JSON, including the error
    /// routes — the extension logs the body verbatim when a capture is
    /// refused, so a human-readable `error` field is what shows up in the
    /// service worker console.
    pub fn json(status: u16, reason: &'static str, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        Response {
            status,
            reason,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body,
        }
    }

    /// A JSON error carrying one explanatory sentence.
    pub fn error(status: u16, reason: &'static str, message: &str) -> Self {
        let body = serde_json::json!({ "error": message });
        Self::json(status, reason, body.to_string())
    }

    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    /// Serialize to the wire. `Connection: close` is unconditional — the relay
    /// handles one request per connection, which keeps the read loop above
    /// honest about where a request ends.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = format!("HTTP/1.1 {} {}\r\n", self.status, self.reason).into_bytes();
        for (name, value) in &self.headers {
            out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        out.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        out.extend_from_slice(b"Connection: close\r\n\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_post_with_a_body_parses() {
        let raw = "POST /capture/42 HTTP/1.1\r\nHost: x\r\nOrigin: chrome-extension://aaa\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        let req = read_request(raw.as_bytes()).expect("parses");
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/capture/42");
        assert_eq!(req.origin.as_deref(), Some("chrome-extension://aaa"));
        assert_eq!(req.body, b"{\"a\":1}");
    }

    #[test]
    fn a_query_string_is_split_off_the_path() {
        let raw = "GET /health?probe=1 HTTP/1.1\r\n\r\n";
        assert_eq!(read_request(raw.as_bytes()).expect("parses").path, "/health");
    }

    #[test]
    fn header_names_are_matched_case_insensitively() {
        let raw = "POST /capture/42 HTTP/1.1\r\nORIGIN: chrome-extension://aaa\r\ncontent-length: 2\r\n\r\n{}";
        let req = read_request(raw.as_bytes()).expect("parses");
        assert_eq!(req.origin.as_deref(), Some("chrome-extension://aaa"));
        assert_eq!(req.body, b"{}");
    }

    #[test]
    fn a_content_length_over_the_ceiling_is_refused_before_any_allocation() {
        let raw = format!("POST /capture/42 HTTP/1.1\r\nContent-Length: {}\r\n\r\n", MAX_BODY_BYTES + 1);
        assert!(matches!(
            read_request(raw.as_bytes()),
            Err(ParseError::Malformed("content-length over the body ceiling"))
        ));
    }

    #[test]
    fn a_garbage_request_line_is_an_error_not_a_panic() {
        assert!(read_request("".as_bytes()).is_err());
        assert!(read_request("GET\r\n\r\n".as_bytes()).is_err());
        assert!(read_request("GET /health\r\n\r\n".as_bytes()).is_err(), "no version");
        assert!(read_request("GET /health JUNK\r\n\r\n".as_bytes()).is_err(), "bad version");
    }

    #[test]
    fn a_target_containing_whitespace_is_refused_rather_than_truncated() {
        // Truncation is the dangerous outcome: `/capture/with space` would
        // parse as the perfectly valid target `/capture/with` and store a
        // capture under a table id the caller never named.
        // Refused on the version check, since `space` is where the version
        // should be; the message differs from the four-token case below but the
        // outcome that matters is identical.
        let err = read_request("POST /capture/with space HTTP/1.1\r\n\r\n".as_bytes())
            .expect_err("whitespace target");
        assert!(matches!(err, ParseError::Malformed(_)), "{err}");

        let err = read_request("GET /health HTTP/1.1 trailing\r\n\r\n".as_bytes())
            .expect_err("trailing token");
        assert!(matches!(err, ParseError::Malformed("request target contains whitespace")), "{err}");
    }

    #[test]
    fn the_wire_format_carries_content_length_and_closes() {
        let bytes = Response::json(200, "OK", "{}").to_bytes();
        let text = String::from_utf8(bytes).expect("ascii");
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: application/json\r\n"));
        assert!(text.contains("Content-Length: 2\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with("\r\n\r\n{}"));
    }
}
