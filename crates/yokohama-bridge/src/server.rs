//! The listener: accept, read one request, answer, close.
//!
//! A thread per connection out of `std` rather than an async runtime. The load
//! is one owner's browser tab posting a capture every few seconds, so the
//! interesting property is not throughput — it is that a stalled or hostile
//! connection cannot wedge the relay, which the read/write timeouts below buy
//! outright.

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::capture::{self, CaptureError};
use crate::http;
use crate::router;

/// How long a single connection may take to deliver its request or accept its
/// answer. Generous for a 30 KB snapshot on loopback, short enough that a
/// half-open connection releases its thread.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(30);

/// Shared relay state: where captures go, and the lock that keeps concurrent
/// appends from interleaving into unparseable lines.
#[derive(Debug)]
pub struct Relay {
    pub dir: PathBuf,
    append_lock: Mutex<()>,
}

impl Relay {
    pub fn new(dir: PathBuf) -> Self {
        Relay { dir, append_lock: Mutex::new(()) }
    }

    /// Append one already-built line to a table's capture file.
    ///
    /// The lock is held across the whole append, and the H0 review measured
    /// what it is actually worth: with the mutex REMOVED, single-`write_all`
    /// `O_APPEND` appends on macOS/APFS stayed atomic from 40 KB all the way to
    /// 32 MiB, so on this machine the lock changes nothing. It stays because
    /// that atomicity is a property of the filesystem, not of POSIX — NFS in
    /// particular does not promise it — and the failure it prevents is silent
    /// corruption of the one artifact the whole phase is built on.
    ///
    /// Read `concurrent_appends_never_interleave` accordingly: it pins the
    /// OUTCOME (16 concurrent appends, 16 parseable lines) and does not
    /// discriminate the lock, since it passes without it here.
    pub fn append(&self, table_id: &str, line: &str) -> Result<u64, CaptureError> {
        let _guard = self.append_lock.lock().unwrap_or_else(|poisoned| {
            // A panic in another handler must not take the capture path down
            // with it: the lock guards file ordering, not any invariant that a
            // panic could have left broken.
            poisoned.into_inner()
        });
        capture::append(&self.dir, table_id, line)
    }
}

/// Accept forever. Returns only if the listener itself dies.
pub fn serve(listener: TcpListener, relay: Arc<Relay>) {
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let relay = Arc::clone(&relay);
                let spawned = std::thread::Builder::new()
                    .name("bridge-conn".into())
                    .spawn(move || handle(stream, &relay));
                if let Err(e) = spawned {
                    // A failed spawn (thread limit) must not kill the accept
                    // loop. The connection dies with the closure and the
                    // extension re-sends from its outbox, so this is a loud log
                    // rather than an abort.
                    eprintln!("could not spawn a handler thread ({e}); connection dropped");
                }
            }
            Err(e) => eprintln!("accept failed: {e}"),
        }
    }
}

/// Read one request off `stream`, answer it, and close.
pub fn handle(stream: TcpStream, relay: &Relay) {
    let _ = stream.set_read_timeout(Some(CONNECTION_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CONNECTION_TIMEOUT));

    let response = match http::read_request(&stream) {
        Ok(request) => router::route(&request, relay),
        Err(e) => http::Response::error(400, "Bad Request", &e.to_string()),
    };

    let mut stream = stream;
    if let Err(e) = stream.write_all(&response.to_bytes()) {
        eprintln!("could not write response: {e}");
    }
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_appends_never_interleave() {
        // Sixteen threads writing long lines at once produce sixteen parseable
        // lines. That is the OUTCOME this pins, and it is worth stating what it
        // does NOT pin: the H0 review removed the mutex and this still passed,
        // because macOS/APFS keeps a single-`write_all` `O_APPEND` write atomic
        // at these sizes. So a green run here is evidence about the file, not
        // proof the lock is load-bearing — see `Relay::append` for why it stays.
        let dir = std::env::temp_dir()
            .join(format!("yokohama-bridge-concurrent-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let relay = Arc::new(Relay::new(dir.clone()));

        let threads: Vec<_> = (0..16)
            .map(|i| {
                let relay = Arc::clone(&relay);
                std::thread::spawn(move || {
                    let payload = "x".repeat(40_000);
                    let line =
                        format!(r#"{{"received_at_ms":{i},"kind":"packets","payload":"{payload}"}}"#);
                    relay.append("895608640", &line).expect("append");
                })
            })
            .collect();
        for t in threads {
            t.join().expect("thread joins");
        }

        let text = std::fs::read_to_string(capture::capture_path(&dir, "895608640")).expect("read");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 16);
        for line in lines {
            let value: serde_json::Value =
                serde_json::from_str(line).expect("each line parses on its own");
            assert_eq!(value["payload"].as_str().map(str::len), Some(40_000));
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
