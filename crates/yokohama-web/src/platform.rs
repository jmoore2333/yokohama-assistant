//! **The platform calls every scan in this workspace refuses** (M6.5c2, spec
//! §2.6's restatements).
//!
//! Three scans read source for them:
//! - this crate's own `platform_calls_stay_behind_the_server_feature`
//!   (`lib.rs`), which ENFORCES a `server` gate on each item that makes one;
//! - each host crate's — `crates/yokohama-wasm`'s and
//!   `crates/yokohama-worker`'s — which refuses them outright above its test
//!   module.
//!
//! Each carried its own copy of the eight. The list is stated HERE once, and
//! [`calls_in`] is the hosts' scan over one file's text.
//!
//! **Each pattern is written in two halves** (`concat!`), and that is not
//! decoration: the layer's own scan reads THIS file, and a list spelled whole
//! would be eight escaped calls to it. The test module below spells them
//! whole, where no scan reads.

/// The eight, in M6.5a's order.
pub const CALLS: [&str; 8] = [
    concat!("System", "Time"),
    concat!("Instant", "::now"),
    concat!("std", "::fs"),
    concat!("std", "::env"),
    concat!("std", "::thread"),
    concat!("std", "::process"),
    concat!("tokio", "::"),
    concat!("axum", "::"),
];

/// A HOST crate's scan of one source file (M6.5c2): every line above the
/// file's first column-0 `#[cfg(test)]` that names one of [`CALLS`] — prose
/// included, since a host's sources name none — as `file:line: pattern: code`.
///
/// It reads TEXT and no file: the host reads its own `src/`, so this builds
/// without `server` and makes no platform call itself. The host keeps its own
/// anti-vacuity partners (the calls it ALLOWS are its own).
pub fn calls_in(file: &str, text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
    let mut found = Vec::new();
    for (n, line) in lines[..end].iter().enumerate() {
        for pattern in CALLS {
            if line.contains(pattern) {
                found.push(format!("{file}:{}: {pattern}: {}", n + 1, line.trim()));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The eight, stated once** (M6.5c2): the list every scan in this
    /// workspace refuses, in M6.5a's order, spelled whole HERE, where the
    /// layer's own scan does not read.
    #[test]
    fn the_eight_platform_calls() {
        assert_eq!(
            CALLS,
            ["SystemTime", "Instant::now", "std::fs", "std::env", "std::thread", "std::process", "tokio::", "axum::"],
        );
    }

    /// **A host's scan over one file** (M6.5c2): every line above the first
    /// column-0 `#[cfg(test)]` that names a call — prose included, since a
    /// host's sources name none — as `file:line: pattern: code`, and nothing
    /// below it. Two hits on one line are two entries.
    #[test]
    fn a_hosts_scan_names_each_call_above_its_test_module_and_nothing_below() {
        let text = "use std::sync::Arc;\n\
                    fn a() { let _ = std::env::var(\"X\"); }\n\
                    // std::fs, named in prose\n\
                    fn b() { let _ = (std::thread::current(), std::process::id()); }\n\
                    #[cfg(test)]\n\
                    mod tests { fn c() { let _ = std::fs::read(\"x\"); } }\n";
        assert_eq!(
            calls_in("host.rs", text),
            [
                "host.rs:2: std::env: fn a() { let _ = std::env::var(\"X\"); }",
                "host.rs:3: std::fs: // std::fs, named in prose",
                "host.rs:4: std::thread: fn b() { let _ = (std::thread::current(), std::process::id()); }",
                "host.rs:4: std::process: fn b() { let _ = (std::thread::current(), std::process::id()); }",
            ],
        );
        assert!(calls_in("clean.rs", "fn main() {}\n").is_empty());
    }
}
