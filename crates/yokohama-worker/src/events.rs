//! **`events`** (spec §2.2, §2.3): `GET /api/games/{id}/events` on this host.
//! It is one of the transport routes a host writes itself — axum's is the
//! `events` handler in `crates/yokohama-web/src/lib.rs`, behind `server` — so it
//! is no row of the layer's route table, and `dispatch` would answer it 404.
//! This module restates that handler, in its order.
//!
//! **`wait` is validated and never held** (spec §2.3) — safe only because this
//! host reports `signal: "ws"` (`HOST` in `lib.rs`). No client runs the held
//! arm against it: that arm re-asks a value source at once
//! (`web/src/signal.ts`), so an immediate answer to it would spin against the
//! object. What asks with `wait` here is spec §2.7's probe, which Task 8 adds to
//! the client: after a socket that fails to open, or one that closes when the
//! arm did not close it and no `end` frame came before the close, it calls the
//! source's own `once()` one time — `wait=25` for a game — and needs an
//! immediate answer. A held request would keep an object invocation open for
//! nothing.

use http::StatusCode;
use yokohama_web::api::{self, EventsQuery};
use yokohama_web::ApiError;

use crate::{error_body, from_the_layer, host_refusal, identity_of, ReplyData};

/// The cap on `wait`, RESTATED from `MAX_WAIT_SECONDS` in
/// `crates/yokohama-web/src/lib.rs` (the orchestrator's ruling R-03). That
/// constant is private and behind `server`, and the refusal is the axum
/// HANDLER's, not the layer parser's: `parse_query` accepts `wait=26`.
const MAX_WAIT_SECONDS: u64 = 25;

/// `GET /api/games/{id}/events?after=&wait=`, in the order axum's handler takes
/// its extractors and then its own check:
///
/// 1. **the caller** — [`identity_of`]; a refusal is the 401 carrying its
///    sentence, the one `dispatch`'s 401 carries for the same caller (for an
///    absent caller, `identity::resolve`'s `worker`-mode sentence);
/// 2. **the query** — the layer's own `parse_query` into its own `EventsQuery`,
///    so an unknown key is the layer's 400, in axum's words;
/// 3. **the cap** — a `wait` above 25 is the handler's 400, its sentence
///    restated word for word;
/// 4. **the reading** — `api::events_at`, the one function axum's handler and
///    its stream both answer with: `Ok` serialized at 200, `Err` in the layer's
///    error shape (`error_body`).
///
/// The first three refuse before the `Games` is touched. The fourth answers
/// through `from_the_layer`, the drain `dispatch` answers through, because
/// `events_at` can build a cold handle — and a handle rebuilt with a bot to move
/// pumps that move and persists it (plan re-review m-c).
pub fn events_inner(id: &str, user: Option<&str>, query: &str) -> ReplyData {
    let user = match identity_of(user) {
        Ok(user) => user,
        Err(sentence) => {
            return host_refusal(ApiError { status: StatusCode::UNAUTHORIZED, message: sentence, table_status: None })
        }
    };
    let q = match api::parse_query::<EventsQuery>(query) {
        Ok(q) => q,
        Err(e) => return host_refusal(e),
    };
    if let Some(wait) = q.wait {
        if wait > MAX_WAIT_SECONDS {
            return host_refusal(ApiError::bad_request(format!(
                "a request can be held for at most {MAX_WAIT_SECONDS} seconds, not {wait}"
            )));
        }
    }
    from_the_layer(|games| match api::events_at(games, id, &user) {
        Ok(events) => (200, serde_json::to_string(&events).expect("an Events serializes"), None),
        Err(e) => (e.status.as_u16(), error_body(&e), None),
    })
}
