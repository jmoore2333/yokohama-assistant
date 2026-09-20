#![cfg(feature = "server")]
//! Serving the built client from the same origin as the API.
//!
//! Hand-rolled rather than a `tower-http` dependency: this is a read-only file
//! map with an SPA fallback, and the crate already carries the one dependency
//! decision it had to justify. If asset serving ever needs range requests,
//! compression negotiation or caching headers, that is the moment to take the
//! dependency — not before.
//!
//! In development none of this runs: Vite serves the client and proxies `/api`
//! to this server, so the fetch paths are identical in both modes.

use std::path::{Component, Path, PathBuf};

use axum::body::Body;
use axum::http::{header, StatusCode, Uri};
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

/// Everything the API did not claim.
///
/// A miss falls back to `index.html` so client-side routes deep-link, which is
/// the standard SPA contract — but **only for paths that could be a route**. A
/// request for a missing `.js` or `.css` gets a 404, because answering it with
/// HTML produces the error every SPA developer has debugged at least once: the
/// browser reports a syntax error at `<!doctype`, and the real fault (a stale
/// or missing build) is nowhere in the message.
pub async fn serve(uri: Uri) -> Response {
    let root = dist_dir();
    let Some(path) = resolve(&root, uri.path()) else {
        return (StatusCode::BAD_REQUEST, "bad path").into_response();
    };

    if let Ok(bytes) = std::fs::read(&path) {
        return ([(header::CONTENT_TYPE, content_type(&path))], Body::from(bytes)).into_response();
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
}
