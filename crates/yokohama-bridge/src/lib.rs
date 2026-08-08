//! The Phase-3.5b BGA bridge relay (design doc `docs/plans/2026-08-07-bga-bridge-design.md` §2.2).
//!
//! One job: **append every item the browser extension hands us to a per-table
//! NDJSON file.** That file is the bridge's raw truth — auditable, replayable,
//! and produced before any translation exists, so a table that finishes fast
//! loses nothing (design §4, "capture first").
//!
//! What this crate deliberately is NOT:
//!
//! - It is not a Yokohama anything. It depends on no engine crate, knows no
//!   rules, and never parses a move. Translation is H1/H2's work inside
//!   `yokohama-cli`; a relay that understood the payload could corrupt it.
//! - It is not a web server. It speaks the smallest slice of HTTP/1.1 that a
//!   Chrome extension's `fetch` needs (one GET, one POST, the CORS preflight
//!   for that POST) and answers everything else with a 404.
//! - It is not reachable from the network. `bind::resolve` REFUSES any address
//!   that is not loopback, so "expose it to the LAN" is a compile-and-run-time
//!   error rather than a flag someone can set by accident.
//!
//! **Delivery is at-least-once, and H1/H2 MUST dedupe.** Nothing here is a
//! transaction: the extension re-POSTs from its outbox whenever it is unsure a
//! capture landed, and an MV3 service worker recycled mid-drain will re-send
//! items it had already delivered. The same is true on the page side, where any
//! wrapper defect of the S-1 class shows up as a response captured twice. The
//! capture file is RAW TRUTH, and raw truth includes duplicates — a consumer
//! that assumes one line per event will replay moves twice. Dedupe on the
//! payload's own identity (a notification's id, a snapshot's `captured_at_ms`
//! plus content), never on `received_at_ms`, which differs across a re-send by
//! construction.
//!
//! Fidelity note, because it affects what H2 can trust: the capture line's
//! `payload` is re-serialized through `serde_json`, whose default map is a
//! `BTreeMap` — so **object keys come back out in alphabetical order, not the
//! order BGA sent them**. That is semantically inert for JSON, and the one
//! place raw bytes genuinely matter is handled on the extension side, which
//! ships a notification response body as a verbatim STRING (`body_text`)
//! rather than as a re-parsed object. Enabling `serde_json/preserve_order`
//! would fix the cosmetic half at the cost of unifying that feature onto every
//! other crate in the workspace, including the one that serializes
//! `GameRecord` — not a trade this job is worth.

pub mod bind;
pub mod capture;
pub mod http;
pub mod router;
pub mod server;

/// Default TCP port. Nothing sacred about the number, but it is pinned in FOUR
/// places that have to agree, and the fourth is the one that gets forgotten:
///
/// 1. here;
/// 2. `bridge/extension/background.js` — `RELAY_ORIGIN`, where the POST goes;
/// 3. `bridge/extension/manifest.json` — the `http://127.0.0.1:8787/*` host
///    permission. Miss this one and the extension is silently blocked from
///    reaching a relay that is running perfectly well on the new port;
/// 4. `docs/bridge-setup.md`, which tells the owner all of the above.
pub const DEFAULT_PORT: u16 = 8787;

/// Default capture directory, relative to wherever the relay is launched.
/// Gitignored — captures are the owner's own game data, never committed.
pub const DEFAULT_DIR: &str = "bridge-captures";
