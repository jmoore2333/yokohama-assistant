#![cfg(feature = "server")]
//! Serving the built client from the same origin as the API.
//!
//! Hand-rolled rather than a `tower-http` dependency: this is a read-only file
//! map with an SPA fallback, and the crate already carries the one dependency
//! decision it had to justify. If asset serving ever needs range requests or
//! compression negotiation, that is the moment to take the dependency — not
//! before. Caching came off that list at M6.5b and is `cache_control`: one
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
        // M6.5b2 (R-6.5b2.9): the web app manifest, which the browser's install
        // check reads. Without this arm it falls through to
        // `application/octet-stream` and the install prompt never appears.
        Some("webmanifest") => "application/manifest+json",
        // The two `dist/fonts/<family>/OFL.txt` copies — the grant `ASSETS.md`
        // says travels with the bytes (whole-branch review, Minor 9). Served as
        // the default they were a DOWNLOAD rather than a licence a reader could
        // open, which is a poor way to carry a licence; and
        // `precache.ts`'s mirror moves with this arm, because the install
        // compares what it computed against what the server answered.
        Some("txt") => "text/plain; charset=utf-8",
        // Cycle F (R-F12; spec §2.7): the sitemap `presence.mjs` writes. A
        // crawler's sitemap reader expects XML, and the default would serve it
        // as an opaque download. It is never precached, so the install never
        // meets it; `precache.ts`'s mirror carries the arm all the same, so the
        // two tables stay equal arm for arm.
        Some("xml") => "application/xml",
        _ => "application/octet-stream",
    }
}

/// How long a browser may keep what this map served, or `None` for no
/// freshness header at all.
///
/// **`assets/` is the rule that matters most.** Vite writes a content hash
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
///
/// **M6.5b2 adds the second rule and it is the opposite one** (R-6.5b2.8). The
/// service worker script is how a browser decides whether a release has
/// changed — a byte comparison of this one file — so a heuristically cached
/// copy delays every update by however long the browser chose to keep it, and
/// with no freshness header at all heuristic caching is exactly what it may do.
/// `no-store` is the only answer that makes update detection a property of the
/// release rather than of a cache the server never saw.
///
/// **M6.5c2 cycle E puts the shell under the same rule** (R-6.5c2.41). The
/// worker installs the shell from `/shell.bin`, a byte copy of `index.html`
/// under a name the zone's edge does not inject its script into — and `.bin` is
/// on Cloudflare's default cached-extension list, so an answer with no header
/// stays at the edge for two hours: the previous release's shell for every
/// install in that window, each failing its digest. The worker's `cache:
/// "reload"` goes past the browser's cache, not the edge's; `no-store` keeps
/// either from holding a copy.
///
/// **Cycle F adds the web presence's three files the art's hour** (R-F12;
/// spec §2.7): [`PRESENCE_FILES`], written by `web/scripts/presence.mjs` at
/// build under stable names, so `immutable` would be a lie and `no-store`
/// would make every crawl a fresh read. `worker/static/_headers` says the same
/// of the same three paths for the other host, and
/// `worker/scripts/assemble-assets.test.mjs` reads the two files against each
/// other.
fn cache_control(uri_path: &str) -> Option<&'static str> {
    if uri_path == "/sw.js" || uri_path == "/shell.bin" {
        return Some("no-store");
    }
    if PRESENCE_FILES.contains(&uri_path) {
        return Some("public, max-age=3600");
    }
    uri_path.starts_with("/assets/").then_some("public, max-age=31536000, immutable")
}

/// The web presence's files (cycle F, R-F12): `robots.txt`, `llms.txt` and the
/// sitemap, each at the root, each given the art's hour by [`cache_control`].
/// Exact paths — never a prefix, which would catch a name that only resembles
/// one.
const PRESENCE_FILES: [&str; 3] = ["/robots.txt", "/llms.txt", "/sitemap.xml"];

/// The page a miss with an extension is answered with, relative to the root
/// (M6.5c2 cycle F, R-F5; spec §2.5): `web/public/404.html`, which the build
/// copies into `dist`.
const NOT_FOUND_PAGE: &str = "404.html";

/// Everything the API did not claim.
///
/// A miss falls back to `index.html` so client-side routes deep-link, which is
/// the standard SPA contract — but **only for paths that could be a route**. A
/// request for a missing `.js` or `.css` gets a 404, because answering it with
/// the shell and a 200 produces the error every SPA developer has debugged at
/// least once: the browser reports a syntax error at `<!doctype`, and the real
/// fault (a stale or missing build) is nowhere in the message.
///
/// **Since cycle F that 404 carries the build's own 404 page** (R-F5; spec
/// §2.5), for parity with the Worker, whose asset layer answers a browser's
/// navigation miss with the same file: a person who types a wrong address
/// sees words and a way home instead of a blank. Still a 404, so a script or
/// an install that asked for the file fails as honestly as before. A dist
/// with no `404.html` answers the bare 404 it always did. The page is no
/// fence against a scanner, and this host needs none: it has no per-request
/// meter.
///
/// **The root is GIVEN since M6.5b2** (`AppState::dist`, read from
/// [`dist_dir`] once at construction) rather than read from the environment per
/// request. It is the seam the archive and the clock already are, and it is
/// what lets the turnstile cell arrange a dist the ROUTER really answers from —
/// without which "the fallback refuses `/enter` rather than serving the shell"
/// could only be asserted about a dist the router never saw.
pub async fn serve(root: &Path, uri: Uri) -> Response {
    serve_path(root, uri.path())
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
        let page = root.join(NOT_FOUND_PAGE);
        let mut response = match std::fs::read(&page) {
            Ok(bytes) => {
                (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, content_type(&page))], Body::from(bytes))
                    .into_response()
            }
            Err(_) => (StatusCode::NOT_FOUND, "no such asset").into_response(),
        };
        // **A miss carries the same freshness rule the hit would have**
        // (M6.5b2, Task 1's deferred Minor). Production ships `sw.js`, so the
        // exposure is a deployment that removed it or served it from a root
        // that has none: without this the 404 goes out with no header at all,
        // a browser may heuristically cache it, and the release cannot be
        // repaired until that window closes. Applied through
        // [`cache_control`] rather than by naming `/sw.js` again, so the rule
        // has one home.
        if let Some(value) = cache_control(uri_path) {
            response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
        }
        return response;
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
        // M6.5b2's fix wave: the OFL copies that ship under `dist/fonts/`.
        assert_eq!(content_type(Path::new("fonts/inter/OFL.txt")), "text/plain; charset=utf-8");
        // Cycle F's sitemap (R-F12): an XML document is `application/xml`, which
        // is what a crawler's sitemap reader expects of it; the default would
        // hand it out as an opaque download.
        assert_eq!(content_type(Path::new("sitemap.xml")), "application/xml");
    }

    /// The manifest's media type, which the installer checks (spec §2.4). Without
    /// this arm it falls through to `application/octet-stream`.
    #[test]
    fn the_web_app_manifest_has_its_own_media_type() {
        assert_eq!(content_type(Path::new("manifest.webmanifest")), "application/manifest+json");
    }

    /// M6.5b's fix wave, widened by M6.5b2 (R-6.5b2.8), by M6.5c2 cycle E
    /// (R-6.5c2.41) and by cycle F's web presence (R-F12; spec §2.7): Vite's
    /// hashed output is immutable, the service worker and the shell it installs
    /// are `no-store`, the three presence files (`/robots.txt`, `/llms.txt`,
    /// `/sitemap.xml`) carry the art's hour, and NOTHING else this map serves
    /// carries a freshness header.
    ///
    /// The `assets/` half is what M6.5b needed — `OnThisDevice` mounts on
    /// every first screen and instantiates the browser host's module, so
    /// without this the 758 KB was re-fetched on every visit. The `index.html`
    /// half is what keeps it safe: that document is the one that NAMES the
    /// hashes, and a cached copy would pin a released build for a year. The
    /// SPA fallback is asserted too, because it answers `index.html`'s bytes
    /// under some other path.
    ///
    /// **One cell and not two**, so the "nothing else" clause stays a single
    /// claim: a second cell for `/sw.js` alone would leave this one's `for` loop
    /// free to be quietly narrowed. The rule is read BOTH ways — through
    /// `serve_path`, which is what proves the header reaches a response, and
    /// through [`cache_control`] directly, which is how paths this map never
    /// reads from disk (`/art/*`, a manifest that is not built in a test) are
    /// covered without arranging a file for each.
    #[test]
    fn hashed_assets_are_immutable_the_worker_and_its_shell_are_no_store_and_nothing_else_carries_a_header() {
        assert_eq!(cache_control("/assets/index-abc123.js"), Some("public, max-age=31536000, immutable"));
        // Update detection is a BYTE COMPARISON of this one file, so a browser
        // that heuristically caches it delays every update by the cache window —
        // and with no header at all, heuristic caching is exactly what it may do.
        assert_eq!(cache_control("/sw.js"), Some("no-store"));
        // The shell the worker installs (R-6.5c2.41). `.bin` is on Cloudflare's
        // default cached-extension list, so with no header the edge keeps the
        // PREVIOUS release's copy for two hours after a deploy, and every
        // install in that window fails its digest.
        assert_eq!(cache_control("/shell.bin"), Some("no-store"));
        // **The web presence** (cycle F, R-F12): the art's hour (`art.rs`), no
        // `immutable` — the names are stable and the bytes change with the
        // build values — and no `no-store`, which would make every crawl a
        // fresh read. The same three paths `worker/static/_headers` names, and
        // `assemble-assets.test.mjs` reads the two files against each other.
        for path in ["/robots.txt", "/llms.txt", "/sitemap.xml"] {
            assert_eq!(cache_control(path), Some("public, max-age=3600"), "{path} carries the art's hour");
        }
        for path in ["/", "/index.html", "/manifest.webmanifest", "/art/manifest.json", "/help/screen.webp"] {
            assert_eq!(cache_control(path), None, "{path} carries no freshness header");
        }
        // EXACT paths: a name that only resembles one, or the same file under
        // a directory, is no presence file.
        for path in ["/robots.txt.bak", "/llms-full.txt", "/sitemap.xml.gz", "/help/robots.txt", "/robots.txt/"] {
            assert_eq!(cache_control(path), None, "{path} is not a presence file");
        }

        let root =
            std::env::temp_dir().join(format!("yokohama-assets-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("assets").join("yokohama_wasm_bg-Bo5LQV_3.wasm"), b"\0asm")
            .unwrap();
        std::fs::write(root.join("index.html"), b"<!doctype html>").unwrap();
        std::fs::write(root.join("sw.js"), b"// the worker").unwrap();
        std::fs::write(root.join("shell.bin"), b"<!doctype html>").unwrap();
        std::fs::write(root.join("robots.txt"), b"User-agent: *\n").unwrap();
        std::fs::write(root.join("llms.txt"), b"# Yokohama\n").unwrap();
        std::fs::write(root.join("sitemap.xml"), b"<urlset/>").unwrap();

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
        assert_eq!(cached("/sw.js").as_deref(), Some("no-store"), "and it reaches the response");
        assert_eq!(cached("/shell.bin").as_deref(), Some("no-store"), "and so does the shell's");
        for path in ["/robots.txt", "/llms.txt", "/sitemap.xml"] {
            assert_eq!(cached(path).as_deref(), Some("public, max-age=3600"), "{path}: the hour reaches the response");
        }
        // And the types the files go out as: the two text files are plain text,
        // which is what a crawler or a model reads them as, and the sitemap is
        // XML (never the octet-stream default, which a sitemap reader may
        // refuse).
        for (path, kind) in [
            ("/robots.txt", "text/plain; charset=utf-8"),
            ("/llms.txt", "text/plain; charset=utf-8"),
            ("/sitemap.xml", "application/xml"),
        ] {
            let response = serve_path(&root, path);
            assert_eq!(
                response.headers().get(header::CONTENT_TYPE).map(|v| v.to_str().unwrap()),
                Some(kind),
                "{path} goes out as {kind}"
            );
        }
        assert_eq!(cached("/index.html"), None, "the document that names the hashes");
        assert_eq!(cached("/lobby"), None, "the same document through the SPA fallback");

        // **And a MISS carries it too** (M6.5b2, Task 1's deferred Minor).
        // Production ships the file, so the exposure is a deployment that
        // removed it — where a heuristically cached 404 keeps the release
        // un-updatable for as long as the browser chose. Read on a root that
        // really has no `sw.js`, with the shape of the answer asserted as well
        // as its header, because a 404 that had become the SPA fallback would
        // carry no extension and take a different arm entirely.
        let bare = root.join("bare");
        std::fs::create_dir_all(&bare).unwrap();
        let missed = serve_path(&bare, "/sw.js");
        assert_eq!(missed.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            missed.headers().get(header::CACHE_CONTROL).map(|v| v.to_str().unwrap()),
            Some("no-store"),
            "an absent worker must not be cached either"
        );
        let other = serve_path(&bare, "/assets/index-abc123.js");
        assert_eq!(other.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            other.headers().get(header::CACHE_CONTROL).map(|v| v.to_str().unwrap()),
            Some("public, max-age=31536000, immutable"),
            "the rule is `cache_control`'s and is not a second mention of /sw.js"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    /// One GET through the WHOLE router, which is what decides whether a path
    /// reaches this map at all (`fallback` in `lib.rs` claims `/api/`,
    /// `/enter` and `/art/` first): status, `content-type`, body.
    async fn get(router: &axum::Router, path: &str) -> (StatusCode, String, Vec<u8>) {
        use tower::ServiceExt;
        let res = router
            .clone()
            .oneshot(axum::http::Request::get(path).body(Body::empty()).expect("a request"))
            .await
            .expect("the router answers");
        let status = res.status();
        let kind = res
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().expect("an ASCII content type").to_string())
            .unwrap_or_default();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        (status, kind, body.to_vec())
    }

    /// A scratch `dist` the ROUTER serves from (`AppState::dist`), holding a
    /// shell and, when `page` is given, a `404.html` — never written through
    /// `YOKOHAMA_WEB_DIST`, a process-global the cells beside it read.
    fn router_over(name: &str, page: Option<&[u8]>) -> (axum::Router, PathBuf) {
        let root = std::env::temp_dir().join(format!("yokohama-assets-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a dist directory");
        std::fs::write(root.join("index.html"), SHELL).expect("a shell");
        if let Some(page) = page {
            std::fs::write(root.join("404.html"), page).expect("a 404 page");
        }
        let state = crate::AppState::in_memory().with_dist(&root);
        (crate::app(std::sync::Arc::new(state)), root)
    }

    const SHELL: &[u8] = b"<!doctype html><title>Yokohama</title>";

    /// **A miss with an extension answers the 404 page; a route still answers
    /// the shell** (M6.5c2 cycle F, R-F5; spec §2.5; Review Focus 5). The
    /// axum host has no edge, so this is its parity with the Worker's asset
    /// layer, which answers a browser's navigation miss with the same file
    /// (`not_found_handling: "404-page"`). The page's BYTES are read from the
    /// dist — the body here is the scratch file's, which no hard-coded page
    /// could match — served 404 as `text/html` the way the shell is. The
    /// extension-less fallback stays the shell, since the client's routes are
    /// hash-only and an old link or a typed path must still open the app.
    /// Mutants: the page hard-coded; the page served 200; the miss arm left
    /// bare; the fallback narrowed so a route answers the page.
    #[tokio::test]
    async fn a_miss_with_an_extension_answers_the_404_page_and_a_route_still_the_shell() {
        let page: &[u8] = b"<!doctype html><title>Nothing here</title><p>the scratch page of the 404 cell</p>";
        let (router, root) = router_over("404-page", Some(page));

        for path in ["/nope.json", "/wp-login.php", "/.env.php", "/assets/index-gone.js"] {
            let (status, kind, body) = get(&router, path).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{path} is a miss");
            assert_eq!(kind, "text/html; charset=utf-8", "{path} answers the page as HTML");
            assert_eq!(body, page, "{path} answers the 404 page's own bytes");
        }
        for path in ["/", "/t/abc", "/lobby"] {
            let (status, kind, body) = get(&router, path).await;
            assert_eq!(status, StatusCode::OK, "{path} is a route");
            assert_eq!(kind, "text/html; charset=utf-8");
            assert_eq!(body, SHELL, "{path} still answers the shell");
        }
        // The page is a file of the build like any other: asked for by name,
        // it is answered 200.
        let (status, _, body) = get(&router, "/404.html").await;
        assert_eq!((status, body.as_slice()), (StatusCode::OK, page));

        std::fs::remove_dir_all(&root).expect("the dist is removable");
    }

    /// **A dist with no `404.html` answers a miss as it did before cycle F**:
    /// a bare 404 with no HTML in it — a self-host built before the page, or a
    /// root someone arranged by hand, does not start answering misses with
    /// the shell. Mutant: the fallback for a missing page made `index.html`.
    #[tokio::test]
    async fn without_a_404_page_a_miss_is_the_bare_404_it_was() {
        let (router, root) = router_over("no-404-page", None);
        let (status, kind, body) = get(&router, "/wp-login.php").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(kind, "text/plain; charset=utf-8");
        assert_eq!(body, b"no such asset");
        std::fs::remove_dir_all(&root).expect("the dist is removable");
    }
}
