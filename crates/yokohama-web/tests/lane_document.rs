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
//! **And §16, "The metered-action ledger", is held to the same two lists**
//! (M6.5c2 cycle F, R-F5; spec §2.5): one row per request the client can make
//! to a server, `METHOD /path` · trigger · purpose · per session / per applied
//! move · before a clearance? · script-able without a browser? · instrument.
//! The cell [`the_ledger_names_every_route`] holds its route column to EVERY
//! route of `api::routes(true)` and of [`HOST_ROUTES`] — one row per method and
//! path, whichever host answers it — and refuses a metered row the code does
//! not know: the ledger cannot invent a route. The release's own files are rows
//! too, NON-metered, their count cell reading exactly `static`; a static row may
//! never name an API path or `/enter`, and the five the spec promises (the
//! three presence files, the 404 page and the build's stamp) must be there.
//!
//! **One parser reads both tables, parameterised by its column set** ([`Table`]):
//! the header row must BE the set, a route row must carry exactly that many
//! cells, and only the route is parsed — so §13's second column is checked as a
//! host by [`map_rows`] and §16's, the trigger, is free text.
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

/// A table this binary reads: the heading of the section it sits under, and
/// its column set, exactly as the header row writes it. The first column is
/// always the route, `` `METHOD /path` ``.
struct Table {
    section: &'static str,
    columns: &'static [&'static str],
}

/// §13, the API map (cycle E, R-6.5c2.45).
const MAP: Table = Table {
    section: "## 13. The API map",
    columns: &["route", "host", "who may call it", "what it answers"],
};

/// §16, the metered-action ledger (cycle F, R-F5; spec §2.5).
const LEDGER: Table = Table {
    section: "## 16. The metered-action ledger",
    columns: &[
        "route",
        "trigger",
        "purpose",
        "per session / per applied move",
        "before a clearance?",
        "script-able without a browser?",
        "instrument",
    ],
};

/// What a ledger row's count cell reads when the row is one of the release's
/// own files: served by the asset layer, never a Worker invocation.
const STATIC: &str = "static";

/// The static rows the spec promises the ledger carries from the start: the
/// three presence files, the 404 page and the build's stamp.
const PROMISED_STATIC: [&str; 5] = ["/robots.txt", "/llms.txt", "/sitemap.xml", "/404.html", "/build-values.json"];

fn document() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/web-lane.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} is the lane document: {e}", path.display()))
}

/// One parsed row of §13: `(host, method, pattern)`.
type Row = (String, String, String);

/// One route row of a [`Table`]: its method, its path, and every cell AFTER
/// the route, in the column set's order — with the line, for a message.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TableRow {
    method: String,
    pattern: String,
    cells: Vec<String>,
    line: String,
}

/// Every route row of `table`'s section: a row whose first cell is a route,
/// `` `METHOD /path` ``.
///
/// A row whose first cell OPENS with a backtick and does not parse is a
/// failure, never a skip — a typo would otherwise drop a route out of the
/// comparison in silence. So is a route row with more or fewer cells than the
/// column set, a header that is not the column set, and a route row before any
/// header. The separator row is passed over, and the section ends at the next
/// `## `.
fn table_rows(doc: &str, table: &Table) -> Vec<TableRow> {
    let start = doc
        .lines()
        .position(|l| l.trim_end() == table.section)
        .unwrap_or_else(|| panic!("docs/web-lane.md has no line {:?}", table.section));
    let mut rows = Vec::new();
    let mut header_seen = false;
    for line in doc.lines().skip(start + 1).take_while(|l| !l.starts_with("## ")) {
        let Some(inner) = line.trim().strip_prefix('|') else { continue };
        let inner = inner.strip_suffix('|').unwrap_or(inner);
        let cells: Vec<&str> = inner.split('|').map(str::trim).collect();
        let first = cells[0];
        if !first.starts_with('`') {
            if cells.iter().all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')) {
                continue;
            }
            assert_eq!(
                cells,
                table.columns,
                "a table under {:?} has the header {:?}: {line:?}",
                table.section,
                table.columns
            );
            header_seen = true;
            continue;
        }
        assert!(header_seen, "a route row comes after its table's header: {line:?}");
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
        assert_eq!(
            cells.len(),
            table.columns.len(),
            "a route row has the column set's {} cells: {line:?}",
            table.columns.len()
        );
        rows.push(TableRow {
            method: method.to_string(),
            pattern: pattern.to_string(),
            cells: cells[1..].iter().map(|c| c.to_string()).collect(),
            line: line.to_string(),
        });
    }
    rows
}

/// §13's rows as `(host, method, pattern)`: [`table_rows`] over [`MAP`], each
/// row's second column held to one of [`HOSTS`].
fn map_rows(doc: &str) -> Vec<Row> {
    table_rows(doc, &MAP)
        .into_iter()
        .map(|row| {
            let host = row.cells[0].trim_matches('`');
            assert!(HOSTS.contains(&host), "the host cell is one of {HOSTS:?}, got {host:?}: {:?}", row.line);
            (host.to_string(), row.method, row.pattern)
        })
        .collect()
}

/// Every route the code answers, by method and path, whichever host answers
/// it: `api::routes(true)` and [`HOST_ROUTES`] — the set §13's map is held to.
fn every_route() -> BTreeSet<(String, String)> {
    api::routes(true)
        .into_iter()
        .map(|r| (r.method.to_string(), r.pattern.to_string()))
        .chain(HOST_ROUTES.iter().map(|(_, m, p)| (m.to_string(), p.to_string())))
        .collect()
}

/// The panic message `f` raised, or `None` when it returned.
fn refusal<T>(f: impl FnOnce() -> T + std::panic::UnwindSafe) -> Option<String> {
    let err = std::panic::catch_unwind(f).err()?;
    Some(
        err.downcast_ref::<String>()
            .cloned()
            .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
    )
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

/// §13 with its heading, header and separator over `rows`, as the document
/// writes them.
fn map_section(rows: &str) -> String {
    format!(
        "# t\n## 13. The API map\n\n| route | host | who may call it | what it answers |\n|---|---|---|---|\n{rows}"
    )
}

/// The parser's own guard, over text, so the two cells above cannot pass by
/// reading nothing: a malformed route cell is refused, a header is passed over,
/// and the section ends at the next `## `. Each malformed case is refused for
/// ITS defect: the message is read, so a case refused for another reason (a
/// header the fixture forgot) cannot pass as this one.
#[test]
fn the_parser_reads_route_rows_refuses_a_malformed_one_and_stops_at_the_next_section() {
    let good = map_section(
        "| `GET /api/me` | `layer` | anyone | the caller |\n\
         | `GET /enter` | axum | anyone | a 302 |\n\
         ## 14. Next\n| `GET /api/nope` | layer | x | y |\n",
    );
    assert_eq!(
        map_rows(&good),
        vec![
            ("layer".to_string(), "GET".to_string(), "/api/me".to_string()),
            ("axum".to_string(), "GET".to_string(), "/enter".to_string()),
        ]
    );
    for (bad, defect) in [
        ("| `GET/api/me` | layer | a | b |\n", "`METHOD /path`"),
        ("| `get /api/me` | layer | a | b |\n", "upper-case"),
        ("| `GET /api/me` | server | a | b |\n", "the host cell"),
        ("| `GET /api/me | layer | a | b |\n", "one code span"),
        ("| `GET /api/me` | layer | a |\n", "the column set's 4 cells"),
        ("| `GET /api/me` | layer | a | b | c |\n", "the column set's 4 cells"),
    ] {
        let text = map_section(bad);
        let said = refusal(|| map_rows(&text));
        assert!(
            said.as_deref().is_some_and(|m| m.contains(defect)),
            "a malformed row is refused for its defect ({defect:?}), never skipped: {bad:?} -> {said:?}"
        );
    }
    let wrong_header = "## 13. The API map\n| Route | Host | Who | What |\n|---|---|---|---|\n| `GET /api/me` | layer | a | b |\n";
    assert!(
        refusal(|| map_rows(wrong_header)).is_some_and(|m| m.contains("has the header")),
        "a header that is not the column set is refused"
    );
    let headless = "## 13. The API map\n| `GET /api/me` | layer | a | b |\n";
    assert!(
        refusal(|| map_rows(headless)).is_some_and(|m| m.contains("after its table's header")),
        "a route row with no header above it is refused"
    );
}

/// **The ledger's columns parse with the same parser** (cycle F): its second
/// column is the trigger, free text, where §13's is a host — the reason the
/// parser takes a column set.
#[test]
fn the_parser_reads_the_ledgers_seven_columns_with_a_free_text_trigger() {
    let header = "| route | trigger | purpose | per session / per applied move | before a clearance? | \
                  script-able without a browser? | instrument |\n|---|---|---|---|---|---|---|\n";
    let text = format!(
        "## 16. The metered-action ledger\n\n{header}\
         | `GET /api/health` | the lobby, on entering it | the facts | E: 3 | yes | yes | the tally |\n\
         | `GET /robots.txt` | a crawler | what may be crawled | static | yes | yes | the probe |\n"
    );
    let rows = table_rows(&text, &LEDGER);
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].method.as_str(), rows[0].pattern.as_str()), ("GET", "/api/health"));
    assert_eq!(rows[0].cells[0], "the lobby, on entering it", "the trigger is free text");
    assert_eq!(rows[1].cells[2], STATIC);
    let short = format!("## 16. The metered-action ledger\n{header}| `GET /api/me` | a | b | c | d | e |\n");
    assert!(
        refusal(|| table_rows(&short, &LEDGER)).is_some_and(|m| m.contains("the column set's 7 cells")),
        "a ledger row with six cells is refused"
    );
    let as_map = format!("## 13. The API map\n{header}| `GET /api/me` | a | b | c | d | e | f |\n");
    assert!(
        refusal(|| map_rows(&as_map)).is_some_and(|m| m.contains("has the header")),
        "the map's parser refuses the ledger's header: each table is read by its own column set"
    );
}

/// **Every route is a row of the ledger, and the ledger names no route the
/// code lacks** (cycle F, R-F5; spec §2.5's "a cell holds every route of the
/// router to a row"). The comparison is by method and path: a route two hosts
/// answer is ONE row, whose trigger and purpose say how each answers it.
#[test]
fn the_ledger_names_every_route() {
    let rows = table_rows(&document(), &LEDGER);
    let count = LEDGER.columns.iter().position(|c| *c == "per session / per applied move").unwrap() - 1;
    let (statics, metered): (Vec<&TableRow>, Vec<&TableRow>) = rows.iter().partition(|r| r.cells[count] == STATIC);

    let code = every_route();
    assert!(code.len() > HOST_ROUTES.len(), "premise: the layer's table has rows");
    let named: BTreeSet<(String, String)> = metered.iter().map(|r| (r.method.clone(), r.pattern.clone())).collect();
    let missing: Vec<_> = code.difference(&named).collect();
    let extra: Vec<_> = named.difference(&code).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "docs/web-lane.md §16 must name every route of api::routes(true) and HOST_ROUTES as a metered row, and no \
         other — a release file's row reads `static` in its count cell:\n  in the code, not the ledger: \
         {missing:?}\n  in the ledger, not the code: {extra:?}"
    );

    for row in &statics {
        let p = row.pattern.as_str();
        assert!(
            p != "/enter" && p != "/api" && !p.starts_with("/api/") && !code.contains(&(row.method.clone(), p.to_string())),
            "a static row is a release file, never a route the Worker answers: {:?}",
            row.line
        );
    }
    let static_paths: BTreeSet<&str> = statics.iter().map(|r| r.pattern.as_str()).collect();
    let unpromised: Vec<_> = PROMISED_STATIC.iter().filter(|p| !static_paths.contains(*p)).collect();
    assert!(unpromised.is_empty(), "the ledger's static rows lack the spec's {unpromised:?}");

    let mut seen = BTreeSet::new();
    let twice: Vec<_> =
        rows.iter().filter(|r| !seen.insert((r.method.clone(), r.pattern.clone()))).map(|r| &r.line).collect();
    assert!(twice.is_empty(), "a (method, path) named twice in §16: {twice:?}");
    let blank: Vec<_> = rows.iter().filter(|r| r.cells.iter().any(|c| c.is_empty())).map(|r| &r.line).collect();
    assert!(blank.is_empty(), "every cell of a ledger row is filled — a blank is a row nobody read: {blank:?}");
}
