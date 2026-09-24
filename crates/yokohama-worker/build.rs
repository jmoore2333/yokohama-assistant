//! **The picker's code identity** (the orchestrator's ruling I-85; spec §2.4):
//! a digest of the sources that give a re-fed picker line its meaning — the
//! CLI's `crates/yokohama-cli/src` (the prompt numbering, the grammars,
//! `Repl::step`) and the layer's `crates/yokohama-web/src` (`api::input` and
//! its gameplay fence) — handed to the crate as the compile-time variable
//! `YOKOHAMA_WORKER_PICKER_CODE`, which `picker.rs` reads as `CODE`. A stored
//! picker row carries the code it was built under, and `restore_picker` clears
//! a row whose code is not this build's rather than re-feed a line whose
//! meaning may have changed. The engine is not among the roots: a change to
//! what a move means there bumps `SEMANTICS_REV`, and a record of another
//! revision does not load at all.
//!
//! **The digest, in full**: every file under the two roots whose name does not
//! begin with a dot (a `.DS_Store`, an editor's swap file), in the byte order
//! of its path from the repository root with `/` between the parts; for each,
//! the path, a NUL, its length in bytes as decimal, a NUL, and its bytes;
//! SHA-256 of all of it, in lowercase hex.
//!
//! `cargo:rerun-if-changed` names every directory and every file under the
//! roots, so an edit, an added file or a removed one runs this script again.
//! It runs on the build host, never in the module: the crate's platform scan
//! (`this_host_makes_no_platform_call_but_the_clock`) reads `src/` alone.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The two roots, from the repository root.
const ROOTS: [&str; 2] = ["crates/yokohama-cli/src", "crates/yokohama-web/src"];

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let repo = manifest
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits at crates/yokohama-worker, two levels under the repository root");
    let mut files = Vec::new();
    for root in ROOTS {
        walk(repo, root, &mut files);
    }
    files.sort();
    let mut hasher = Sha256::new();
    for path in &files {
        let bytes = fs::read(repo.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes.len().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
    }
    println!("cargo:rustc-env=YOKOHAMA_WORKER_PICKER_CODE={:x}", hasher.finalize());
}

/// Every file under `rel` — a path from the repository root, `/`-separated —
/// pushed onto `files`, and every directory and file named to cargo.
fn walk(repo: &Path, rel: &str, files: &mut Vec<String>) {
    let dir = repo.join(rel);
    println!("cargo:rerun-if-changed={}", dir.display());
    let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("{rel}: {e}"));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("{rel}: {e}"));
        let name = entry.file_name();
        let name = name.to_str().unwrap_or_else(|| panic!("{rel}: a file name that is not UTF-8"));
        if name.starts_with('.') {
            continue;
        }
        let child = format!("{rel}/{name}");
        if entry.file_type().unwrap_or_else(|e| panic!("{child}: {e}")).is_dir() {
            walk(repo, &child, files);
        } else {
            println!("cargo:rerun-if-changed={}", repo.join(&child).display());
            files.push(child);
        }
    }
}
