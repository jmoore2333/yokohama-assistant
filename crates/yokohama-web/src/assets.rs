#![cfg(feature = "server")]
//! Serving the built client from the same origin as the API.
//!
//! Hand-rolled rather than a `tower-http` dependency: this is a read-only file
//! map with an SPA fallback, and the crate already carries the one dependency
//! decision it had to justify. If asset serving ever needs range requests or
//! compression negotiation, that is the moment to take the dependency — not
//! before. Caching came off that list at M6.5b and is [`cache_control`]: one
//! header on one prefix is not a negotiation.
//!
//! In development none of this runs: Vite serves the client and proxies `/api`
//! to this server, so the fetch paths are identical in both modes.

use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

/// Where the built client lives. `YOKOHAMA_WEB_DIST` overrides it; the default
/// is the Vite output relative to the repo root, which is where it lands for
/// anyone running the server from a checkout.
pub fn dist_dir() -> PathBuf {
    std::env::var_os("YOKOHAMA_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("web/dist"))
}

/// Resolve a request path to a file inside `root`, or `None`.
///
/// **The traversal guard is structural, not textual.** The path is walked
/// component by component and anything that is not a plain name — `..`, a root,
/// a prefix, a `.` — refuses the whole request. Rejecting the STRING `".."`
/// would be a denylist, and a denylist over paths is a game one eventually
/// loses; refusing every component that is not `Component::Normal` cannot be
/// encoded around.
pub fn resolve(root: &Path, uri_path: &str) -> Option<PathBuf> {
    let trimmed = uri_path.trim_start_matches('/');
    let rel = Path::new(if trimmed.is_empty() { "index.html" } else { trimmed });
    let mut out = root.to_path_buf();
    for component in rel.components() {
        match component {
            Component::Normal(part) => out.push(part),
            _ => return None,
        }
    }
    Some(out)
}

pub fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("woff2") => "font/woff2",
        Some("ico") => "image/x-icon",
        // M6.5b (R-6.5b.14): the browser host's module, emitted by Vite as a
        // hashed asset. `WebAssembly.instantiateStreaming` refuses any other
        // type, and the fallback it refuses into is a silent slow path at best.
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// How long a browser may keep what this map served, or `None` for no
/// freshness header at all.
///
/// **Only `assets/`, and that is the whole rule.** Vite writes a content hash
/// into every file name it emits there, so a changed byte is a changed name
/// and a year with `immutable` is the truth about it. Everything else must
/// stay uncached and for different reasons: `index.html` is the document that
/// NAMES those hashes, so a cached one pins a released build for a year, and
/// it is also what the SPA fallback answers under any other path.
///
/// **M6.5b is why this exists.** Before it, nothing here set any caching
/// header — deferred, and said so in the module doc — and the cost was a
/// re-fetched bundle. This cycle put the browser host's module (758 KB
/// gzipped) under `assets/` and mounts `OnThisDevice` on every first screen,
/// so the gap was being paid per visit.
fn cache_control(uri_path: &str) -> Option<&'static str> {
    uri_path.starts_with("/assets/").then_some("public, max-age=31536000, immutable")
}

/// Everything the API did not claim.
///
/// A miss falls back to `index.html` so client-side routes deep-link, which is
/// the standard SPA contract — but **only for paths that could be a route**. A
/// request for a missing `.js` or `.css` gets a 404, because answering it with
/// HTML produces the error every SPA developer has debugged at least once: the
/// browser reports a syntax error at `<!doctype`, and the real fault (a stale
/// or missing build) is nowhere in the message.
pub async fn serve(uri: Uri) -> Response {
    serve_path(&dist_dir(), uri.path())
}

/// [`serve`]'s whole body, over a root given rather than read from the
/// environment.
///
/// Split out at M6.5b's fix wave so the caching rule below can be asserted by
/// a cell: `dist_dir()` reads a process-global variable, and a test that set
/// it would race every other test in this binary.
fn serve_path(root: &Path, uri_path: &str) -> Response {
    let Some(path) = resolve(root, uri_path) else {
        return (StatusCode::BAD_REQUEST, "bad path").into_response();
    };

    if let Ok(bytes) = std::fs::read(&path) {
        let mut response =
            ([(header::CONTENT_TYPE, content_type(&path))], Body::from(bytes)).into_response();
        // The REQUEST's path decides, not the resolved file's: the fallback
        // below serves `index.html` under paths of its own, and it must not
        // pick the header up from the document it happens to read.
        if let Some(value) = cache_control(uri_path) {
            response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
        }
        return response;
    }
    if path.extension().is_some() {
        return (StatusCode::NOT_FOUND, "no such asset").into_response();
    }
    match std::fs::read(root.join("index.html")) {
        Ok(bytes) => {
            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], Body::from(bytes)).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            format!(
                "no client build at {} — run `npm run build` in web/, or set YOKOHAMA_WEB_DIST",
                root.display()
            ),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_is_refused_structurally() {
        let root = Path::new("/srv/dist");
        for bad in ["/../etc/passwd", "/a/../../etc/passwd", "/./x", "//../x"] {
            assert!(resolve(root, bad).is_none(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn ordinary_paths_resolve_under_the_root() {
        let root = Path::new("/srv/dist");
        assert_eq!(resolve(root, "/").unwrap(), root.join("index.html"));
        assert_eq!(resolve(root, "").unwrap(), root.join("index.html"));
        assert_eq!(
            resolve(root, "/assets/index-abc.js").unwrap(),
            root.join("assets").join("index-abc.js"),
        );
    }

    #[test]
    fn content_types_cover_what_a_vite_build_emits() {
        assert_eq!(content_type(Path::new("a/index.html")), "text/html; charset=utf-8");
        assert_eq!(content_type(Path::new("a/index-x.js")), "text/javascript; charset=utf-8");
        assert_eq!(content_type(Path::new("a/index-x.css")), "text/css; charset=utf-8");
        assert_eq!(content_type(Path::new("a/font.woff2")), "font/woff2");
        // M6.5b (R-6.5b.14): the browser host's module. Vite emits it as a
        // hashed asset beside the bundle, and a browser refuses to
        // `instantiateStreaming` anything not served as `application/wasm`.
        assert_eq!(content_type(Path::new("wasm/yokohama_wasm.wasm")), "application/wasm");
    }

    /// M6.5b's fix wave: Vite's hashed output is immutable, and NOTHING else
    /// this map serves carries a freshness header.
    ///
    /// The `assets/` half is what the cycle needs — `OnThisDevice` mounts on
    /// every first screen and instantiates the browser host's module, so
    /// without this the 758 KB was re-fetched on every visit. The `index.html`
    /// half is what keeps it safe: that document is the one that NAMES the
    /// hashes, and a cached copy would pin a released build for a year. The
    /// SPA fallback is asserted too, because it answers `index.html`'s bytes
    /// under some other path.
    #[test]
    fn hashed_assets_are_immutable_and_nothing_else_carries_a_cache_header() {
        let root =
            std::env::temp_dir().join(format!("yokohama-assets-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets").join("yokohama_wasm_bg-Bo5LQV_3.wasm"), b"\0asm")
            .unwrap();
        std::fs::write(root.join("index.html"), b"<!doctype html>").unwrap();

        let cached = |uri: &str| -> Option<String> {
            serve_path(&root, uri)
                .headers()
                .get(header::CACHE_CONTROL)
                .map(|v| v.to_str().unwrap().to_string())
        };

        assert_eq!(
            cached("/assets/yokohama_wasm_bg-Bo5LQV_3.wasm").as_deref(),
            Some("public, max-age=31536000, immutable"),
            "a hashed asset is immutable by construction"
        );
        assert_eq!(cached("/index.html"), None, "the document that names the hashes");
        assert_eq!(cached("/lobby"), None, "the same document through the SPA fallback");

        std::fs::remove_dir_all(&root).unwrap();
    }
}
