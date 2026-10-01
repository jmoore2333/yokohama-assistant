//! **`events`** (spec §2.2, §2.3): `GET /api/games/{id}/events` on this host.
//! It is one of the transport routes a host writes itself — axum's is the
//! `events` handler in `crates/yokohama-web/src/lib.rs`, behind `server` — so it
//! is no row of the layer's route table, and `dispatch` would answer it 404.
//! This module restates that handler, in its order.
//!
//! **`after` and `wait` are parsed and IGNORED** (spec §2.3; M6.5c2 cycle D,
//! R-D5): the held arm is retired on every host, so this route answers at
//! once, as axum's handler now does too, and no host reports `held`. The keys
//! stay for an older precached release: its close probe (spec §2.7 — after a
//! socket that fails to open, or one that closes when the arm did not close it
//! and no `end` frame came before the close, the source's own `once()`, one
//! time) sends `wait=25` for a game, and `EventsQuery`'s `deny_unknown_fields`
//! would otherwise answer it a 400. The current client's probe sends no
//! `wait`. A held request here would keep an object invocation open for
//! nothing.

use http::StatusCode;
use yokohama_web::api::{self, EventsQuery};
use yokohama_web::ApiError;

use crate::{error_body, from_the_layer, host_refusal, identity_of, ReplyData, Scope};

/// `GET /api/games/{id}/events?after=&wait=`, in the order axum's handler takes
/// its extractors and then its own check:
///
/// 1. **the caller** — [`identity_of`]; a refusal is the 401 carrying its
///    sentence, the one `dispatch`'s 401 carries for the same caller (for an
///    absent caller, `identity::resolve`'s absent-caller sentence in the door's
///    mode);
/// 2. **the query** — the layer's own `parse_query` into its own `EventsQuery`,
///    so an unknown key is the layer's 400, in axum's words;
/// 3. **the cap** — `api::check_wait`, the layer's own (M6.5c2), which axum's
///    handler calls too: a `wait` above 25 is its 400, in its sentence. The
///    refusal is the HANDLER's, not the parser's: `parse_query` accepts
///    `wait=26`;
/// 4. **the reading** — `api::events_at`, the one function axum's handler and
///    its stream both answer with: `Ok` serialized at 200, `Err` in the layer's
///    error shape (`error_body`).
///
/// The first three refuse before the `Games` is touched. The fourth answers
/// through `from_the_layer`, the drain `dispatch` answers through, because
/// `events_at` can build a cold handle — and a handle rebuilt with a bot to move
/// pumps that move and persists it (plan re-review m-c). It answers under
/// `Scope::Table(id)`, so that record change clears the table's picker
/// (`picker::watching`; R1's cleanup debt 1).
pub fn events_inner(id: &str, user: Option<&str>, query: &str) -> ReplyData {
    let user = match identity_of(user) {
        Ok(user) => user,
        Err(sentence) => {
            return host_refusal(ApiError { status: StatusCode::UNAUTHORIZED, message: sentence, table_status: None, foreign_bot: None })
        }
    };
    let q = match api::parse_query::<EventsQuery>(query) {
        Ok(q) => q,
        Err(e) => return host_refusal(e),
    };
    if let Err(e) = api::check_wait(q.wait) {
        return host_refusal(e);
    }
    from_the_layer(Scope::Table(id), |games| match api::events_at(games, id, &user) {
        Ok(events) => (200, serde_json::to_string(&events).expect("an Events serializes"), None),
        Err(e) => (e.status.as_u16(), error_body(&e), None),
    })
}
