#![cfg(feature = "server")]
//! **The self-host's keyed-build start refusal, run on the binary itself**
//! (M6.5c2 cycle F, Task 10.E; the mutation battery's finding F-1).
//!
//! `assets::keyed_build_refusal` decides and `assets::check_build_stamp`
//! reads, and unit cells beside them pin both. The one call in `main` that
//! turns the decision into an exit 1 was run by no cell: the battery deleted
//! it and every test stayed green. These cells spawn the built server
//! (`CARGO_BIN_EXE_yokohama-web`, the way `yokohama-cli`'s `piped_dogfood`
//! spawns the CLI) over a scratch `dist` and read what the PROCESS does:
//!
//! - a stamp saying the client was built with a Turnstile site key, behind
//!   `cf-access` — the process exits 1 before it listens, and its stderr is
//!   the pure rule's own sentence for the same inputs (the catcher);
//! - the same stamp at the `open` door — the process starts (a control: what
//!   the catcher reads is the stamp under a door, not the stamp alone);
//! - no stamp at all, behind `cf-access` — the process starts and says
//!   nothing about a stamp (a control: a `dist` built before the stamp runs).
//!
//! And since security S1a (finding B3; R-SEC8(b); the review's I-1), the seed
//! rule on the same path: a creator's seed beside an open seat, posted to the
//! running binary, is the 400 in `SEED_NOT_SOLO`'s words with no table stored,
//! and the same body unseeded a 201 — `main` → `AppState::new` → `production`,
//! the NAS's own constructor path, which no in-process cell runs.
//!
//! And since security S1c (finding D26; the fix round's Minor 3), the
//! `segment` subcommand the deploy window's rename reads its table from: the
//! binary run as `yokohama-web segment <address>…`, which prints and exits
//! before any start, so it takes no door and no scratch root. A dropped column
//! or two swapped ones would rename every folder wrongly, and nothing else
//! runs the subcommand.
//!
//! And since security S2a (decision D-1), the header set on the seed rule's
//! path: `production` reads the scratch `dist`'s `headers.json`,
//! so the running binary answers `/` and `/api/health` with its pairs, and
//! `no-store` beside them under `/api`; a `dist` with no file starts and
//! says so on stderr; a malformed file exits 1 before it listens, naming
//! the file.
//!
//! And since the S2 stack review (lens 2, Important 1), the stream cap's slot
//! on the binary: a caller's eight table streams held open over real sockets,
//! each read past its first frame, make the ninth a 429 in
//! `TOO_MANY_STREAMS_CALLER`'s words, and closing one socket admits the next.
//! The slot rides the SSE body, and only a body hyper polls and drops proves
//! it: the in-process cells hold an unpolled response, so a slot released at
//! the body's first poll passed every one of them.
//!
//! **Every wait is bounded.** Both pipes are read on their own threads into
//! channels; a start is polled every 50 ms for its listening line or its exit,
//! up to [`BOUND`], and a reaped child's pipes get [`DRAIN`] to close. A
//! [`Server`] kills and reaps its child when dropped, so a cell that fails
//! mid-wait leaves no process listening behind it.
//!
//! **The child's environment is the cell's own.** Every inherited `YOKOHAMA_`
//! variable is removed from the child, so a developer's shell cannot change
//! what start reads, and only what a start needs is set: the door, the scratch
//! `dist`, a store directory under the scratch root, and port 0 — the OS picks
//! a port nobody holds at the moment of the bind, so the cells run in parallel
//! with each other and on a busy machine without a collision (the listening
//! line names the configured port, so it reads `:0`). The two cells that send
//! the server a request each bind a port found free a moment before
//! ([`free_port`]), since a child bound to 0 cannot say where it listens.
//! `Command::env` sets the CHILD's environment, never this process's, so
//! nothing here mutates the test runner's environment
//! (`identity::from_env_value`'s rule).

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use yokohama_web::archive::{caller_segment, legacy_caller_segment};
use yokohama_web::assets::{keyed_build_refusal, Stamp, BUILD_STAMP};
use yokohama_web::identity::{IdentityMode, User, ENV_VAR as IDENTITY_VAR};

/// How long a start may take to show what it does: exit, or listen.
const BOUND: Duration = Duration::from_secs(20);

/// How long a reaped child's pipes may take to deliver the rest and close.
const DRAIN: Duration = Duration::from_secs(5);

/// The start of the line `main` prints once it has bound its port, and only
/// then.
const LISTENING: &str = "yokohama-web listening on ";

/// A stamp as `web/vite.config.ts` writes it, all six build values `true`:
/// the client was built with a Turnstile site key.
const KEYED_STAMP: &str = r#"{
  "project_url": true,
  "blurb": true,
  "notice": true,
  "release_feed": true,
  "site_url": true,
  "turnstile_sitekey": true
}
"#;

/// A scratch root under the target directory's test scratch area, unique per
/// cell and run, holding an empty `dist`. Removed when dropped — even when the
/// cell panics — and only ever this root, which the cell created.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(cell: &str) -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("start-refusal-{cell}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(root.join("dist")).expect("a scratch dist");
        Self { root }
    }

    fn dist(&self) -> PathBuf {
        self.root.join("dist")
    }

    /// The stamp's path as `check_build_stamp` shows it in its sentence: the
    /// served root joined with the stamp's name, displayed.
    fn stamp_shown(&self) -> String {
        self.dist().join(BUILD_STAMP).display().to_string()
    }

    fn write_stamp(&self, text: &str) {
        std::fs::write(self.dist().join(BUILD_STAMP), text).expect("the stamp writes");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Every line `pipe` carries, newline included, on a channel that disconnects
/// when the pipe closes.
fn lines_of(pipe: impl Read + Send + 'static) -> Receiver<String> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        loop {
            let mut line = Vec::new();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(String::from_utf8_lossy(&line).into_owned()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    rx
}

/// The rest of what a closed pipe carried, read until its channel
/// disconnects — bounded, since a pipe that never closes belongs to a process
/// that never went away.
fn drain(pipe: &Receiver<String>, deadline: Instant) -> String {
    let mut text = String::new();
    loop {
        match pipe.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(line) => text.push_str(&line),
            Err(RecvTimeoutError::Disconnected) => return text,
            Err(RecvTimeoutError::Timeout) => {
                panic!("a pipe of the reaped server stayed open past {DRAIN:?}; it carried {text:?}")
            }
        }
    }
}

/// What a start showed within [`BOUND`].
enum Start {
    /// The process exited, with this status.
    Exited(ExitStatus),
    /// The process printed its listening line (trimmed) and was running.
    Listening(String),
    /// Neither, by the bound.
    Silent,
}

/// The built binary with every inherited `YOKOHAMA_` variable removed, so a
/// developer's shell cannot change what it reads.
fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_yokohama-web"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("YOKOHAMA_") {
            command.env_remove(&key);
        }
    }
    command
}

/// The built server, spawned over a scratch root behind one door; killed and
/// reaped when dropped.
struct Server {
    child: Child,
    stdout: Receiver<String>,
    stderr: Receiver<String>,
    /// What stdout carried while [`Server::start`] watched it.
    printed: String,
}

impl Server {
    fn spawn(door: &str, scratch: &Scratch) -> Self {
        Self::spawn_on(door, scratch, 0)
    }

    /// [`Server::spawn`] bound to `port`: 0 lets the OS pick, and the
    /// listening line then reads `:0`, which is all a cell that never reaches
    /// the server needs.
    fn spawn_on(door: &str, scratch: &Scratch, port: u16) -> Self {
        let mut command = binary();
        command
            .env(IDENTITY_VAR, door)
            .env("YOKOHAMA_WEB_DIST", scratch.dist())
            .env("YOKOHAMA_WEB_DIR", scratch.root.join("games"))
            .env("YOKOHAMA_WEB_PORT", port.to_string());
        Self::piped(command)
    }

    /// The binary as `yokohama-web segment <addresses>…` (security S1c,
    /// finding D26): a one-shot that prints and exits, so no door is set.
    fn segment(addresses: &[&str]) -> Self {
        let mut command = binary();
        command.arg("segment").args(addresses);
        Self::piped(command)
    }

    /// `command` spawned with stdin closed and both pipes read on their own
    /// threads.
    fn piped(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the server binary spawns");
        let stdout = lines_of(child.stdout.take().expect("stdout is piped"));
        let stderr = lines_of(child.stderr.take().expect("stderr is piped"));
        Self { child, stdout, stderr, printed: String::new() }
    }

    /// Poll every 50 ms, up to [`BOUND`], for the listening line or an exit.
    fn start(&mut self) -> Start {
        let deadline = Instant::now() + BOUND;
        loop {
            while let Ok(line) = self.stdout.try_recv() {
                self.printed.push_str(&line);
                if line.starts_with(LISTENING) {
                    return Start::Listening(line.trim_end().to_string());
                }
            }
            if let Some(status) = self.child.try_wait().expect("the child's status reads") {
                return Start::Exited(status);
            }
            if Instant::now() >= deadline {
                return Start::Silent;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Whether the process has not exited.
    fn running(&mut self) -> bool {
        self.child.try_wait().expect("the child's status reads").is_none()
    }

    /// Kill the process if it still runs, reap it, and return its status and
    /// everything each pipe carried.
    fn finish(&mut self) -> (ExitStatus, String, String) {
        let _ = self.child.kill();
        let status = self.child.wait().expect("the child is reaped");
        let deadline = Instant::now() + DRAIN;
        let mut stdout = std::mem::take(&mut self.printed);
        stdout.push_str(&drain(&self.stdout, deadline));
        (status, stdout, drain(&self.stderr, deadline))
    }

    /// The status and both pipes of a process that exits by itself within
    /// [`BOUND`], or a failure naming what it did instead.
    fn exited(&mut self, what: &str) -> (ExitStatus, String, String) {
        match self.start() {
            Start::Exited(_) => self.finish(),
            Start::Listening(line) => {
                let (_, stdout, stderr) = self.finish();
                panic!("{what}: the binary listened ({line}) instead of exiting; stdout {stdout:?}, stderr {stderr:?}")
            }
            Start::Silent => {
                let (_, stdout, stderr) = self.finish();
                panic!("{what}: the binary did not exit within {BOUND:?}; stdout {stdout:?}, stderr {stderr:?}")
            }
        }
    }

    /// The listening line, or a failure naming what the process did instead.
    fn listening(&mut self, what: &str) -> String {
        match self.start() {
            Start::Listening(line) => line,
            Start::Exited(status) => {
                let (_, stdout, stderr) = self.finish();
                panic!("{what}: the server exited ({status}) instead of listening; stdout {stdout:?}, stderr {stderr:?}")
            }
            Start::Silent => {
                let (_, stdout, stderr) = self.finish();
                panic!(
                    "{what}: the server neither listened nor exited within {BOUND:?}; stdout {stdout:?}, stderr \
                     {stderr:?}"
                )
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A port nobody held a moment ago: the OS's pick for a listener bound to
/// port 0, released at once. Another process taking it in between fails the
/// start loudly — the bind's error and an exit — never silently.
#[cfg(feature = "open-door")]
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).expect("a free port").port()
}

/// One `POST` to the server at `port` as `user` at the open door (the claimed
/// header), over a plain socket — no client crate — with `Connection: close`,
/// so the response is everything until the server closes; each read and write
/// bounded by [`BOUND`]. The status, and the body parsed as JSON (`Null` when
/// it is not).
#[cfg(feature = "open-door")]
fn post(port: u16, user: &str, path: &str, body: &serde_json::Value) -> (u16, serde_json::Value) {
    use std::io::Write;
    use yokohama_web::identity::CLAIMED_HEADER;
    let body = body.to_string();
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server accepts");
    stream.set_read_timeout(Some(BOUND)).expect("a read bound");
    stream.set_write_timeout(Some(BOUND)).expect("a write bound");
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{CLAIMED_HEADER}: {user}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("the request writes");
    let mut response = String::new();
    stream.read_to_string(&mut response).expect("the response reads");
    let (head, rest) = response.split_once("\r\n\r\n").expect("a head and a body");
    assert!(
        !head.to_ascii_lowercase().contains("transfer-encoding: chunked"),
        "a chunked body, which this reader does not decode: {head}"
    );
    let status = head.split(' ').nth(1).and_then(|s| s.parse().ok()).expect("a status line");
    (status, serde_json::from_str(rest).unwrap_or(serde_json::Value::Null))
}

/// One `GET` to the server at `port` with NO identity header (security S2a, decision D-1), over the same plain
/// socket [`post`] uses: `Connection: close`, so the response is everything until the server closes, and each
/// read and write bounded by [`BOUND`]. The status, and the response's head as an [`http::HeaderMap`] — the
/// headers as the server wrote them, a repeated name kept as both lines; a head line that is no header fails
/// here by name rather than being dropped, so a header the cell asks for is never missing for a parse's sake.
#[cfg(feature = "open-door")]
fn get(port: u16, path: &str) -> (u16, http::HeaderMap) {
    use std::io::Write;
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server accepts");
    stream.set_read_timeout(Some(BOUND)).expect("a read bound");
    stream.set_write_timeout(Some(BOUND)).expect("a write bound");
    write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").expect("the request writes");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("the response reads");
    let response = String::from_utf8_lossy(&response);
    let (head, _) = response.split_once("\r\n\r\n").expect("a head and a body");
    let mut lines = head.split("\r\n");
    let status = lines.next().and_then(|l| l.split(' ').nth(1)).and_then(|s| s.parse().ok()).expect("a status line");
    let mut headers = http::HeaderMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').unwrap_or_else(|| panic!("a head line that is no header: {line:?}"));
        let name = http::HeaderName::from_bytes(name.trim().as_bytes()).unwrap_or_else(|e| panic!("{line:?}: {e}"));
        let value = http::HeaderValue::from_str(value.trim()).unwrap_or_else(|e| panic!("{line:?}: {e}"));
        headers.append(name, value);
    }
    (status, headers)
}

/// One stream `GET` to the server at `port` as `user` at the open door, over a
/// plain socket held OPEN (no `Connection: close`), each read and write
/// bounded by [`BOUND`]. A 200 is read until its first `signal` frame has
/// arrived — so hyper has polled the body at least once, the moment a slot
/// released at the first poll would be gone — and the socket is handed back
/// for the cell to hold; any other status is read to its `Content-Length` and
/// its body parsed as JSON. The status, the socket and the body (`Null` for a
/// 200).
#[cfg(feature = "open-door")]
fn open_stream(port: u16, user: &str, path: &str) -> (u16, std::net::TcpStream, serde_json::Value) {
    use std::io::Write;
    use yokohama_web::identity::CLAIMED_HEADER;
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server accepts");
    stream.set_read_timeout(Some(BOUND)).expect("a read bound");
    stream.set_write_timeout(Some(BOUND)).expect("a write bound");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{CLAIMED_HEADER}: {user}\r\nAccept: text/event-stream\r\n\r\n"
    )
    .expect("the request writes");
    let mut seen = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(at) = seen.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        let n = stream.read(&mut chunk).expect("the head reads");
        assert!(n > 0, "the server closed before a whole head: {:?}", String::from_utf8_lossy(&seen));
        seen.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&seen[..head_end]).into_owned();
    let status = head.split(' ').nth(1).and_then(|s| s.parse().ok()).expect("a status line");
    if status == 200 {
        while !String::from_utf8_lossy(&seen[head_end..]).contains("event: signal") {
            let n = stream.read(&mut chunk).expect("the first frame reads");
            assert!(n > 0, "the stream closed before its first frame: {:?}", String::from_utf8_lossy(&seen));
            seen.extend_from_slice(&chunk[..n]);
        }
        return (status, stream, serde_json::Value::Null);
    }
    let length: usize = head
        .lines()
        .find_map(|l| l.split_once(':').filter(|(name, _)| name.trim().eq_ignore_ascii_case("content-length")))
        .and_then(|(_, value)| value.trim().parse().ok())
        .unwrap_or_else(|| panic!("a {status} with no Content-Length, which this reader does not decode: {head}"));
    while seen.len() < head_end + length {
        let n = stream.read(&mut chunk).expect("the body reads");
        assert!(n > 0, "the server closed inside the body: {:?}", String::from_utf8_lossy(&seen));
        seen.extend_from_slice(&chunk[..n]);
    }
    let body = serde_json::from_slice(&seen[head_end..head_end + length]).unwrap_or(serde_json::Value::Null);
    (status, stream, body)
}

/// No stamp sentence of any of the rule's three shapes: each names the stamp
/// file, and each says "Turnstile site key".
fn assert_no_stamp_sentence(stderr: &str, shown: &str, what: &str) {
    assert!(!stderr.contains(shown), "{what}: stderr names the stamp file: {stderr:?}");
    assert!(!stderr.contains("Turnstile site key"), "{what}: stderr speaks of a site key: {stderr:?}");
}

/// **The catcher** (finding F-1): a client built with a Turnstile site key
/// behind `cf-access` — the process exits 1 BEFORE it listens (nothing on
/// stdout, where the listening line would be), and its stderr is exactly the
/// sentence `keyed_build_refusal` builds for the same stamp, file and door,
/// which names the file, the key, `YOKOHAMA_TURNSTILE_SITEKEY` and the door.
/// Mutant: the `check_build_stamp` call in `main` deleted — the server
/// starts, and this cell fails on its listening line.
#[test]
fn a_keyed_build_behind_cf_access_exits_1_before_it_listens_saying_the_rule_s_own_sentence() {
    let scratch = Scratch::new("keyed-cf-access");
    scratch.write_stamp(KEYED_STAMP);
    let shown = scratch.stamp_shown();
    let expected = keyed_build_refusal(Stamp::Text(KEYED_STAMP), &shown, IdentityMode::CfAccess)
        .expect_err("the rule refuses a keyed stamp behind cf-access");
    for part in [
        shown.as_str(),
        "says the client was built with a Turnstile site key",
        "YOKOHAMA_TURNSTILE_SITEKEY",
        "YOKOHAMA_WEB_IDENTITY is cf-access",
    ] {
        assert!(expected.contains(part), "the rule's sentence names {part:?}: {expected}");
    }

    let mut server = Server::spawn("cf-access", &scratch);
    match server.start() {
        Start::Exited(status) => {
            let (_, stdout, stderr) = server.finish();
            assert_eq!(status.code(), Some(1), "the refusal exits 1; stderr {stderr:?}");
            assert_eq!(stdout, "", "nothing printed before the exit, so it never listened");
            assert_eq!(stderr, format!("{expected}\n"), "stderr is the rule's sentence, whole");
        }
        Start::Listening(line) => {
            let (_, _, stderr) = server.finish();
            panic!(
                "the server STARTED with a keyed client build behind cf-access — it printed {line:?} and its stderr \
                 was {stderr:?}: the start refusal in main did not run"
            )
        }
        Start::Silent => {
            let (_, stdout, stderr) = server.finish();
            panic!(
                "the server did not exit within {BOUND:?} with a keyed client build behind cf-access (killed); \
                 stdout {stdout:?}, stderr {stderr:?}"
            )
        }
    }
}

/// **The control at the open door**: the SAME keyed stamp under `open` — the
/// door a keyed build is for — starts, is still running when killed, and
/// prints no stamp sentence. So what the catcher reads is the stamp under a
/// door, not the stamp alone. A build without the `open-door` feature refuses
/// `open` by name before the stamp is read, so the cell is that feature's.
#[cfg(feature = "open-door")]
#[test]
fn the_same_keyed_build_at_the_open_door_starts_and_says_nothing_of_the_stamp() {
    let scratch = Scratch::new("keyed-open");
    scratch.write_stamp(KEYED_STAMP);
    let shown = scratch.stamp_shown();
    assert_eq!(
        keyed_build_refusal(Stamp::Text(KEYED_STAMP), &shown, IdentityMode::Open),
        Ok(()),
        "the rule asks nothing at the open door",
    );

    let mut server = Server::spawn("open", &scratch);
    let line = server.listening("a keyed build at the open door");
    assert!(server.running(), "still running when killed, after {line:?}");
    let (status, _, stderr) = server.finish();
    assert_ne!(status.code(), Some(1), "killed, not refused; stderr {stderr:?}");
    assert_no_stamp_sentence(&stderr, &shown, "a keyed build at the open door");
}

/// **The control with no stamp**: a `dist` with no `build-values.json` (one
/// built before the stamp) behind `cf-access` — the rule reads `Absent` as a
/// start, and the process starts: it listens, is still running when killed,
/// and prints no stamp sentence.
#[test]
fn a_dist_with_no_stamp_behind_cf_access_starts_and_says_nothing_of_a_stamp() {
    let scratch = Scratch::new("absent-cf-access");
    let shown = scratch.stamp_shown();
    assert!(!Path::new(&shown).exists(), "the scratch dist holds no stamp");
    assert_eq!(keyed_build_refusal(Stamp::Absent, &shown, IdentityMode::CfAccess), Ok(()), "the rule starts it");

    let mut server = Server::spawn("cf-access", &scratch);
    let line = server.listening("no stamp behind cf-access");
    assert!(server.running(), "still running when killed, after {line:?}");
    let (status, _, stderr) = server.finish();
    assert_ne!(status.code(), Some(1), "killed, not refused; stderr {stderr:?}");
    assert_no_stamp_sentence(&stderr, &shown, "no stamp behind cf-access");
}

/// **The NAS's own path builds the production seed rule** (security S1a,
/// finding B3; R-SEC8(b); the review's I-1). The binary `main` runs —
/// `AppState::new`, which hands its file store to `AppState::production` —
/// refuses a creator's seed beside an open seat with `SEED_NOT_SOLO`'s
/// sentence and stores no table, and opens the same body unseeded with a 201.
///
/// **A cell on the wrong host is not coverage** (the M6.4 lesson):
/// `the_production_state_honours_a_seed_at_a_solo_table_only` pins
/// `production` in process, but nothing in process runs `main` → `new` →
/// `production`. `new` rebuilt through a test constructor — its pre-task body,
/// `with_store_chat_and_archive`, now the `Diagnostic` wrapper — left every
/// other suite green (the review's W13); here it reads 201 where 400. The
/// open door, so a claimed plain name is the creator; a port picked free,
/// because this cell must reach the server; the store read back by the
/// layer's own `FsStore` over the directory the child was given.
#[cfg(feature = "open-door")]
#[test]
fn the_binary_refuses_a_seed_beside_an_open_seat_and_opens_the_same_table_unseeded() {
    use yokohama_web::store::{Doc, FsStore, Store};
    use yokohama_web::tables::SEED_NOT_SOLO;

    let scratch = Scratch::new("seed-open");
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    let line = server.listening("the seed rule at the open door");
    assert!(line.ends_with(&format!(":{port}")), "listening where it was told to: {line:?}");
    let store = FsStore::new(scratch.root.join("games"));
    let body = |seed: bool| {
        let mut b = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": [{ "user": "red" }, { "open": true }], "id": "seeded", "name": "seeded",
        });
        if seed {
            b["seed"] = serde_json::json!("20260831");
        }
        b
    };
    let (status, refused) = post(port, "red", "/api/games", &body(true));
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["error"], SEED_NOT_SOLO, "the layer's sentence: {refused}");
    assert!(store.read(Doc::Table, "seeded").expect("the store reads").is_none(), "a refused create stored a table");
    let (status, opened) = post(port, "red", "/api/games", &body(false));
    assert_eq!(status, 201, "{opened}");
    // The anti-vacuity partner: the store read above is the one the server writes.
    assert!(store.read(Doc::Table, "seeded").expect("the store reads").is_some(), "the 201's table is in the store");
    assert!(server.running(), "still serving after both requests");
    let (status, _, stderr) = server.finish();
    assert_ne!(status.code(), Some(1), "killed, not refused; stderr {stderr:?}");
}

/// **The binary refuses an address under the open door and admits a name**
/// (security S2b, finding C1; decision D-2): the catcher of `main` →
/// `AppState::new` → `production` → `NamesOnly`, which no in-process cell runs
/// — the seed rule's binary cell's shape. An address claimed at the open door
/// is the layer's 401 in its one sentence; a plain name creates the table.
/// Mutants: the production literal building `Diagnostic` (the address reads
/// 201); the adapter resolving with a literal policy.
#[cfg(feature = "open-door")]
#[test]
fn the_binary_refuses_an_address_claim_at_the_open_door_and_admits_a_name() {
    use yokohama_web::identity::ADDRESS_NOT_A_NAME;

    let scratch = Scratch::new("claims-open");
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    let line = server.listening("the key rule at the open door");
    assert!(line.ends_with(&format!(":{port}")), "listening where it was told to: {line:?}");
    let body = |user: &str, id: &str| {
        serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seats": [{ "user": user }, { "open": true }], "id": id, "name": id,
        })
    };
    let (status, refused) = post(port, "red@wire.test", "/api/games", &body("red@wire.test", "addressed"));
    assert_eq!((status, refused), (401, serde_json::json!({ "error": ADDRESS_NOT_A_NAME })));
    let (status, opened) = post(port, "ben", "/api/games", &body("ben", "named"));
    assert_eq!(status, 201, "{opened}");
    assert!(server.running(), "still serving after both requests");
    let (status, _, stderr) = server.finish();
    assert_ne!(status.code(), Some(1), "killed, not refused; stderr {stderr:?}");
}

/// **The rename's table, read off the binary** (security S1c, finding D26;
/// the fix round's Minor 3). The deploy window renames every legacy folder from
/// what `yokohama-web segment` prints, so the line is pinned whole: one line
/// per address, three tab-separated columns in the order address, legacy,
/// current, the two segments equal to the library's own for that address. A
/// dropped column (M24) or the two segments swapped — which would rename every
/// folder backwards — is red at the first assertion. A value the grammar
/// refuses prints nothing on stdout, names itself on stderr and makes the exit
/// code 1, beside an address that prints; no address at all is no output and
/// exit 0.
#[test]
fn the_segment_subcommand_prints_address_legacy_and_current_and_exits_1_when_a_value_is_refused() {
    let user = User::parse("red@example.test").expect("a key");
    let (legacy, current) = (legacy_caller_segment(&user), caller_segment(&user));
    assert_ne!(legacy, current, "premise: the two columns differ, so a swap is visible");
    let line = format!("red@example.test\t{legacy}\t{current}\n");

    let (status, stdout, stderr) = Server::segment(&["red@example.test"]).exited("one address");
    assert_eq!(stdout, line, "address, legacy, current: {stderr:?}");
    assert_eq!((status.code(), stderr.as_str()), (Some(0), ""));

    let (status, stdout, stderr) = Server::segment(&[".."]).exited("a refused value alone");
    assert_eq!(stdout, "", "a refused value prints nothing on stdout");
    assert!(stderr.starts_with("..\t") && stderr.contains("is neither an email nor a name"), "{stderr:?}");
    assert_eq!(status.code(), Some(1), "a refused value is exit 1");

    let (status, stdout, stderr) = Server::segment(&["red@example.test", ".."]).exited("a refused value");
    assert_eq!(stdout, line, "the address the grammar admits still prints");
    assert!(stderr.starts_with("..\t") && stderr.contains("is neither an email nor a name"), "{stderr:?}");
    assert_eq!(stderr.lines().count(), 1, "{stderr:?}");
    assert_eq!(status.code(), Some(1), "any refused value is exit 1");

    let (status, stdout, stderr) = Server::segment(&[]).exited("no address");
    assert_eq!((status.code(), stdout.as_str(), stderr.as_str()), (Some(0), "", ""));
}

/// **The production path reads `dist/headers.json`** (security S2a, decision D-1) — `main → AppState::new →
/// production → assets::response_headers`, which no in-process cell runs: `/` answers every pair of a scratch
/// dist's file and `/api/health` the pairs plus `no-store`; with NO file the binary still starts, prints one line
/// naming `headers.json` on stderr and answers `/api/health` 200 without the set; with a MALFORMED file it exits 1
/// before it listens, naming the file.
#[cfg(feature = "open-door")]
#[test]
fn the_binary_serves_the_dist_header_set_and_refuses_a_malformed_one() {
    let pairs = r#"[["X-Frame-Options", "DENY"], ["Content-Security-Policy-Report-Only", "default-src 'self'"]]"#;
    // (1) The file present: both pairs on `/`, and `no-store` beside them on `/api/health`. The scratch dist holds
    // an `index.html`, else `/` is the 404 page (which carries the set too, but 200 is what this arm reads).
    let scratch = Scratch::new("headers-present");
    std::fs::write(scratch.dist().join("headers.json"), pairs).unwrap();
    std::fs::write(scratch.dist().join("index.html"), "<!doctype html><div id=root></div>").unwrap();
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    server.listening("present");
    let (status, headers) = get(port, "/");
    assert_eq!(status, 200);
    assert_eq!(headers.get("x-frame-options").and_then(|v| v.to_str().ok()), Some("DENY"));
    assert_eq!(
        headers.get("content-security-policy-report-only").and_then(|v| v.to_str().ok()),
        Some("default-src 'self'")
    );
    let (status, headers) = get(port, "/api/health");
    assert_eq!(status, 200);
    assert_eq!(headers.get("x-frame-options").and_then(|v| v.to_str().ok()), Some("DENY"));
    assert_eq!(
        headers.get("content-security-policy-report-only").and_then(|v| v.to_str().ok()),
        Some("default-src 'self'")
    );
    assert_eq!(headers.get("cache-control").and_then(|v| v.to_str().ok()), Some("no-store"));
    // With the file present the start says NOTHING of it: a `main` that printed the no-set line at every start
    // would pass the absent arm alone (Task 3's review, I-1).
    let (_, _, stderr) = server.finish();
    assert!(!stderr.contains("headers.json"), "a present file is not announced: {stderr}");
    drop(server);
    // (2) No file: the binary starts, says so once on stderr, serves without the set.
    let scratch = Scratch::new("headers-absent");
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    server.listening("absent");
    let (status, headers) = get(port, "/api/health");
    assert_eq!(status, 200);
    assert!(headers.get("x-frame-options").is_none(), "no file, no set");
    let (_, _, stderr) = server.finish();
    assert_eq!(stderr.matches("headers.json").count(), 1, "the start says ONCE what it could not read: {stderr}");
    drop(server);
    // (3) A malformed file: exit 1 before the listening line, the file named — read as the keyed-build cell
    // reads its exit.
    let scratch = Scratch::new("headers-malformed");
    std::fs::write(scratch.dist().join("headers.json"), "{}").unwrap();
    let (status, _, stderr) = Server::spawn("open", &scratch).exited("a malformed headers.json");
    assert_eq!(status.code(), Some(1));
    assert!(stderr.contains("headers.json"), "{stderr}");
}

/// One `GET /api/health` to the server at `port` with the `Host` the cell chooses (security S3, D4), over the same
/// plain socket [`get`] uses; only the status is read.
#[cfg(feature = "open-door")]
fn get_health_as_host(port: u16, host: &str) -> u16 {
    use std::io::Write;
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the server accepts");
    stream.set_read_timeout(Some(BOUND)).expect("a read bound");
    stream.set_write_timeout(Some(BOUND)).expect("a write bound");
    write!(stream, "GET /api/health HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").expect("the request writes");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("the response reads");
    let response = String::from_utf8_lossy(&response);
    response.split(' ').nth(1).and_then(|s| s.parse().ok()).expect("a status line")
}

/// **A malformed hostname list stops the binary before it listens, naming the entry** (security S3, D4;
/// R-S3-14): `main` reads `YOKOHAMA_WEB_HOSTNAMES` beside the bind, and an entry with a space in it is an exit 1
/// with the entry on stderr and nothing on stdout where the listening line would be. Mutant: the `exit(1)`
/// removed (the list is dropped and the binary listens).
#[test]
fn a_malformed_hostname_list_exits_one_before_it_listens_naming_the_entry() {
    let scratch = Scratch::new("hostnames-malformed");
    let mut command = binary();
    command
        .env(IDENTITY_VAR, "cf-access")
        .env("YOKOHAMA_WEB_DIST", scratch.dist())
        .env("YOKOHAMA_WEB_DIR", scratch.root.join("games"))
        .env("YOKOHAMA_WEB_PORT", "0")
        .env("YOKOHAMA_WEB_HOSTNAMES", "good.example,bad name");
    let (status, stdout, stderr) = Server::piped(command).exited("a malformed hostname list");
    assert_eq!(status.code(), Some(1), "stderr {stderr:?}");
    assert!(stderr.contains("bad name") && stderr.contains("YOKOHAMA_WEB_HOSTNAMES"), "{stderr:?}");
    assert!(!stdout.contains(LISTENING), "no listening line: {stdout:?}");
}

/// **The binary answers its listed hostname and refuses another with 421** (security S3, D4): the catcher of
/// `main` → `with_hostnames` → `app`, which no in-process cell runs. The start line names the listed name; the
/// loopback the cell's own socket uses is in the default set, so the cell sends `Host` itself. Mutant: `main`
/// never handing the state its reading (the default set holds, and the listed name reads 421).
#[cfg(feature = "open-door")]
#[test]
fn the_binary_answers_its_listed_hostname_and_refuses_another_with_421() {
    let scratch = Scratch::new("hostnames-listed");
    let port = free_port();
    let mut command = binary();
    command
        .env(IDENTITY_VAR, "open")
        .env("YOKOHAMA_WEB_DIST", scratch.dist())
        .env("YOKOHAMA_WEB_DIR", scratch.root.join("games"))
        .env("YOKOHAMA_WEB_PORT", port.to_string())
        .env("YOKOHAMA_WEB_HOSTNAMES_ENABLED", "on")
        .env("YOKOHAMA_WEB_HOSTNAMES", "yokohama-nas.moore.nyc");
    let mut server = Server::piped(command);
    server.listening("a listed hostname");
    assert_eq!(get_health_as_host(port, "yokohama-nas.moore.nyc"), 200);
    assert_eq!(get_health_as_host(port, "attacker.example"), 421);
    assert_eq!(get_health_as_host(port, "localhost:1"), 200, "the default set stands beside the list");
    let (_, stdout, stderr) = server.finish();
    assert!(
        stdout.contains("yokohama-web answers host names:") && stdout.contains("yokohama-nas.moore.nyc"),
        "the start line names the list: {stdout:?}"
    );
    assert!(stdout.find("answers host names").unwrap() < stdout.find(LISTENING).unwrap(), "before the listening line: {stdout:?}");
    assert!(!stderr.contains("attacker.example"), "{stderr:?}");
}

/// **A malformed switch refuses to start, and `off` answers any host and says so** (security S3, R-S3-14): `yes`
/// is an exit 1 naming the variable and the value; `off` starts, answers a foreign `Host` 200 and prints
/// "answers any host name". Mutant: an unknown value read as on (the exit disappears).
#[cfg(feature = "open-door")]
#[test]
fn a_malformed_switch_value_exits_one_and_off_answers_any_host_saying_so() {
    let spawn = |scratch: &Scratch, port: u16, enabled: &str| {
        let mut command = binary();
        command
            .env(IDENTITY_VAR, "open")
            .env("YOKOHAMA_WEB_DIST", scratch.dist())
            .env("YOKOHAMA_WEB_DIR", scratch.root.join("games"))
            .env("YOKOHAMA_WEB_PORT", port.to_string())
            .env("YOKOHAMA_WEB_HOSTNAMES_ENABLED", enabled);
        Server::piped(command)
    };
    let scratch = Scratch::new("hostnames-switch-bad");
    let (status, stdout, stderr) = spawn(&scratch, 0, "yes").exited("a malformed switch");
    assert_eq!(status.code(), Some(1), "stderr {stderr:?}");
    assert!(stderr.contains("YOKOHAMA_WEB_HOSTNAMES_ENABLED") && stderr.contains("\"yes\""), "{stderr:?}");
    assert!(!stdout.contains(LISTENING), "no listening line: {stdout:?}");

    let scratch = Scratch::new("hostnames-switch-off");
    let port = free_port();
    let mut server = spawn(&scratch, port, "off");
    server.listening("the switch off");
    assert_eq!(get_health_as_host(port, "attacker.example"), 200);
    let (_, stdout, _) = server.finish();
    assert!(stdout.contains("answers any host name"), "the start line says so: {stdout:?}");
}

/// **The stream slot is held by the body hyper serves, and freed when the
/// socket closes** (the S2 stack review, lens 2, Important 1; security S2c,
/// D3): the binary `main` runs, at the open door, over real sockets. A
/// caller's eight table streams, each held open and read past its first
/// frame, make the ninth a 429 in `TOO_MANY_STREAMS_CALLER`'s words with no
/// frame; another caller is still admitted beside them, so the refusal is the
/// caller's figure and not a server that stopped answering; closing one of the
/// eight sockets admits the caller's next stream, polled every 50 ms up to
/// [`BOUND`] since the server learns of a close only when hyper reads it.
/// Mutant (the review's M-slot-early): `Admitted`'s slot an `Option` taken at
/// the body's first poll — every in-process cell stays green, and here the
/// ninth reads 200.
#[cfg(feature = "open-door")]
#[test]
fn the_binary_holds_a_stream_slot_while_its_socket_is_open_and_frees_it_when_the_socket_closes() {
    use yokohama_web::streams::{STREAMS_PER_CALLER, TOO_MANY_STREAMS_CALLER};

    let scratch = Scratch::new("stream-slots");
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    let line = server.listening("the stream cap at the open door");
    assert!(line.ends_with(&format!(":{port}")), "listening where it was told to: {line:?}");
    let created = serde_json::json!({
        "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
        "seats": [{ "user": "red" }, { "open": true }], "id": "slots", "name": "slots",
    });
    let (status, body) = post(port, "red", "/api/games", &created);
    assert_eq!(status, 201, "{body}");
    let path = "/api/games/slots/stream";

    let mut held = Vec::new();
    for n in 1..=STREAMS_PER_CALLER {
        let (status, socket, body) = open_stream(port, "red", path);
        assert_eq!(status, 200, "stream {n} of {STREAMS_PER_CALLER} is admitted: {body}");
        held.push(socket);
    }
    let (status, _, body) = open_stream(port, "red", path);
    assert_eq!(
        (status, &body),
        (429, &serde_json::json!({ "error": TOO_MANY_STREAMS_CALLER })),
        "the ninth stream, with eight held open and each past its first frame"
    );
    let (status, blue, body) = open_stream(port, "blue", path);
    assert_eq!(status, 200, "another caller is admitted beside red's eight: {body}");
    drop(blue);

    drop(held.pop());
    let deadline = Instant::now() + BOUND;
    let reopened = loop {
        let (status, socket, body) = open_stream(port, "red", path);
        match status {
            200 => break socket,
            429 if Instant::now() < deadline => {
                assert_eq!(body["error"], TOO_MANY_STREAMS_CALLER, "only the caller's figure refuses here: {body}");
                drop(socket);
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => panic!("a closed socket did not free its slot within {BOUND:?}: the last open read {status} {body}"),
        }
    };
    drop(reopened);
    drop(held);
    assert!(server.running(), "still serving after every stream");
    let (status, _, stderr) = server.finish();
    assert_ne!(status.code(), Some(1), "killed, not refused; stderr {stderr:?}");
}
