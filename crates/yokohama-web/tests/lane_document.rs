//! **The lane document's API map is the route table** (M6.5c2 cycle E, R-6.5c2.45).
//!
//! `docs/web-lane.md` §13, "The API map", is one table: `METHOD /path` · host
//! · who may call it · what it answers. The owner asked that a route added
//! later fail the build until the map names it, so this binary reads the
//! table and holds it to two lists:
//!
//! - the rows whose host is `layer` are EXACTLY `api::routes(true)` — the
//!   layer's own table with multiplayer on — so a route the layer gains fails
//!   here until the map names it, and a row naming a route the layer dropped
//!   fails too;
//! - the rows whose host is `axum`, `worker` or `browser` are EXACTLY
//!   [`HOST_ROUTES`], the routes a host answers itself before or beside the
//!   layer's dispatch, as the cycle E dossier's item 27 lists them
//!   (`docs/plans/2026-09-29-web-frontend-m6.5c2e-dossier.md`, "The API route
//!   list"). That list is the document's own pin: a host route added to the
//!   map without a line here fails, and so does one the map drops.
//!
//! What it cannot see: a host route added to `crates/yokohama-web/src/lib.rs`
//! (`app`) or to `worker/src/route.ts` and `admin.ts` without a line in the
//! map AND here. `worker/scripts/access-paths.test.mjs` reads those router
//! files for their `/api/<segment>` prefixes, which catches a new prefix but
//! not a new route under an existing one. Nor a host that stops answering
//! `GET /api/art` itself: the map carries it as a `layer` row, since it is in
//! `api::routes`, and both hosts intercept it before dispatch (axum's adapter,
//! the Worker's `route.ts`) — this binary holds the row, not the interception.
//!
//! Per-edit, with the other `tests/` binaries: it reads one committed file and
//! runs in milliseconds.

use std::collections::BTreeSet;
use std::path::PathBuf;

use yokohama_web::api;

/// The routes a host answers itself (the dossier's item 27), one line each:
/// `(host, method, pattern)`. The document's own pin — read the module doc
/// before adding a line.
///
/// - `axum`: the NAS/Docker binary's three transport routes, registered with
///   multiplayer on, and the `/enter` turnstile, registered with multiplayer
///   or an archive on (`lib.rs`, `app`).
/// - `worker`: the Cloudflare shell's own — the turnstile, the table object's
///   two transport routes, the house's lobby socket, and the two operator
///   routes behind `YOKOHAMA_WEB_ADMINS` (`worker/src/route.ts`,
///   `worker/src/admin.ts`, `worker/src/house.ts`).
/// - `browser`: none. The local host (`crates/yokohama-wasm`) answers the
///   `layer` rows through the same `api::dispatch` and adds no path of its
///   own; its seven exports are not paths.
const HOST_ROUTES: &[(&str, &str, &str)] = &[
    ("axum", "GET", "/api/games/{id}/events"),
    ("axum", "GET", "/api/games/{id}/stream"),
    ("axum", "GET", "/api/stream"),
    ("axum", "GET", "/enter"),
    ("worker", "GET", "/enter"),
    ("worker", "GET", "/api/games/{id}/events"),
    ("worker", "GET", "/api/games/{id}/stream"),
    ("worker", "GET", "/api/stream"),
    ("worker", "GET", "/api/admin/tables"),
    ("worker", "POST", "/api/admin/tables/{id}/expire"),
];

/// The four host names a row may carry.
const HOSTS: [&str; 4] = ["layer", "axum", "worker", "browser"];

/// The section's heading, exactly as the document writes it.
const SECTION: &str = "## 13. The API map";

fn document() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/web-lane.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} is the lane document: {e}", path.display()))
}

/// One parsed row: `(host, method, pattern)`.
type Row = (String, String, String);

/// Every row of §13's tables whose first cell is a route: `` `METHOD /path` ``.
///
/// A row whose first cell OPENS with a backtick and does not parse is a
/// failure, never a skip — a typo in the map would otherwise drop a route out
/// of the comparison in silence. The header and the separator open with no
/// backtick and are passed over.
fn map_rows(doc: &str) -> Vec<Row> {
    let start = doc
        .lines()
        .position(|l| l.trim_end() == SECTION)
        .unwrap_or_else(|| panic!("docs/web-lane.md has no line {SECTION:?}"));
    let mut rows = Vec::new();
    for line in doc.lines().skip(start + 1).take_while(|l| !l.starts_with("## ")) {
        let Some(inner) = line.trim().strip_prefix('|') else { continue };
        let cells: Vec<&str> = inner.split('|').map(str::trim).collect();
        let first = cells[0];
        if !first.starts_with('`') {
            continue;
        }
        let route = first
            .strip_prefix('`')
            .and_then(|s| s.strip_suffix('`'))
            .unwrap_or_else(|| panic!("a route cell is one code span: {line:?}"));
        let (method, pattern) = route
            .split_once(' ')
            .unwrap_or_else(|| panic!("a route cell is `METHOD /path`: {line:?}"));
        assert!(
            !method.is_empty() && method.chars().all(|c| c.is_ascii_uppercase()),
            "the method is upper-case letters: {line:?}"
        );
        assert!(pattern.starts_with('/') && !pattern.contains(' '), "the path is one absolute path: {line:?}");
        let host = cells.get(1).map(|h| h.trim_matches('`')).unwrap_or_default();
        assert!(HOSTS.contains(&host), "the host cell is one of {HOSTS:?}, got {host:?}: {line:?}");
        rows.push((host.to_string(), method.to_string(), pattern.to_string()));
    }
    rows
}

#[test]
fn the_maps_layer_rows_are_the_layers_route_table() {
    let rows = map_rows(&document());
    let named: BTreeSet<(String, String)> = rows
        .iter()
        .filter(|(host, ..)| host == "layer")
        .map(|(_, method, pattern)| (method.clone(), pattern.clone()))
        .collect();
    let table: BTreeSet<(String, String)> =
        api::routes(true).into_iter().map(|r| (r.method.to_string(), r.pattern.to_string())).collect();
    assert!(!table.is_empty(), "premise: the layer's table has rows");
    let missing: Vec<_> = table.difference(&named).collect();
    let extra: Vec<_> = named.difference(&table).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "docs/web-lane.md §13 must name every route of api::routes(true) as a `layer` row and no other:\n  \
         in the table, not the map: {missing:?}\n  in the map, not the table: {extra:?}"
    );
}

#[test]
fn the_maps_host_rows_are_the_routes_each_host_answers_itself() {
    let rows = map_rows(&document());
    let named: BTreeSet<Row> = rows.iter().filter(|(host, ..)| host != "layer").cloned().collect();
    let pinned: BTreeSet<Row> =
        HOST_ROUTES.iter().map(|(h, m, p)| (h.to_string(), m.to_string(), p.to_string())).collect();
    let missing: Vec<_> = pinned.difference(&named).collect();
    let extra: Vec<_> = named.difference(&pinned).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "docs/web-lane.md §13's axum, worker and browser rows must be HOST_ROUTES exactly:\n  \
         pinned, not in the map: {missing:?}\n  in the map, not pinned: {extra:?}"
    );
}

#[test]
fn no_route_is_named_twice_for_one_host() {
    let rows = map_rows(&document());
    let mut seen = BTreeSet::new();
    let twice: Vec<_> = rows.iter().filter(|row| !seen.insert((*row).clone())).collect();
    assert!(twice.is_empty(), "a (host, method, path) named twice in §13: {twice:?}");
}

/// The parser's own guard, over text, so the two cells above cannot pass by
/// reading nothing: a malformed route cell is refused, a header is passed over,
/// and the section ends at the next `## `.
#[test]
fn the_parser_reads_route_rows_refuses_a_malformed_one_and_stops_at_the_next_section() {
    let good = "# t\n## 13. The API map\n\n| Route | Host | Who | What |\n|---|---|---|---|\n\
                | `GET /api/me` | `layer` | anyone | the caller |\n\
                | `GET /enter` | axum | anyone | a 302 |\n\
                ## 14. Next\n| `GET /api/nope` | layer | x | y |\n";
    assert_eq!(
        map_rows(good),
        vec![
            ("layer".to_string(), "GET".to_string(), "/api/me".to_string()),
            ("axum".to_string(), "GET".to_string(), "/enter".to_string()),
        ]
    );
    for bad in [
        "## 13. The API map\n| `GET/api/me` | layer | a | b |\n",
        "## 13. The API map\n| `get /api/me` | layer | a | b |\n",
        "## 13. The API map\n| `GET /api/me` | server | a | b |\n",
        "## 13. The API map\n| `GET /api/me | layer | a | b |\n",
    ] {
        let refused = std::panic::catch_unwind(|| map_rows(bad));
        assert!(refused.is_err(), "a malformed row is refused, never skipped: {bad:?}");
    }
}
