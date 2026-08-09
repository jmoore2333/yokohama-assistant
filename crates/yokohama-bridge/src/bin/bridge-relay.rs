//! `bridge-relay` — the localhost capture sink for the BGA bridge (Phase 3.5b, H0).
//!
//! Run it, load the extension, open the table. Every `gamedatas` snapshot and
//! every notification response the page receives lands in
//! `<dir>/<table_id>.ndjson`, one JSON object per line.
//!
//! Usage:
//! ```text
//! bridge-relay [--dir <path>] [--port <n>] [--host <loopback addr>]
//! ```
//!
//! `--host` exists only so the choice between `127.0.0.1` and `::1` is the
//! owner's; it cannot be used to expose the relay, because anything that is not
//! loopback is refused before the socket is opened. See `bind::resolve`.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use yokohama_bridge::server::{self, Relay};
use yokohama_bridge::{bind, DEFAULT_DIR, DEFAULT_PORT};

const USAGE: &str = "\
bridge-relay - BGA bridge capture sink (Phase 3.5b)

USAGE:
    bridge-relay [--dir <path>] [--port <n>] [--host <addr>]

OPTIONS:
    --dir  <path>   where capture files are written (default: ./bridge-captures)
    --port <n>      TCP port to listen on (default: 8787)
    --host <addr>   loopback address to bind (default: 127.0.0.1)
                    non-loopback addresses are refused by design
    -h, --help      print this and exit

ROUTES:
    GET  /health              liveness probe; reports the capture directory
    POST /capture/<table_id>  append one {kind, payload} item as NDJSON
";

struct Args {
    dir: PathBuf,
    port: u16,
    host: String,
}

fn parse_args(argv: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut args =
        Args { dir: PathBuf::from(DEFAULT_DIR), port: DEFAULT_PORT, host: "127.0.0.1".into() };
    let mut argv = argv.peekable();
    while let Some(flag) = argv.next() {
        let mut value = || argv.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--dir" => args.dir = PathBuf::from(value()?),
            "--port" => {
                let raw = value()?;
                args.port = raw.parse().map_err(|_| format!("--port {raw:?} is not a port number"))?;
            }
            "--host" => args.host = value()?,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(Some(args))
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(Some(args)) => args,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("bridge-relay: {e}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    // Refuse a non-loopback bind BEFORE opening anything.
    let addr = match bind::resolve(&args.host, args.port) {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("bridge-relay: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Create the directory up front so the health probe can report an absolute
    // path — the owner's first question when a capture seems to be missing is
    // "where is it writing?", and the answer should not be a relative path.
    if let Err(e) = std::fs::create_dir_all(&args.dir) {
        eprintln!("bridge-relay: could not create capture directory {}: {e}", args.dir.display());
        return ExitCode::FAILURE;
    }
    let dir = std::fs::canonicalize(&args.dir).unwrap_or(args.dir);

    let listener = match TcpListener::bind(addr) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("bridge-relay: could not bind {addr}: {e}");
            if e.kind() == std::io::ErrorKind::AddrInUse {
                eprintln!("  (another bridge-relay is probably already running on this port)");
            }
            return ExitCode::FAILURE;
        }
    };

    println!("bridge-relay listening on http://{addr}");
    println!("  captures -> {}", dir.display());
    println!("  health   -> curl http://{addr}/health");
    println!("  read-only: this process never talks to Board Game Arena.");

    server::serve(listener, Arc::new(Relay::new(dir)));
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> Result<Option<Args>, String> {
        parse_args(argv.iter().map(|s| s.to_string()))
    }

    #[test]
    fn defaults_match_the_documented_ones() {
        let args = parse(&[]).expect("no args parse").expect("not help");
        assert_eq!(args.dir, PathBuf::from("bridge-captures"));
        assert_eq!(args.port, 8787);
        assert_eq!(args.host, "127.0.0.1");
    }

    #[test]
    fn every_flag_is_honored() {
        let args = parse(&["--dir", "/tmp/caps", "--port", "9999", "--host", "::1"])
            .expect("parses")
            .expect("not help");
        assert_eq!(args.dir, PathBuf::from("/tmp/caps"));
        assert_eq!(args.port, 9999);
        assert_eq!(args.host, "::1");
    }

    #[test]
    fn help_short_circuits_and_bad_input_is_an_error_not_a_default() {
        assert!(parse(&["--help"]).expect("parses").is_none());
        assert!(parse(&["-h"]).expect("parses").is_none());
        // A typo must not silently fall back to a default — that is how a relay
        // ends up writing somewhere the owner is not looking.
        assert!(parse(&["--dirr", "x"]).is_err());
        assert!(parse(&["--port", "not-a-number"]).is_err());
        assert!(parse(&["--dir"]).is_err(), "a flag with no value is an error");
    }
}
